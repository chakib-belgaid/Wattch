use bytes::{BufMut, BytesMut};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::errors::{Result, WattchError};

pub const MAX_FRAME_SIZE: usize = 1024 * 1024;

pub fn encode_frame<M>(message: &M) -> Result<BytesMut>
where
    M: Message,
{
    let payload_len = message.encoded_len();
    if payload_len > MAX_FRAME_SIZE {
        return Err(WattchError::FrameTooLarge {
            size: payload_len,
            max: MAX_FRAME_SIZE,
        });
    }

    let mut frame = BytesMut::with_capacity(4 + payload_len);
    frame.put_u32_le(payload_len as u32);
    message.encode(&mut frame)?;
    Ok(frame)
}

pub fn decode_frame<M>(frame: &[u8]) -> Result<M>
where
    M: Message + Default,
{
    if frame.len() < 4 {
        return Err(WattchError::TruncatedPayload {
            expected: 4,
            actual: frame.len(),
        });
    }

    let payload_len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if payload_len > MAX_FRAME_SIZE {
        return Err(WattchError::FrameTooLarge {
            size: payload_len,
            max: MAX_FRAME_SIZE,
        });
    }

    let actual_len = frame.len() - 4;
    if actual_len < payload_len {
        return Err(WattchError::TruncatedPayload {
            expected: payload_len,
            actual: actual_len,
        });
    }
    if actual_len > payload_len {
        return Err(WattchError::BadRequest(
            "frame contains trailing bytes after protobuf payload".to_string(),
        ));
    }

    Ok(M::decode(&frame[4..])?)
}

pub async fn read_frame_async<R, M>(reader: &mut R) -> Result<M>
where
    R: AsyncRead + Unpin,
    M: Message + Default,
{
    let mut header = [0_u8; 4];
    reader.read_exact(&mut header).await?;

    let payload_len = u32::from_le_bytes(header) as usize;
    if payload_len > MAX_FRAME_SIZE {
        return Err(WattchError::FrameTooLarge {
            size: payload_len,
            max: MAX_FRAME_SIZE,
        });
    }

    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(M::decode(payload.as_slice())?)
}

pub async fn write_frame_async<W, M>(writer: &mut W, message: &M) -> Result<()>
where
    W: AsyncWrite + Unpin,
    M: Message,
{
    let frame = encode_frame(message)?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::WattchError;
    use wattch_proto::wattch::v1::{
        request, response, HelloRequest, HelloResponse, Request, Response,
    };

    #[test]
    fn frame_roundtrip_request() {
        let request = Request {
            request_id: 42,
            kind: Some(request::Kind::Hello(HelloRequest {
                protocol_version: 1,
                client_name: "unit-test".to_string(),
            })),
        };

        let frame = encode_frame(&request).expect("encode request");
        let decoded: Request = decode_frame(&frame).expect("decode request");

        assert_eq!(decoded.request_id, 42);
        assert!(matches!(decoded.kind, Some(request::Kind::Hello(_))));
    }

    #[test]
    fn frame_roundtrip_response() {
        let response = Response {
            request_id: 7,
            kind: Some(response::Kind::Hello(HelloResponse {
                protocol_version: 1,
                daemon_version: "0.1.0".to_string(),
            })),
        };

        let frame = encode_frame(&response).expect("encode response");
        let decoded: Response = decode_frame(&frame).expect("decode response");

        assert_eq!(decoded.request_id, 7);
        assert!(matches!(decoded.kind, Some(response::Kind::Hello(_))));
    }

    #[test]
    fn frame_rejects_too_large_payload() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&((MAX_FRAME_SIZE as u32) + 1).to_le_bytes());

        let error = decode_frame::<Request>(&frame).expect_err("oversized frame should fail");
        assert!(matches!(error, WattchError::FrameTooLarge { .. }));
    }

    #[test]
    fn frame_rejects_truncated_payload() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&8_u32.to_le_bytes());
        frame.extend_from_slice(&[1, 2, 3]);

        let error = decode_frame::<Request>(&frame).expect_err("truncated frame should fail");
        assert!(matches!(error, WattchError::TruncatedPayload { .. }));
    }
}

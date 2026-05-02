use std::path::PathBuf;

fn main() {
    let proto_file = PathBuf::from("../../proto/wattch.proto");
    let proto_dir = PathBuf::from("../../proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    prost_build::Config::new()
        .compile_protos(&[proto_file], &[proto_dir])
        .expect("failed to compile wattch protobuf definitions");
}

pub mod wattch {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/wattch.v1.rs"));
    }
}

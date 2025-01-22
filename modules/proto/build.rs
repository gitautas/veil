// build.rs
use grpc_build::Builder;

fn main() {
    Builder::new()
        .build_client(true)
        .build_server(true)
        .force(true)
        .out_dir("out_rs")
        .build(".")
        .unwrap();
}
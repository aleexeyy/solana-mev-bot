use tonic_build;
use tonic_prost_build;

fn main() {
    const PROTOC_ENVAR: &str = "PROTOC";
    if std::env::var(PROTOC_ENVAR).is_err() {
        #[cfg(not(windows))]
        unsafe {
            std::env::set_var(PROTOC_ENVAR, protobuf_src::protoc());
        }
    }

    tonic_prost_build::configure()
        .compile_protos(
            &[
                "protos/auth.proto",
                "protos/shared.proto",
                "protos/shredstream.proto",
            ],
            &["protos"],
        )
        .unwrap();
}
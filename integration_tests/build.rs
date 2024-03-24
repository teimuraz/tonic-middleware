use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .build_server(true)
        .include_file("util.rs")
        .out_dir(out_dir.clone())
        .file_descriptor_set_path(out_dir.clone().join("test_services.bin"))
        .compile(&["proto/test_services.proto"], &["proto"])?;
    Ok(())
}

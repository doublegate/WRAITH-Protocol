fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true) // This is the client
        .compile_protos(&["../../proto/redops.proto"], &["../../proto"])?;
    tauri_build::build();
    Ok(())
}

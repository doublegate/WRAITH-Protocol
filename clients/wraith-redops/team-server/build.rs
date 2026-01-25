fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // This is the server
        .compile(
            &["../proto/redops.proto"],
            &["../proto"],
        )?;
    Ok(())
}

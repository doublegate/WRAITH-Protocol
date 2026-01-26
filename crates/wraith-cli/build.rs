fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["../../clients/wraith-redops/proto/redops.proto"],
            &["../../clients/wraith-redops/proto"],
        )?;
    Ok(())
}

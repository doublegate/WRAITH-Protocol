use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

const CONFIG_MAGIC: &[u8] = b"WRAITH_CONFIG_BLOCK";

pub struct Builder;

impl Builder {
    pub fn patch_implant(
        template_path: &Path,
        output_path: &Path,
        server_addr: &str,
        sleep_interval: u64,
    ) -> anyhow::Result<()> {
        let mut data = Vec::new();
        File::open(template_path)?.read_to_end(&mut data)?;

        // Find magic
        let pos = data
            .windows(CONFIG_MAGIC.len())
            .position(|window| window == CONFIG_MAGIC)
            .ok_or_else(|| anyhow::anyhow!("Magic signature not found in template"))?;

        // Patch server_addr (at pos + 19)
        let addr_start = pos + 19;
        let addr_bytes = server_addr.as_bytes();
        let addr_len = addr_bytes.len().min(64);

        // Clear old addr
        for i in 0..64 {
            data[addr_start + i] = 0;
        }
        // Write new addr
        data[addr_start..addr_start + addr_len].copy_from_slice(&addr_bytes[..addr_len]);

        // Patch sleep_interval (at pos + 19 + 64)
        let sleep_start = addr_start + 64;
        let sleep_bytes = sleep_interval.to_le_bytes();
        data[sleep_start..sleep_start + 8].copy_from_slice(&sleep_bytes);

        File::create(output_path)?.write_all(&data)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn compile_implant(
        source_dir: &Path,
        output_path: &Path,
        _server_addr: &str,
    ) -> anyhow::Result<()> {
        // Compile from source using cargo
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(source_dir.join("Cargo.toml"))
            // .env("WRAITH_SERVER", server_addr) // Pass config via env if build script handles it
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Compilation failed"));
        }

        // Copy artifact
        let artifact = source_dir.join("target/release/spectre_implant"); // Binary name might differ
        if artifact.exists() {
            std::fs::copy(artifact, output_path)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Artifact not found after build"))
        }
    }
}

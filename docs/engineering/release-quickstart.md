# WRAITH Protocol - Release Quick Start

Quick reference for creating releases. See [docs/operations/release-guide.md](../operations/release-guide.md) for detailed documentation.

## Quick Release (Tag-Based)

```bash
# 1. Update CHANGELOG.md
nano CHANGELOG.md  # Add section for your version

# 2. Update version in Cargo.toml
# Edit: version = "0.1.0"
nano Cargo.toml

# 3. Commit changes
git add CHANGELOG.md Cargo.toml
git commit -m "chore: prepare release v0.1.0"
git push origin main

# 4. Create and push tag
git tag -a v0.1.0 -m "WRAITH Protocol v0.1.0"
git push origin v0.1.0

# 5. Monitor release workflow at:
# https://github.com/doublegate/WRAITH-Protocol/actions
```

## Manual Release (GitHub UI)

1. Go to **Actions** → **Release** workflow
2. Click **"Run workflow"**
3. Enter version (e.g., `v0.1.0`)
4. Check options (pre-release, draft)
5. Click **"Run workflow"**

## Version Format

- Stable: `v1.0.0`, `v1.2.3`
- Pre-release: `v0.1.0-alpha.1`, `v1.0.0-beta.2`, `v2.0.0-rc.1`

## Release Checklist

- [ ] Tests pass: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] Formatted: `cargo fmt --all -- --check`
- [ ] CHANGELOG.md updated
- [ ] Version in Cargo.toml updated
- [ ] Changes committed to main
- [ ] CI passing on main

## Workflow Stages

```
┌─────────────┐
│  Validate   │  (30s) - Check version format, CHANGELOG
└──────┬──────┘
       │
┌──────▼──────┐
│    Test     │  (2-5m) - Run tests, clippy, fmt
└──────┬──────┘
       │
┌──────▼──────┐
│   Build     │  (10-15m) - Build 6 platform binaries in parallel
└──────┬──────┘
       │
┌──────▼──────┐
│   Release   │  (30s) - Create GitHub Release with artifacts
└─────────────┘

Total: ~15-20 minutes
```

## Release Artifacts

Each release includes:
- `wraith-x86_64-linux-gnu.tar.gz` - Linux x86_64 (glibc)
- `wraith-x86_64-linux-musl.tar.gz` - Linux x86_64 (static)
- `wraith-aarch64-linux-gnu.tar.gz` - Linux ARM64
- `wraith-x86_64-macos.tar.gz` - macOS Intel
- `wraith-aarch64-macos.tar.gz` - macOS Apple Silicon
- `wraith-x86_64-windows.zip` - Windows x64
- `SHA256SUMS.txt` - Combined checksums

## Verify Downloads

```bash
# Linux/macOS
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.1.0/SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt --ignore-missing

# Windows (PowerShell)
Get-FileHash wraith-x86_64-windows.zip -Algorithm SHA256
```

## Common Issues

**Invalid version format:**
```
❌ Use: v0.1.0  (vX.Y.Z)
✓  Not: 0.1.0, v0.1, v1
```

**Missing CHANGELOG entry:**
```markdown
## [0.1.0] - 2025-11-29

### Added
- Your changes here
```

**Test failures:**
```bash
# Fix locally, then delete and recreate tag
git tag -d v0.1.0
git push origin :refs/tags/v0.1.0
# Fix tests, commit, then create new tag
git tag -a v0.1.0 -m "WRAITH Protocol v0.1.0"
git push origin v0.1.0
```

## Support

- **Detailed Guide:** [docs/operations/release-guide.md](../operations/release-guide.md)
- **Workflow File:** [.github/workflows/release.yml](../../.github/workflows/release.yml)
- **Actions Logs:** https://github.com/doublegate/WRAITH-Protocol/actions

---

**Quick Tip:** Use `v0.x.x-beta.N` for pre-releases to test the workflow before stable releases.

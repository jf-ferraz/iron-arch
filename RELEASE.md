# Iron v0.1.0 Release Checklist

## Pre-Release Verification

All checks completed:

- [x] All 165 tests passing (`cargo test --workspace`)
- [x] No clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
- [x] Code formatted (`cargo fmt --all`)
- [x] Security review passed (no injection vulnerabilities)
- [x] Performance validated (< 1ms startup, 2.9MB binary)
- [x] Documentation complete (README, USER-GUIDE, CONTRIBUTING, EXAMPLES)
- [x] Example configurations complete

## Version Consistency

All references at v0.1.0:
- [x] `Cargo.toml` workspace version
- [x] `pkg/PKGBUILD` pkgver
- [x] `pkg/.SRCINFO` pkgver
- [x] CLI `--version` output

## Release Steps

### 1. Create Git Tag

```bash
# Ensure all changes are committed
git status

# Create annotated tag
git tag -a v0.1.0 -m "Iron v0.1.0 - Initial Release

Phase 1-7 Complete:
- Foundation & Domain Models
- Infrastructure Crates (fs, pacman, git, systemd)
- Core Services (state, bundle, profile, module, update, sync, secrets, recovery)
- CLI Implementation (11 command groups)
- TUI Dashboard with Setup Wizard
- Integration & E2E Tests (165 tests)
- Documentation & Packaging

Features:
- Declarative Arch Linux configuration management
- Host → Bundle → Profile → Module hierarchy
- Safe updates with risk assessment
- Git-crypt secrets management
- System snapshot integration
"

# Push tag to trigger release workflow
git push origin v0.1.0
```

### 2. Automated Release (GitHub Actions)

The `release.yml` workflow will automatically:
1. Build binaries for x86_64-linux-gnu, x86_64-linux-musl, aarch64-linux-gnu
2. Create archives with checksums
3. Extract release notes from CHANGELOG.md
4. Create GitHub release with assets

### 3. AUR Submission (Manual)

```bash
# Clone AUR package
git clone ssh://aur@aur.archlinux.org/iron.git aur-iron
cd aur-iron

# Copy PKGBUILD and .SRCINFO
cp ../iron/pkg/PKGBUILD .
cp ../iron/pkg/.SRCINFO .

# Update checksums
updpkgsums

# Regenerate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Initial release v0.1.0"
git push
```

### 4. Post-Release

- [ ] Verify GitHub release page
- [ ] Verify AUR package installation
- [ ] Announce release (if applicable)

## Installation Verification

After release, verify installation works:

```bash
# From AUR
paru -S iron
# or
yay -S iron

# Or from release binary
curl -fsSL https://github.com/laraj/iron/releases/download/v0.1.0/iron-x86_64-unknown-linux-gnu.tar.gz | tar xz
./iron --version

# Or from source
git clone https://github.com/laraj/iron
cd iron
./scripts/install.sh
iron --version
```

## Troubleshooting

### Build Failures
- Ensure Rust 1.75+ is installed
- Run `cargo update` to refresh dependencies

### Missing Dependencies
- Install: `sudo pacman -S git pacman`
- For development: `sudo pacman -S cargo git`

### AUR Issues
- Verify checksums match release tarball
- Ensure .SRCINFO is regenerated after PKGBUILD changes

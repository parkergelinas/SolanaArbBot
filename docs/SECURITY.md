# Security

Responsible disclosure and immediate security practices for this repository.

Reporting issues
- Open a private issue or email the maintainers if you find a security vulnerability.

Secrets and key material
- Never commit private keys or keypair JSON files. Use environment variables or
  external secret stores for production secrets.
- The workspace contains `crates/wallet` which can load keypairs from files or
  environment variables. Ensure key files are excluded from git and stored
  with strict filesystem permissions.

Automated checks
- This repo runs `cargo-audit` and `cargo-deny` via GitHub Actions at
  `.github/workflows/security-checks.yml`.

Best practices
- Enforce `dry_run` defaults in development and require explicit opt-in for
  live trading features.
- Add `cargo-audit` to CI and fix any vulnerabilities reported before enabling
  live trading.
- Use Git LFS for large binaries and never add keypair files to the repository.

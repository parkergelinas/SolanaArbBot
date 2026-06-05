# Integration Tests

Cross-crate integration tests that exercise the full pipeline or multi-crate boundaries.

## Structure

- `pipeline/` – end-to-end pipeline tests (stream → risk)
- `stress/` – sustained-throughput tests

## Running

```bash
cargo test --test '*'
```

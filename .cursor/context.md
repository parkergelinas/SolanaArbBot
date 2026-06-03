System:
Solana Market Analysis + Arbitrage Engine (Rust)

Source of truth:
docs/system_contract.md

Pipeline:
stream → decoder → pricing → graph → routing → execution → risk → engine

Cursor must operate strictly within this pipeline.
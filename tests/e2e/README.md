E2E Simulation Harness
======================

Purpose
-------
This directory contains a small, configurable harness to run end-to-end simulations locally against a `solana-test-validator` and the repo services. It is intended as a reproducible driver that replays recorded market/backtest events and validates high-level signals.

How it works
------------
- Starts `solana-test-validator` (if available) to provide a local chain.
- Optionally starts services (`stream-api`, `control-api`) unless `SKIP_START=1`.
- Replays events found in `data/backtest_results.json` by POSTing them to an ingestion endpoint specified in `INGEST_ENDPOINT` env var.
- Collects health checks and saves logs/artifacts to `tests/e2e/artifacts/`.

Quick start
-----------
Install Python 3.10+ and `requests`:

```bash
python -m pip install -r tests/e2e/requirements.txt
```

Run the harness:

```bash
python tests/e2e/run_e2e.py
```

Configuration
-------------
- `INGEST_ENDPOINT` — HTTP endpoint to receive replayed events (default: unset; script will print events).
- `SKIP_START=1` — don't start services from the harness; assume they're running locally.

Notes
-----
This is a lightweight harness to help reproduce flows; integration into CI requires stable, containerized services or a staging environment.

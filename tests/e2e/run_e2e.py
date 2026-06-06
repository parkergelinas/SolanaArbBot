#!/usr/bin/env python3
"""Simple E2E replay driver.

Usage:
  INGEST_ENDPOINT=http://localhost:8080/ingest SKIP_START=1 python run_e2e.py

This script is intentionally conservative: it will not assume specific ingestion APIs
— set `INGEST_ENDPOINT` to point at a local ingestion endpoint for the stream or control API.
"""
import os
import subprocess
import sys
import time
import json
from pathlib import Path

import requests

ROOT = Path(__file__).resolve().parents[3]
DATA_FILE = ROOT / 'data' / 'backtest_results.json'
ARTIFACTS = ROOT / 'tests' / 'e2e' / 'artifacts'
ARTIFACTS.mkdir(parents=True, exist_ok=True)


def start_validator():
    if shutil := shutil_available():
        pass


def shutil_available():
    import shutil
    return shutil.which('solana-test-validator')


def start_service(command, cwd=None, env=None):
    return subprocess.Popen(command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)


def wait_for_http(url, timeout=30):
    start = time.time()
    while time.time() - start < timeout:
        try:
            r = requests.get(url, timeout=2)
            if r.status_code < 500:
                return True
        except Exception:
            pass
        time.sleep(1)
    return False


def replay_events(ingest_url):
    if not DATA_FILE.exists():
        print("No data/backtest_results.json found — cannot replay events.")
        return

    with DATA_FILE.open('r', encoding='utf-8') as f:
        data = json.load(f)

    # If the data is an array of events, iterate; otherwise try to post entire payload
    if isinstance(data, list):
        for i, ev in enumerate(data):
            print(f"Posting event {i+1}/{len(data)}")
            if ingest_url:
                try:
                    r = requests.post(ingest_url, json=ev, timeout=5)
                    print('->', r.status_code)
                except Exception as e:
                    print('POST failed:', e)
            else:
                print(json.dumps(ev)[:200])
            time.sleep(0.01)
    else:
        if ingest_url:
            requests.post(ingest_url, json=data)
        else:
            print(json.dumps(data)[:1000])


def main():
    ingest = os.environ.get('INGEST_ENDPOINT')
    skip = os.environ.get('SKIP_START') == '1'

    procs = []

    try:
        if not skip:
            # Optionally start solana-test-validator
            if shutil_available():
                print('Starting solana-test-validator...')
                sv = subprocess.Popen(['solana-test-validator', '--reset'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                procs.append(('solana-test-validator', sv))
                time.sleep(2)

            # Start stream-api and control-api
            print('Starting stream-api...')
            p_stream = start_service(['cargo', 'run', '-p', 'stream-api'], cwd=ROOT)
            procs.append(('stream-api', p_stream))

            print('Starting control-api...')
            p_control = start_service(['cargo', 'run', '-p', 'control-api'], cwd=ROOT)
            procs.append(('control-api', p_control))

            # Wait for services
            print('Waiting for stream-api health...')
            wait_for_http('http://localhost:8080/health', timeout=60)
            print('Waiting for control-api health...')
            wait_for_http('http://localhost:3001/api/health', timeout=60)

        # Replay events
        print('Replaying events...')
        replay_events(ingest)

        print('E2E replay complete — collect artifacts in', ARTIFACTS)

    finally:
        print('Shutting down processes...')
        for name, p in procs:
            try:
                p.terminate()
            except Exception:
                pass


if __name__ == '__main__':
    main()

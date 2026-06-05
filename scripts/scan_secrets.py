#!/usr/bin/env python3
"""Simple repository secret scanner.

Searches for PEM private key headers and Solana CLI keypair JSON arrays.
"""
import re
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]

PEM_RE = re.compile(r"-----BEGIN (?:RSA |)PRIVATE KEY-----")
SOLANA_KEYPAIR_RE = re.compile(r"\[\s*(?:\d+\s*,\s*){63}\d+\s*\]")

def scan_file(path: Path):
    try:
        text = path.read_text(encoding='utf-8', errors='ignore')
    except Exception:
        return []
    hits = []
    if PEM_RE.search(text):
        hits.append('PEM_PRIVATE_KEY')
    if SOLANA_KEYPAIR_RE.search(text):
        hits.append('SOLANA_KEYPAIR_JSON')
    return hits

def main():
    results = []
    for p in ROOT.rglob('*'):
        if p.is_file():
            if any(part.startswith('.git') or part == 'target' for part in p.parts):
                continue
            hits = scan_file(p)
            if hits:
                results.append((str(p.relative_to(ROOT)), hits))

    if not results:
        print('No obvious secrets found')
        return 0

    print('Potential secrets found:')
    for path, hits in results:
        print(f"{path}: {', '.join(hits)}")
    return 1

if __name__ == '__main__':
    sys.exit(main())

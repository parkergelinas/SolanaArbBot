import os, re, tomllib
from pathlib import Path
registry = Path(os.environ.get("USERPROFILE","")) / ".cargo/registry/src"
# find index dir
idx = list(registry.glob("index.crates.io-*"))
if not idx:
    print("no registry")
    raise SystemExit(1)
root = idx[0]
# crates from lock we care about
names = set()
text=open(r"C:\Users\parke\Downloads\Civil Project\fieldlog-lite\SolanaArbBot\Cargo.lock",encoding="utf-8").read().split("\n[[package]]\n")[1:]
vers={}
for b in text:
    n=re.search(r'^name = "([^"]+)"', b, re.M)
    v=re.search(r'^version = "([^"]+)"', b, re.M)
    if n and v:
        vers[n.group(1)] = v.group(1)
        names.add(n.group(1))
allow={"MIT","Apache-2.0","ISC","BSD-2-Clause","BSD-3-Clause"}
bad=[]
missing=[]
for pkg, ver in sorted(vers.items()):
    if pkg in ("accounts","alpha-engine","arb-engine"): # workspace without source line
        continue
    cargo = root / f"{pkg}-{ver}" / "Cargo.toml"
    if not cargo.exists():
        # try alternate folder naming
        alt = list(root.glob(f"{pkg}-{ver}*"))
        if not alt:
            missing.append(pkg)
            continue
        cargo = alt[0] / "Cargo.toml"
    try:
        data = tomllib.loads(cargo.read_text(encoding="utf-8"))
    except Exception:
        missing.append(pkg)
        continue
    lic = data.get("package",{}).get("license") or data.get("license")
    if not lic:
        licexpr = data.get("package",{}).get("license-file") or data.get("license-file")
        bad.append((pkg, ver, f"LICENSE-FILE:{licexpr}"))
        continue
  # dual licenses
    lic_s = str(lic)
    tokens = re.split(r"\s+OR\s+|\s+AND\s+|/", lic_s)
    tokens = [t.strip() for t in tokens if t.strip()]
    ok = lic_s in allow
    if not ok:
        for t in tokens:
            if t in allow:
                ok = True
            if t.startswith("Apache-2.0") or t.startswith("MIT"):
                ok = True
        if "MIT" in lic_s and "Apache" in lic_s:
            ok = True
    if not ok:
        bad.append((pkg, ver, lic_s))
print("checked", len(vers), "missing manifests", len(missing))
print("likely deny license violations", len(bad))
for row in bad[:100]:
    print(f"{row[0]} {row[1]}: {row[2]}")

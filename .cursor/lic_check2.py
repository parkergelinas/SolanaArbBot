import re, tomllib
from pathlib import Path
lock=Path("Cargo.lock").read_text(encoding="utf-8")
blocks=lock.split("\n[[package]]\n")[1:]
vers=[]
for b in blocks:
    n=re.search(r'^name = "([^"]+)"', b, re.M)
    v=re.search(r'^version = "([^"]+)"', b, re.M)
    s=re.search(r'^source = ', b, re.M)
    if n and v and s:
        vers.append((n.group(1), v.group(1)))
root = list(Path.home().joinpath('.cargo/registry/src').glob('index.crates.io-*'))[0]
allow={"MIT","Apache-2.0","ISC","BSD-2-Clause","BSD-3-Clause"}
bad=[]
for name, ver in vers:
    cargo = root / f"{name}-{ver}" / "Cargo.toml"
    if not cargo.exists():
        continue
    data=tomllib.loads(cargo.read_text(encoding='utf-8'))
    pkg=data.get('package', data)
    lic=pkg.get('license')
    if not lic:
        bad.append((name, ver, 'NO_LICENSE_FIELD'))
        continue
    ok = lic in allow or ('MIT' in lic and 'Apache' in lic)
    if not ok:
        for part in re.split(r'\s+OR\s+|\s+AND\s+|/', lic):
            part=part.strip()
            if part in allow or part.startswith('Apache-2.0') or part.startswith('MIT'):
                ok=True
    if not ok:
        bad.append((name, ver, lic))
print('cached crates checked', sum(1 for n,v in vers if (root/f"{n}-{v}/Cargo.toml").exists()), '/', len(vers))
print('violations', len(bad))
for row in sorted(bad):
    print(f"{row[0]} {row[1]}: {row[2]}")

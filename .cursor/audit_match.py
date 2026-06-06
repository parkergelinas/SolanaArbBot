import re, os
from pathlib import Path
adb = Path.home()/'.cargo/advisory-db/crates'
lock_path = Path(r"C:\Users\parke\Downloads\Civil Project\fieldlog-lite\SolanaArbBot\Cargo.lock")
text = lock_path.read_text(encoding='utf-8')
vers = {}
for b in text.split('\n[[package]]\n')[1:]:
    n = re.search(r'^name = "([^"]+)"', b, re.M)
    v = re.search(r'^version = "([^"]+)"', b, re.M)
    s = re.search(r'^source = ', b, re.M)
    if n and v and s:
        vers[n.group(1)] = v.group(1)
# parse advisory frontmatter simply
hits=[]
for crate, ver in sorted(vers.items()):
    cdir = adb/crate
    if not cdir.is_dir():
        continue
    for md in cdir.glob('RUSTSEC-*.md'):
        body = md.read_text(encoding='utf-8', errors='replace')
        if not body.startswith('---'):
            continue
        end = body.find('---', 3)
        fm = body[3:end]
        fields={}
        for line in fm.splitlines():
            if '=' in line:
                k,vv=line.split('=',1)
                fields[k.strip()] = vv.strip().strip('"')
        if fields.get('withdrawn'):
            continue
        av = fields.get('affected-versions','')
        pv = fields.get('patched-versions','')
        # very rough: if exact ver in affected or if no patched covers ver
        # cargo-audit uses semver ranges; approximate check
        if ver in av:
            hits.append((crate, ver, md.name, fields.get('title',''), av, pv))
        else:
            # check <= patched for simple cases like <1.2.3
            m = re.search(r'[<>= ]*([0-9][0-9A-Za-z\.\-+]*)\s*$', av)
            if m and ver == m.group(1):
                hits.append((crate, ver, md.name, fields.get('title',''), av, pv))
print('potential hits', len(hits))
for h in hits[:40]:
    print('\n'.join(h[:4]))

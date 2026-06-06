import re
from collections import defaultdict
text=open("Cargo.lock",encoding="utf-8").read()
blocks=text.split("\n[[package]]\n")[1:]
by_name=defaultdict(list)
for b in blocks:
    m_name=re.search(r'^name = "([^"]+)"', b, re.M)
    m_ver=re.search(r'^version = "([^"]+)"', b, re.M)
    if m_name and m_ver:
        by_name[m_name.group(1)].append(m_ver.group(1))
dups={k:v for k,v in by_name.items() if len(v)>1}
print("crates with multiple versions in lock:", len(dups))
for k in sorted(dups, key=lambda x: (-len(dups[x]), x))[:50]:
    print(k, dups[k])

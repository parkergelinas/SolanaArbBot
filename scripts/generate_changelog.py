#!/usr/bin/env python3
"""Generate a changelog summary from git commit history.

Usage:
  python scripts/generate_changelog.py [--since-tag TAG] [--output docs/CHANGELOG.md]

By default the script uses the latest git tag as the starting point. If no tags
exist it includes the entire history.
"""
import argparse
import subprocess
import sys
from datetime import datetime
import re


def run(cmd):
    return subprocess.check_output(cmd, shell=True, text=True).strip()


def get_latest_tag():
    try:
        return run("git describe --tags --abbrev=0")
    except subprocess.CalledProcessError:
        return None


def commits_since(ref=None):
    if ref:
        rev_range = f"{ref}..HEAD"
    else:
        rev_range = "HEAD"
    fmt = "%H%x1f%an%x1f%ad%x1f%s%x1e"
    out = run(f"git log --pretty=format:'{fmt}' --date=short {rev_range}")
    if not out:
        return []
    entries = out.strip().split('\x1e')
    commits = []
    for e in entries:
        if not e.strip():
            continue
        parts = e.strip().split('\x1f')
        if len(parts) >= 4:
            commits.append({
                'hash': parts[0],
                'author': parts[1],
                'date': parts[2],
                'subject': parts[3],
            })
    return commits


CONVENTIONAL_RE = re.compile(r'^(?P<type>[^(:!]+)(?:\((?P<scope>[^)]+)\))?(!)?:?\s*(?P<desc>.+)')


def classify_commits(commits):
    groups = {}
    for c in commits:
        m = CONVENTIONAL_RE.match(c['subject'])
        if m:
            t = m.group('type').lower()
            desc = m.group('desc')
        else:
            t = 'other'
            desc = c['subject']
        groups.setdefault(t, []).append((c['hash'][:7], c['date'], desc))
    return groups


DEFAULT_ORDER = ['feat', 'fix', 'perf', 'refactor', 'docs', 'chore', 'test', 'ci', 'other']


def render_changelog(groups, since_label=None):
    today = datetime.utcnow().date().isoformat()
    header = f"## {today} - {since_label or 'unreleased'}\n\n"
    lines = [header]
    summary_items = []
    for key in DEFAULT_ORDER + sorted(k for k in groups.keys() if k not in DEFAULT_ORDER):
        items = groups.get(key)
        if not items:
            continue
        title = key.capitalize()
        lines.append(f"### {title}\n")
        for h, d, desc in items:
            lines.append(f"- {desc} ({h}, {d})")
            summary_items.append(desc)
        lines.append("")

    # short summary: first lines joined
    summary = ' '.join(s for s in summary_items[:6])
    return f"{header}\n{summary}\n\n" + "\n".join(lines)


def write_output(path, content):
    with open(path, 'a', encoding='utf-8') as f:
        f.write(content)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--since-tag', help='Start from this tag (exclusive).')
    p.add_argument('--output', default='docs/CHANGELOG.md')
    args = p.parse_args()

    since = args.since_tag or get_latest_tag()
    if since:
        ref = since
    else:
        ref = None

    commits = commits_since(ref)
    if not commits:
        print('No commits found since', ref or 'start')
        sys.exit(0)

    groups = classify_commits(commits)
    content = render_changelog(groups, since_label=since)
    write_output(args.output, content)
    print('Wrote changelog to', args.output)


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Inventory claimed ATM10 ports for a source review; flags are not completeness proofs.

Reads metadata and Horizon Rust only. Does not change the worklist or deployed mods.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def audit(work):
    worklist = (work / 'porting-worklist.md').read_text()
    completed = set()
    for line in worklist.splitlines():
        if line.startswith('| `') and '✅ **Portado' in line:
            completed.add(re.match(r'\| `([^`]+)`', line)[1])
    batches = set(re.findall(r'- \[x\] \*\*`([^`]+)`', (ROOT / 'ports/BATCHES.md').read_text()))
    rows = []
    for mod in sorted(completed | batches):
        source = ROOT / 'ports' / mod / 'src/lib.rs'
        text = source.read_text() if source.exists() else ''
        # Restrict method inventory to production code, excluding unit-test modules.
        production = text.split('#[cfg(test)]')[0]
        methods = re.findall(r'^    fn (\w+)\(', production, re.M)
        base = work / 'mods' / mod
        status_path = base / 'porting-status.json'
        status = json.loads(status_path.read_text()) if status_path.exists() else None
        evidence = []
        for name in (status or {}).get('evidence', []):
            path = base / name
            entry = {'path': name, 'exists': path.is_file()}
            if path.is_file() and path.suffix == '.json':
                data = json.loads(path.read_text())
                if isinstance(data, dict):
                    entry['source'] = data.get('source')
                    entry['status'] = data.get('status')
            evidence.append(entry)
        flags = []
        if not text:
            flags.append('missing-source')
        if 'ticks: true' in production and 'tick_block_entities' not in methods:
            flags.append('declares-tick-without-override')
        if re.search(r'fn tick_block_entities\([^\n]*_handles', production):
            flags.append('ignores-supplied-handles-review-body')
        if status is None:
            flags.append('no-completeness-attestation')
        elif not evidence or not all(e['exists'] for e in evidence):
            flags.append('missing-evidence')
        rust = sorted(source.parent.glob('*.rs')) if source.exists() else []
        rows.append({
            'id': mod, 'worklist_complete': mod in completed, 'batch_checked': mod in batches,
            'source': str(source.relative_to(ROOT)), 'methods': methods,
            'source_sha256': {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in rust},
            'declares_ticking': 'ticks: true' in production,
            'complete_attestation': (status or {}).get('complete'), 'evidence': evidence,
            'deployed_wasm': (base / 'carry/mod.wasm').is_file(), 'flags': flags,
        })
    return {
        'generated_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'union of checked BATCHES.md rows and worklist Portado rows',
        'counts': {'rows': len(rows), 'worklist_complete': len(completed), 'batch_checked': len(batches)},
        'flag_counts': dict(Counter(f for row in rows for f in row['flags'])),
        'mods': rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    report = audit(args.work)
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'counts': report['counts'], 'flags': report['flag_counts']}))


if __name__ == '__main__':
    main()

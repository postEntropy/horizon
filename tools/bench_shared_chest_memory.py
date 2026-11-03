#!/usr/bin/env python3
"""Paired shared-chest host/WASM timing and Linux process-memory diagnostic.

Baseline directory contains port_tick_bench and both *_port.wasm snapshots.
Run without concurrent builds/tests. No installed pack or player data is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=3)
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error('--repeats must be positive')
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:6])
    os.nice(max(0, 10 - os.getpriority(os.PRIO_PROCESS, 0)))
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    binaries = set()
    for mod, block in [('dimstorage', 'dimensional_chest'), ('enderstorage', 'ender_chest')]:
        for count in (1, 100, 1000):
            for repeat in range(args.repeats):
                # Alternate order to reduce systematic warm-cache/temperature bias.
                for phase in (('before', 'after') if repeat % 2 == 0 else ('after', 'before')):
                    binary = (args.before / 'port_tick_bench' if phase == 'before' else
                              ROOT / 'target/release/examples/port_tick_bench').resolve()
                    core = (args.before / f'{mod}_port.wasm' if phase == 'before' else
                            ROOT / f'ports/target/wasm32-unknown-unknown/release/{mod}_port.wasm').resolve()
                    manifest = ROOT / f'ports/{mod}/horizon-mod.toml'
                    stem = f'{phase}-{mod}-{count}-{repeat}'
                    result = out / f'{stem}.json'
                    command = [str(binary), str(manifest.parent), str(core), f'{mod}:{block}',
                               str(count), '200', str(result)]
                    peak = 0
                    start = time.monotonic()
                    with (out / f'{stem}.log').open('w') as log:
                        process = subprocess.Popen(command, stdout=log, stderr=log)
                        while process.poll() is None:
                            try:
                                status = Path(f'/proc/{process.pid}/status').read_text()
                            except FileNotFoundError:
                                break
                            for line in status.splitlines():
                                if line.startswith('VmHWM:'):
                                    peak = max(peak, int(line.split()[1]))
                            time.sleep(0.001)
                        if process.wait():
                            raise RuntimeError(f'{stem} failed: see {stem}.log')
                    if not peak:
                        raise RuntimeError(f'{stem}: no memory sample')
                    row = {'phase': phase, 'mod': mod, 'count': count, 'repeat': repeat,
                           'peak_rss_kib': peak, 'wall_seconds': time.monotonic() - start,
                           'tick': json.loads(result.read_text())['idle'],
                           'host': str(binary.relative_to(ROOT)),
                           'core': str(core.relative_to(ROOT)),
                           'manifest': str(manifest.relative_to(ROOT))}
                    rows.append(row)
                    binaries.update((binary, core, manifest))
                    print(stem, peak, 'KiB', flush=True)
    # Hash after measuring: large binary reads must not inflate child-inherited RSS.
    hashes = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
              for path in sorted(binaries)}
    report = {'scope': 'host + WASM; explicit empty chest views; not game loop',
              'memory_method': '/proc/PID/status VmHWM sampled every 1 ms; may miss final <1 ms; includes JIT/create',
              'cpu_affinity': sorted(os.sched_getaffinity(0)),
              'nice': os.getpriority(os.PRIO_PROCESS, 0), 'samples': 200, 'warmup': 10,
              'sha256': hashes, 'results': rows}
    (out / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()

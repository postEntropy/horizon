#!/usr/bin/env python3
"""Observe cooking-pot storage/serving through the original mod as a black box.

Uses only our NeoForge driver and vanilla APIs. No mod source/class inspection.
Run under taskset -c 0-5 nice -n 10. Raw oracle data stays under .local/.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jar', type=Path, required=True)
    parser.add_argument('--raw', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')
    raw = args.raw.resolve()
    if not raw.is_relative_to(oracle.ROOT / '.local'):
        parser.error('--raw must be under repository .local/')
    raw.parent.mkdir(parents=True, exist_ok=True)
    scenarios = []
    for name in ['store', 'serve', 'blocked_output', 'full_meal', 'unlit', 'conductor', 'serve_stored', 'serve_partial', 'serve_without_container']:
        slots = {'0': ['farmersdelight:rice', 2]}
        if name == 'serve':
            slots['7'] = ['minecraft:bowl', 2]
        if name == 'blocked_output':
            slots['7'] = ['minecraft:bowl', 2]
            slots['8'] = ['minecraft:diamond', 64]
        if name == 'full_meal':
            slots['6'] = ['farmersdelight:cooked_rice', 64]
        neighbors = [{'x': 0, 'y': -1, 'z': 0, 'block': 'minecraft:magma_block'}]
        if name == 'unlit':
            neighbors[0].update(block='minecraft:campfire', properties={'lit': 'false'})
        if name == 'conductor':
            neighbors[0]['block'] = 'minecraft:hopper'
            neighbors.append({'x': 0, 'y': -2, 'z': 0, 'block': 'minecraft:magma_block'})
        events = []
        if name in ['serve_stored', 'serve_partial']:
            slots = {'0': ['farmersdelight:rice', 1]}
            events = [{'tick': 100, 'slot': 6, 'stack': ['farmersdelight:cooked_rice', 32]},
                      {'tick': 100, 'slot': 7, 'stack': ['minecraft:bowl', 32]},
                      {'tick': 100, 'x': 0, 'y': -1, 'z': 0, 'block': 'minecraft:stone'}]
            if name == 'serve_partial':
                events.append({'tick': 100, 'slot': 8, 'stack': ['farmersdelight:cooked_rice', 15]})
        if name == 'serve_without_container':
            slots = {'6': ['farmersdelight:dumplings', 64]}
            neighbors[0]['block'] = 'minecraft:stone'
        scenarios.append({'name': name, 'block': 'farmersdelight:cooking_pot',
                          'slots': slots, 'neighbors': neighbors, 'menu_slots': 9, 'events': events, 'ticks': 101 if name in ['serve_stored', 'serve_partial'] else (1 if name == 'serve_without_container' else 200)})
    raw.unlink(missing_ok=True)
    scenario_path = raw.with_suffix('.scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    with tempfile.TemporaryDirectory(prefix='horizon-farmer-original-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'farmer-serving-{uuid.uuid4().hex}', [args.jar], [exporter])
        seconds, log = oracle.start(home, run, {'horizon.oracle.out': raw,
                                              'horizon.oracle.machines': scenario_path}, timeout=90)
    if not raw.exists():
        raise RuntimeError(f'oracle produced no trace: {run}\n{log[-3000:]}')
    traces = json.loads(raw.read_text())
    final = {row['name']: row['trace'][-1]['slots'] for row in traces}
    assert final['store'] == {'6': ['farmersdelight:cooked_rice', 2]}, final
    assert final['serve'] == {'8': ['farmersdelight:cooked_rice', 2]}, final
    assert final['blocked_output'] == {'6': ['farmersdelight:cooked_rice', 2],
                                       '7': ['minecraft:bowl', 2], '8': ['minecraft:diamond', 64]}, final
    assert final['full_meal'] == {'0': ['farmersdelight:rice', 2],
                                  '6': ['farmersdelight:cooked_rice', 64]}, final
    assert final['unlit'] == {'0': ['farmersdelight:rice', 2]}, final
    assert final['conductor'] == final['store'], final
    assert final['serve_stored'] == {'6': ['farmersdelight:cooked_rice', 16],
                                    '7': ['minecraft:bowl', 16], '8': ['farmersdelight:cooked_rice', 16]}, final
    assert final['serve_partial'] == {'6': ['farmersdelight:cooked_rice', 31],
                                     '7': ['minecraft:bowl', 31], '8': ['farmersdelight:cooked_rice', 16]}, final
    assert final['serve_without_container'] == {'8': ['farmersdelight:dumplings', 64]}, final
    report = {'status': 'passed', 'scope': 'original mod black-box machine ticks through vanilla APIs',
              'jar_sha256': hashlib.sha256(args.jar.read_bytes()).hexdigest(),
              'raw_sha256': hashlib.sha256(raw.read_bytes()).hexdigest(),
              'raw_path': str(raw.relative_to(oracle.ROOT)),
              'cpu_affinity': sorted(os.sched_getaffinity(0)), 'nice': os.getpriority(os.PRIO_PROCESS, 0),
              'oracle_wall_seconds': seconds, 'final_slots': final,
              'remaining': ['implement and compare Horizon meal storage/serving', 'full game validation']}
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(traces)} original cooking-pot scenarios passed; {seconds:.2f}s')


if __name__ == '__main__':
    main()

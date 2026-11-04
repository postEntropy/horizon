#!/usr/bin/env python3
"""Validate only item-in-air PASS cases observed against the original mod."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
ITEMS = ['allthetweaks:atm_star', 'allthetweaks:greg_star', 'allthetweaks:withers_compass']


def count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        rows = []
        for item in ITEMS:
            game.command(f'item replace entity @s hotbar.0 with {item}')
            game.call('hotbar', '1')
            before = game.call('inventory')
            game.call('use-item')
            after = game.call('inventory')
            before_count, after_count = count(before, item), count(after, item)
            assert before_count == after_count == 1, (mode, item, before_count, after_count)
            rows.append({'item': item, 'count_before': before_count, 'count_after': after_count})
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/allthetweaks-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/allthetweaks-item-use-l6.json')
    args = p.parse_args()
    for name in ('hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'status': 'passed', 'cpu_affinity': sorted(os.sched_getaffinity(0)),
                                    'scope': 'three original-observed item-use-in-air PASS cases; stack preserved',
                                    'cases': cases}, indent=2) + '\n')
    print('AllTheTweaks observed item-use PASS cases preserved in both game modes')


if __name__ == '__main__':
    main()

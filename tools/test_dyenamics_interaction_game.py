#!/usr/bin/env python3
"""Validate only the original-observed Dyenamics dye-on-block PASS paths."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 102, 0)
CASES = [
    ('vanilla_wool', 'minecraft:white_wool', 'dyenamics:amber_dye'),
    ('dyenamics_wool', 'dyenamics:amber_wool', 'dyenamics:aquamarine_dye'),
    ('dyenamics_glass', 'dyenamics:amber_stained_glass', 'dyenamics:aquamarine_dye'),
    ('non_color_block', 'minecraft:stone', 'dyenamics:amber_dye'),
    ('vanilla_dye_control', 'minecraft:white_wool', 'minecraft:blue_dye'),
]


def item_count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def has_block(game, block):
    x, y, z = POS
    marker = 'DYENAMICS_TARGET_PRESENT'
    reply = game.command(f'execute if block {x} {y} {z} {block} run say {marker}')
    return marker in reply


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.call('setblock', 0, 101, 1, 'minecraft:stone')
        game.command('tp @s 0.5 102 1.5')
        rows = []
        for name, block, dye in CASES:
            game.call('setblock', *POS, block)
            game.command(f'item replace entity @s hotbar.0 with {dye}')
            game.call('hotbar', '1')
            before = game.call('inventory')
            assert has_block(game, block), (name, 'target block missing before click')
            game.call('interact', *POS)
            after = game.call('inventory')
            assert has_block(game, block), (name, 'block changed after observed PASS path')
            assert item_count(after, dye) == item_count(before, dye) == 1, (name, before, after)
            rows.append({'case': name, 'block': block, 'item': dye,
                         'block_unchanged': True, 'item_count_before': 1, 'item_count_after': 1})
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/dyenamics-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/dyenamics-interaction-l6.json')
    args = p.parse_args()
    for name in ('hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed',
        'scope': 'five original-observed item-on-block PASS cases: unchanged target and unconsumed item',
        'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'cases': cases,
    }, indent=2) + '\n')
    print('Dyenamics PASS interactions preserved in both game modes')


if __name__ == '__main__':
    main()

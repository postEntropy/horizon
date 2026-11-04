#!/usr/bin/env python3
"""Validate only the four Chisel right-click results recorded in MachineTrace."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 102, 0)
CASES = [
    ('stone_with_chisel', 'minecraft:stone', 'chisel:chisel'),
    ('array_stone_cycle', 'chisel:array/stone', 'chisel:chisel'),
    ('unsupported_bedrock', 'minecraft:bedrock', 'chisel:chisel'),
    ('stone_empty_hand', 'minecraft:stone', 'minecraft:air'),
]


def has_block(game, block):
    marker = 'CHISEL_TARGET_UNCHANGED'
    x, y, z = POS
    return marker in game.command(f'execute if block {x} {y} {z} {block} run say {marker}')


def item_count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.call('setblock', 0, 101, 1, 'minecraft:stone')
        game.command('tp @s 0.5 102 1.5')
        rows = []
        for name, block, item in CASES:
            game.call('setblock', *POS, block)
            game.command(f'item replace entity @s hotbar.0 with {item}')
            game.call('hotbar', '1')
            before = game.call('inventory')
            assert has_block(game, block), (mode, name, 'target block missing before use')
            game.call('interact', *POS)
            after = game.call('inventory')
            assert has_block(game, block), (mode, name, 'block changed')
            if item != 'minecraft:air':
                assert item_count(before, item) == item_count(after, item) == 1, (mode, name, before, after)
            rows.append({'case': name, 'original_trace_result': 'PASS_TO_DEFAULT_BLOCK_INTERACTION',
                         'block_unchanged': True, 'item_count_before': item_count(before, item),
                         'item_count_after': item_count(after, item)})
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/chisel-l6-carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/chisel.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/chisel/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/chisel-observed.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/chisel-interaction-l6.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')

    block = args.carry / 'data/chisel/horizon/block/array/stone.json'
    item = args.carry / 'data/chisel/horizon/item/chisel.json'
    block.parent.mkdir(parents=True, exist_ok=True)
    item.parent.mkdir(parents=True, exist_ok=True)
    block.write_text('{"template":"minecraft:stone"}\n')
    item.write_text('{"template":"minecraft:iron_pickaxe"}\n')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'partial; exact four measured right-clicks reproduced',
        'scope': 'Chisel tool on stone, array/stone, and bedrock plus empty hand on stone; all blocks and stacks remain unchanged. Other Chisel mechanics remain unverified.',
        'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'cases': cases,
    }, indent=2) + '\n')
    print('Chisel observed right-click outcomes passed in both game modes')


if __name__ == '__main__':
    main()

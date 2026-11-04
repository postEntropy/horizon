#!/usr/bin/env python3
"""Validate the exact four Macaw's Lights outcomes observed in MachineTrace."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 102, 0)
LAMP = 'mcwlights:white_paper_lamp'
GARDEN = 'mcwlights:garden_light'


def has_block(game, block):
    marker = 'MCWLIGHTS_TARGET_PRESENT'
    x, y, z = POS
    reply = game.command(f'execute if block {x} {y} {z} {block} run say {marker}')
    return marker in reply


def item_count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.call('setblock', 0, 101, 1, 'minecraft:stone')
        game.command('tp @s 0.5 102 1.5')
        rows = []
        for name, state, item in [
            ('garden_off_empty_hand', f'{GARDEN}[lit=false,powered=false]', 'minecraft:air'),
            ('garden_on_empty_hand', f'{GARDEN}[lit=true,powered=false]', 'minecraft:air'),
            ('white_lamp_red_dye', LAMP, 'minecraft:red_dye'),
            ('white_lamp_stick', LAMP, 'minecraft:stick'),
        ]:
            game.call('setblock', *POS, state)
            game.command(f'item replace entity @s hotbar.0 with {item}')
            game.call('hotbar', '1')
            before = game.call('inventory')
            assert has_block(game, state), (mode, name, 'block missing before use')
            game.call('interact', *POS)
            after = game.call('inventory')
            assert has_block(game, state), (mode, name, 'block changed')
            if item != 'minecraft:air':
                assert item_count(before, item) == item_count(after, item) == 1, (mode, name, before, after)
            rows.append({'case': name, 'original_trace_result': 'CONSUME' if item != 'minecraft:air' else 'PASS',
                         'block_unchanged': True,
                         'item_count_before': item_count(before, item),
                         'item_count_after': item_count(after, item)})
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/mcwlights-l6-carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/mcwlights.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/mcwlights/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/mcwlights-observed.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/mcwlights-interaction-l6.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')

    lamp = args.carry / 'data/mcwlights/horizon/block/white_paper_lamp.json'
    garden = args.carry / 'data/mcwlights/horizon/block/garden_light.json'
    lamp.parent.mkdir(parents=True, exist_ok=True)
    lamp.write_text('{"template":"minecraft:stone"}\n')
    garden.write_text('{"template":"minecraft:stone","state_properties":{"lit":["false","true"],"powered":["false","true"]}}\n')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed; exact four observed traces only',
        'original_trace': 'MachineTrace: empty-hand garden_light lit=false/true -> PASS unchanged; red dye/stick on white_paper_lamp -> CONSUME unchanged, held count 1',
        'scope': 'No other colors, lamps, toggles, or item interactions were inferred.',
        'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'cases': cases,
    }, indent=2) + '\n')
    print("Macaw's Lights observed interactions passed in both game modes")


if __name__ == '__main__':
    main()

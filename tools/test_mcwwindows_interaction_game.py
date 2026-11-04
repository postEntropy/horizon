#!/usr/bin/env python3
"""Check the observed Macaw's Windows shutter interaction in both game modes."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
SHUTTER = 'mcwwindows:oak_shutter'
POS = (0, 102, 0)
ITEMS = [('empty_hand', 'minecraft:air'), ('hammer', 'mcwwindows:hammer'),
         ('key', 'mcwwindows:key')]


def is_state(game, state):
    x, y, z = POS
    reply = game.command(f'execute if block {x} {y} {z} {SHUTTER}[open={state}] run say MCW_STATE_{state.upper()}')
    return f'MCW_STATE_{state.upper()}' in reply


def item_count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.call('setblock', 0, 101, 1, 'minecraft:stone')
        game.command('tp @s 0.5 102 1.5')
        records = []
        for name, item in ITEMS:
            game.call('setblock', *POS, f'{SHUTTER}[open=false]')
            if item == 'minecraft:air':
                game.command('item replace entity @s hotbar.0 with minecraft:air')
            else:
                game.command(f'item replace entity @s hotbar.0 with {item}')
            game.call('hotbar', '1')
            before = game.call('inventory')
            assert not is_state(game, 'true'), (name, 'expected closed before interaction')
            game.call('interact', *POS)
            assert is_state(game, 'true'), (name, 'did not open shutter')
            game.step(5)
            game.call('interact', *POS)
            assert is_state(game, 'false'), (name, 'did not close shutter')
            after = game.call('inventory')
            if item != 'minecraft:air':
                assert item_count(after, item) == item_count(before, item) == 1, (name, before, after)
            records.append({'case': name, 'item': item, 'opened': True, 'closed_again': True,
                            'item_count_before': item_count(before, item),
                            'item_count_after': item_count(after, item)})
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': records}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=Path('/home/leonardo/.local/share/horizon/ports/all-the-mods-10-atm10/mods/mcwwindows/carry'))
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/mcwwindows.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/mcwwindows/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/mcwwindows-l6/mcwwindows-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/mcwwindows-interaction-l6.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    rows = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed', 'original': 'NeoForge 21.1.259, Macaw\'s Windows 2.4.2',
        'scope': 'oak shutter open state with empty hand, hammer and key; each item opens then closes, no item consumed',
        'cases': rows,
    }, indent=2) + '\n')
    print('Macaw\'s Windows shutter interactions passed in both game modes')


if __name__ == '__main__':
    main()

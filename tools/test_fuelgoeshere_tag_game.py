#!/usr/bin/env python3
"""Validate the original Fuel Goes Here forced_fuels tag in Horizon."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
CASES = [
    ('minecraft:coal', True),
    ('minecraft:charcoal', True),
    ('minecraft:stick', False),
    ('minecraft:dirt', False),
]


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('item replace entity @s hotbar.0 with minecraft:oak_planks')
        vanilla = game.command('execute if items entity @s hotbar.0 #minecraft:planks run say VANILLA_ITEM_TAG_MATCH')
        assert 'VANILLA_ITEM_TAG_MATCH' in vanilla, (mode, 'vanilla tag baseline', vanilla)
        rows = []
        for item, expected in CASES:
            game.command(f'item replace entity @s hotbar.0 with {item}')
            reply = game.command('execute if items entity @s hotbar.0 #fuelgoeshere:forced_fuels run say FUEL_TAG_MATCH')
            matched = 'FUEL_TAG_MATCH' in reply
            assert matched == expected, (mode, item, expected, reply)
            rows.append({'item': item, 'tag_member': matched})
        try:
            game.command('fuel coal')
        except AssertionError as error:
            assert 'Unknown or incomplete command' in repr(error), (mode, error)
        else:
            raise AssertionError((mode, 'original has no /fuel command'))
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/fuelgoeshere-l6-carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/fuelgoeshere.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/fuelgoeshere/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/fuelgoeshere-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/fuelgoeshere-tag-game-l6.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    tag = args.carry / 'data/fuelgoeshere/tags/item/forced_fuels.json'
    tag.parent.mkdir(parents=True, exist_ok=True)
    tag.write_text('{"replace":false,"values":["minecraft:coal","minecraft:charcoal"]}\n')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'status': 'passed', 'cpu_affinity': sorted(os.sched_getaffinity(0)),
                                    'scope': 'forced_fuels membership only; not burn-duration parity',
                                    'cases': cases}, indent=2) + '\n')
    print('Fuel Goes Here forced_fuels tag validated in both game modes')


if __name__ == '__main__':
    main()

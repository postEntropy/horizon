#!/usr/bin/env python3
"""Validate all observed battery upgrade orders in both real game modes."""
import argparse
import itertools
import json
import os
from pathlib import Path
import re

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
GADGETS = ['mininggadgets:mininggadget_simple', 'mininggadgets:mininggadget',
           'mininggadgets:mininggadget_fancy']
UPGRADES = {1: 'mininggadgets:upgrade_battery_1',
            2: 'mininggadgets:upgrade_battery_2',
            3: 'mininggadgets:upgrade_battery_3'}
CAPACITY = {1: 2_000_000, 2: 5_000_000, 3: 10_000_000}


def scenarios():
    rows = []
    for gadget in GADGETS:
        for tier in (1, 2, 3):
            rows.append((f'{gadget.rsplit(":", 1)[-1]}_single_{tier}', gadget, [tier]))
    for first, second in itertools.permutations((1, 2, 3), 2):
        rows.append((f'pair_{first}_{second}', GADGETS[1], [first, second]))
    for order in itertools.permutations((1, 2, 3)):
        rows.append((f'triple_{"_".join(map(str, order))}', GADGETS[1], list(order)))
    for tier in (1, 2, 3):
        rows.append((f'duplicate_{tier}', GADGETS[1], [tier, tier]))
    assert len(rows) == 24
    return rows


def read_tag(game, pos):
    return game.command(f'data get block {pos[0]} {pos[1]} {pos[2]} Items')


def validate_case(game, case, index):
    name, gadget, tiers = case
    x, y, z = (index % 6) * 3, 100, (index // 6) * 3
    pos = (x, y, z)
    game.call('setblock', *pos, 'mininggadgets:modificationtable')
    game.command(f'item replace block {x} {y} {z} container.0 with {gadget}')
    for slot, tier in enumerate(tiers, 1):
        game.command(f'item replace block {x} {y} {z} container.{slot} with {UPGRADES[tier]}')
    return {'name': name, 'pos': pos, 'gadget': gadget, 'tiers': tiers,
            'expected_capacity': CAPACITY[tiers[0]],
            'expected_remaining': [UPGRADES[tier] for tier in tiers[1:]]}


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('tp @s 8 102 7')
        cases = [validate_case(game, case, index) for index, case in enumerate(scenarios())]
        game.step(2)
        for case in cases:
            tag = read_tag(game, case['pos'])
            cap = re.search(r'mininggadgets:forge_energy_max_energy["\']?:\s*(\d+)', tag)
            stored = re.search(r'horizon:energy["\']?:\s*(\d+)[lL]?', tag)
            assert cap and int(cap[1]) == case['expected_capacity'], (case, tag)
            assert stored and int(stored[1]) == 1, (case, tag)
            remaining = [item for item in case['expected_remaining'] if item in tag]
            assert remaining == case['expected_remaining'], (case, tag)
            assert 'upgrade_battery_' not in tag or remaining, (case, tag)
            case['observed_capacity'] = int(cap[1])
            case['observed_stored'] = int(stored[1])
            case['observed_remaining'] = remaining
        game.stop()
        return {'mode': mode, 'status': 'passed', 'scenario_count': len(cases), 'cases': cases}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/mininggadgets-work/mods/mininggadgets/carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/mininggadgets.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/mininggadgets/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/mininggadgets-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/mininggadgets-battery-game.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    rows = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed', 'original_scenario_count': 24,
        'scope': 'first battery tier, capacities, initial 1 FE and later-upgrade retention',
        'cases': rows,
    }, indent=2) + '\n')
    print('Mining Gadgets battery upgrade orders passed in both game modes')


if __name__ == '__main__':
    main()

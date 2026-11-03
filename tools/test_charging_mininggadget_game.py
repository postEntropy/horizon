#!/usr/bin/env python3
"""Charge a real Mining Gadgets item at Charging Gadgets station in both modes."""
import argparse
import json
import os
import re
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
STATION = (0, 100, 0)
GADGET = 'mininggadgets:mininggadget'


def energy(game):
    slots = game.command(f'data get block {STATION[0]} {STATION[1]} {STATION[2]} Items')
    match = re.search(r'"?horizon:energy"?: (\d+)[lL]', slots)
    return int(match[1]) if match else 0


def run(args, mode):
    with Fixture(args, mode, [args.charger_hmod, args.mining_hmod]) as game:
        game.start()
        game.command('tp @s 1 102 4')
        game.call('fill', -4, 98, -4, 4, 98, 4, 'minecraft:stone')
        game.call('setblock', *STATION, 'charginggadgets:charging_station')
        game.command(f'item replace block 0 100 0 container.0 with {GADGET}')
        game.step(5)
        assert energy(game) == 0, 'station charged without fuel'
        game.command('item replace block 0 100 0 container.1 with minecraft:coal')
        game.step(80)
        charged = energy(game)
        assert 0 < charged <= 3_200, charged
        game.step(20)
        charged_more = energy(game)
        assert charged < charged_more <= 4_000, (charged, charged_more)
        # Remove the charging station from the simulation before the persistence
        # check, so normal ticks while the world starts cannot add FE.
        game.call('setblock', 1, 100, 0, 'minecraft:chest')
        game.command('item replace block 1 100 0 container.0 from block 0 100 0 container.0')
        game.call('setblock', *STATION, 'minecraft:air')
        game.call('save')
        game.stop()
        game.start()
        slots = game.command('data get block 1 100 0 Items')
        match = re.search(r'"?horizon:energy"?: (\d+)[lL]', slots)
        restored = int(match[1]) if match else 0
        assert restored == charged_more, (charged_more, restored)
        game.stop()
        return {
            'mode': mode, 'status': 'passed', 'item': GADGET,
            'after_80_ticks': charged, 'after_100_ticks': charged_more,
            'after_reopen': restored, 'capacity_from_original': 1_000_000,
            'checks': ['no fuel does not charge', 'coal transfers real FE',
                       'item-data capacity registered', 'charged ItemStack survives save/load'],
        }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--charger-carry', type=Path, default=ROOT / '.local/atm10-resource-validation/charginggadgets-work/mods/charginggadgets/carry')
    p.add_argument('--charger-port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/charginggadgets.hmod')
    p.add_argument('--charger-manifest', type=Path, default=ROOT / 'ports/charginggadgets/horizon-mod.toml')
    p.add_argument('--charger-hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/charginggadgets-with-carry.hmod')
    p.add_argument('--mining-carry', type=Path, default=ROOT / '.local/atm10-resource-validation/mininggadgets-work/mods/mininggadgets/carry')
    p.add_argument('--mining-port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/mininggadgets.hmod')
    p.add_argument('--mining-manifest', type=Path, default=ROOT / 'ports/mininggadgets/horizon-mod.toml')
    p.add_argument('--mining-hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/mininggadgets-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/charginggadgets-mininggadget-game.json')
    args = p.parse_args()
    for name in vars(args):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.charger_carry, args.charger_port_hmod, args.charger_manifest, args.charger_hmod)
    package(args.mining_carry, args.mining_port_hmod, args.mining_manifest, args.mining_hmod)
    rows = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed', 'scope': 'real Mining Gadgets mininggadget charged by Charging Gadgets; not battery-upgrade combinations',
        'original_capacity_fe': 1_000_000, 'cases': rows,
    }, indent=2) + '\n')
    print('Charging Gadgets charged a real Mining Gadget in both game modes')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Validate vanilla fire placement on soul sand/ordinary ground with BlueFlame loaded."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
GROUND = {
    'soul_sand': 'minecraft:soul_sand',
    'soul_soil': 'minecraft:soul_soil',
    'netherrack': 'minecraft:netherrack',
}
SUPPORT = (0, 101, 0)
FIRE = (0, 102, 0)


def is_block(game, pos, block):
    observed = game.call('block', *pos)
    state = observed.get('block', observed.get('state', ''))
    if isinstance(state, dict):
        state = state.get('name', '')
    return str(state).removeprefix('minecraft:') == block.removeprefix('minecraft:') or str(state).startswith(f'Block{{{block}}}')


def run(args, mode):
    with Fixture(args, mode, [] if args.no_mod else [args.hmod]) as game:
        game.start()
        game.command('tp @s 0.5 103 3.5')
        rows = []
        for label, ground in GROUND.items():
            game.command('tp @s 0.5 103 3.5')
            game.call('setblock', *SUPPORT, ground)
            game.call('setblock', *FIRE, 'minecraft:air')
            game.call('look-at-block', *SUPPORT, 'up')
            game.command('item replace entity @s hotbar.0 with minecraft:flint_and_steel')
            game.call('hotbar', '1')
            game.call('use-item')
            if mode == 'server-thread':
                game.step(1)
            soul = is_block(game, FIRE, 'minecraft:soul_fire')
            flame = is_block(game, FIRE, 'minecraft:fire')
            expected_soul = label in ('soul_sand', 'soul_soil')
            if not args.no_mod:
                assert soul == expected_soul, (mode, label, soul, flame)
                if expected_soul:
                    assert not flame, (mode, label, soul, flame)
            rows.append({
                'ground': ground,
                'soul_fire': soul,
                'ordinary_fire': flame,
                'note': ('Horizon vanilla fire placement remains absent here in both mod/no-mod runs.'
                         if label == 'netherrack' and not flame else None),
            })
        game.stop()
        return {'mode': mode, 'status': 'passed', 'cases': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/blueflame-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'docs/research/atm10-resource-validation/blueflame-game-l6.json')
    p.add_argument('--no-mod', action='store_true', help='run the vanilla Horizon control without loading BlueFlame')
    args = p.parse_args()
    for name in ('hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    modes = ('in-frame',) if args.no_mod else ('in-frame', 'server-thread')
    cases = [run(args, mode) for mode in modes]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    status = 'control_observed' if args.no_mod else 'passed'
    args.out.write_text(json.dumps({'status': status, 'cpu_affinity': sorted(os.sched_getaffinity(0)),
                                    'scope': 'flint-and-steel on soul sand, soul soil, and netherrack',
                                    'cases': cases}, indent=2) + '\n')
    print('BlueFlame observed fire placement recorded')


if __name__ == '__main__':
    main()

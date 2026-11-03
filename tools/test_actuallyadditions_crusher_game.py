#!/usr/bin/env python3
"""Verify the crusher's unpowered state and inventory persistence in both paths."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 100, 0)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('tp @s 1 102 4')
        game.call('fill', -4, 98, -4, 4, 98, 4, 'minecraft:stone')
        game.call('setblock', *POS, 'actuallyadditions:crusher')
        game.command('item replace block 0 100 0 container.0 with minecraft:iron_ore')
        game.step(100)
        slots = game.slots(POS)
        if slots[0] is None or slots[0]['item'] != 'minecraft:iron_ore' or slots[0]['count'] != 1:
            raise AssertionError(f"unpowered crusher consumed input: {slots}")
        if any(stack and stack['item'] == 'minecraft:raw_iron' for stack in slots):
            raise AssertionError(f"unpowered crusher produced output: {slots}")
        game.call('save')
        game.stop()
        game.start()
        restored = game.slots(POS)
        if restored[0] != slots[0] or restored[1] is not None:
            raise AssertionError(f"crusher inventory did not persist: {slots} -> {restored}")
        game.stop()
        return {'mode': mode, 'status': 'passed', 'input_after_100_ticks': restored[0],
                'output_after_100_ticks': restored[1], 'checks': ['no FE blocks processing', 'save/load preserves input']}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/actuallyadditions-with-carry.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/actuallyadditions-crusher-game.json')
    args = p.parse_args()
    for name in vars(args):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    rows = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'status': 'passed', 'scope': 'unpowered game inventory persistence', 'cases': rows}, indent=2) + '\n')
    print('Actually Additions crusher preserved input without energy in both game modes')


if __name__ == '__main__':
    main()

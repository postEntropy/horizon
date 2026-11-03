#!/usr/bin/env python3
"""Cooking-pot heat, meal storage, menu restrictions, hoppers and disk reopen.

Uses a scratch hmod and fresh game homes; does not touch installed packs/worlds.
"""
import argparse
import json
import os
import time
from pathlib import Path
from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
POT = (0, 100, 0)
HEAT = (0, 99, 0)
RICE = 'farmersdelight:rice'
MEAL = 'farmersdelight:cooked_rice'


def count(slots, slot, item):
    stack = slots[slot]
    return stack['count'] if stack and stack['item'] == item else 0


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('tp @s 0 101 2')
        game.call('fill', -3, 98, -3, 3, 98, 3, 'minecraft:stone')
        game.call('setblock', *HEAT, 'minecraft:stone')
        game.call('setblock', *POT, 'farmersdelight:cooking_pot')
        game.replace(POT, 0, RICE + ' 2')
        game.step(110)
        assert count(game.slots(POT), 0, RICE) == 2
        assert not game.total([POT], MEAL)
        game.call('setblock', *HEAT, 'minecraft:campfire[lit=false]')
        game.step(110)
        assert count(game.slots(POT), 0, RICE) == 2
        assert not game.total([POT], MEAL)
        game.call('setblock', *HEAT, 'minecraft:campfire[lit=true]')
        game.step(50)
        assert not game.total([POT], MEAL)
        game.call('save')
        game.stop()
        game.start()
        game.step(50)
        slots = game.slots(POT)
        assert count(slots, 0, RICE) == 1, slots
        assert count(slots, 6, MEAL) == 1, slots
        assert slots[8] is None, slots
        # Cooking can continue behind a blocked served output. Bowls are only
        # consumed when serving succeeds, not while the internal meal is made.
        game.replace(POT, 7, 'minecraft:bowl 2')
        game.replace(POT, 8, 'minecraft:stone 64')
        game.step(100)
        slots = game.slots(POT)
        assert slots[0] is None, slots
        assert count(slots, 6, MEAL) == 2, slots
        assert count(slots, 7, 'minecraft:bowl') == 2, slots
        assert count(slots, 8, 'minecraft:stone') == 64, slots
        game.replace(POT, 8, 'minecraft:air')
        game.step(1)
        slots = game.slots(POT)
        assert slots[6] is None and slots[7] is None, slots
        assert count(slots, 8, MEAL) == 2, slots
        # The hopper is a heat conductor; it may take served food, never meals.
        game.call('setblock', 1, 99, 0, 'minecraft:chest')
        game.call('setblock', 0, 98, 0, 'minecraft:magma_block')
        game.call('setblock', *HEAT, 'minecraft:hopper[facing=east]')
        game.step(30)
        assert game.total([(1, 99, 0)], MEAL) == 2
        assert not game.total([POT], MEAL)
        game.replace(POT, 6, MEAL + ' 64')
        game.replace(POT, 0, RICE)
        game.step(110)
        slots = game.slots(POT)
        assert count(slots, 6, MEAL) == 64 and count(slots, 0, RICE) == 1, slots
        assert game.total([(1, 99, 0)], MEAL) == 2
        game.call('save')
        game.stop()
        game.start()
        assert count(game.slots(POT), 6, MEAL) == 64
        assert count(game.slots(POT), 0, RICE) == 1
        # The player's click path must enforce the same protected reservoir.
        game.command('tp @s 0 101 2')
        game.call('interact', *POT)
        game.call('click-slot', 306)
        menu = game.call('menu')
        assert not menu.get('carried'), menu
        assert count(game.slots(POT), 6, MEAL) == 64
        game.call('click-slot', 306, 'shift')
        assert count(game.slots(POT), 6, MEAL) == 64
        game.call('close')
        # Serve a batch without a heat source, respecting the observed maximum.
        game.call('setblock', *HEAT, 'minecraft:stone')
        game.replace(POT, 7, 'minecraft:bowl 64')
        game.step(1)
        slots = game.slots(POT)
        assert count(slots, 6, MEAL) == 48, slots
        assert count(slots, 7, 'minecraft:bowl') == 48, slots
        assert count(slots, 8, MEAL) == 16, slots
        game.command('tp @s 0 101 2')
        game.call('interact', *POT)
        game.call('click-slot', 308)
        menu = game.call('menu')
        assert menu.get('carried', {}).get('item') == MEAL, menu
        assert menu['carried']['count'] == 16, menu
        # Integrated-server clicks are sent asynchronously. Wait for the
        # authoritative inventory update, without advancing simulation ticks.
        deadline = time.monotonic() + 5
        slots = game.slots(POT)
        while slots[8] is not None and time.monotonic() < deadline:
            time.sleep(0.1)
            slots = game.slots(POT)
        assert slots[8] is None, slots
        assert count(slots, 6, MEAL) == 48, slots
        game.call('close')
        game.stop()
        return {'mode': mode, 'status': 'passed', 'checks': [
            'cold and unlit do not cook', 'real campfire cooks', 'mid-progress disk reopen',
            'cooking stores meals without container', 'blocked served output preserves bowls',
            'hopper extracts served food but not reservoir', 'full reservoir preserves ingredients',
            '64 stored meal survives disk reopen', 'player pickup and shift-click refuse reservoir',
            'serving without heat respects observed stack limit', 'player can take served output']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hmod', type=Path, required=True)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--mode', choices=['both', 'in-frame', 'server-thread'], default='both')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    for name in ['hmod', 'bin_dir', 'resources', 'out']:
        setattr(args, name, getattr(args, name).resolve())
    assert len(os.sched_getaffinity(0)) <= 6 and os.getpriority(os.PRIO_PROCESS, 0) >= 10
    modes = ['in-frame', 'server-thread'] if args.mode == 'both' else [args.mode]
    rows = []
    for mode in modes:
        row = run(args, mode)
        rows.append(row)
        # Preserve each successful path even if a later path fails.
        checkpoint = args.out.with_name(f'{args.out.stem}-{mode}-checkpoint.json')
        checkpoint.write_text(json.dumps(row, indent=2) + '\n')
    args.out.write_text(json.dumps({'status': 'passed', 'scope': 'real game; isolated Farmer carry and cooked-rice recipe, not complete mod parity', 'cases': rows}, indent=2) + '\n')


if __name__ == '__main__':
    main()

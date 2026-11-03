#!/usr/bin/env python3
"""Real-game consumption checks for the three vanilla-class Farmer's Delight foods."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
ITEMS = {
    'farmersdelight:wheat_dough': (2, 1.2, 1.6),
    'farmersdelight:raw_pasta': (2, 1.2, 1.6),
    'farmersdelight:chicken_cuts': (1, 0.6, 0.8),
}


def snapshot(game):
    state = game.call('state')
    return state['player'], game.call('inventory')


def held_count(inventory, item):
    slots = inventory.get('hotbar', []) + inventory.get('main', [])
    return sum(s['count'] for s in slots if s and s['item'] == item)


def run(args, mode):
    with Fixture(args, mode, [args.carry]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('effect give @s hunger 120 255')
        game.step(80)
        player, _ = snapshot(game)
        if player['food'] >= 20:
            raise AssertionError(f'hunger setup did not lower food: {player}')
        game.command('effect clear @s')
        rows = []
        for item, (nutrition, saturation, seconds) in ITEMS.items():
            game.command('item replace entity @s hotbar.0 with ' + item + ' 16')
            game.call('hotbar', '1')
            before, inv_before = snapshot(game)
            # Releasing before the observed use duration must not consume the
            # stack; holding slightly longer must complete it below.
            game.call('use-item', '--hold', str(seconds - 0.25))
            _, inv_short = snapshot(game)
            if held_count(inv_short, item) != held_count(inv_before, item):
                raise AssertionError((item, 'consumed before consume_seconds', seconds))
            before, inv_before = snapshot(game)
            # Run the original probability (0.3); repeat until Hunger appears,
            # with a generous finite bound so a random miss is not a failure.
            got_hunger = False
            attempts = 0
            for attempts in range(1, 16):
                game.call('use-item', '--hold', str(seconds + 0.2))
                after, inv_after = snapshot(game)
                expected_count = held_count(inv_before, item) - 1
                if held_count(inv_after, item) != expected_count:
                    raise AssertionError((item, 'inventory', inv_before, inv_after))
                if after['food'] < before['food'] or after['food'] > min(20, before['food'] + nutrition):
                    raise AssertionError((item, 'food delta', before, after))
                if after['food'] > before['food'] and after['saturation'] + 0.05 < before['saturation']:
                    raise AssertionError((item, 'saturation decreased', before, after))
                got_hunger = any(e['id'] == 'hunger' and e['amplifier'] == 0
                                 and e['duration_ticks'] >= 1 for e in after['effects'])
                rows.append({'item': item, 'attempt': attempts,
                             'consume_seconds': seconds, 'short_hold_no_consume': True,
                             'food_before': before['food'], 'food_after': after['food'],
                             'saturation_before': before['saturation'],
                             'saturation_after': after['saturation'],
                             'remaining': held_count(inv_after, item),
                             'hunger_effect_observed': got_hunger})
                if got_hunger:
                    break
                # Restore low food with a distinguishable high-amplifier effect,
                # then clear it so it cannot be mistaken for the food's effect
                # or drain saturation during the next consumption.
                game.command('effect give @s hunger 120 255')
                game.step(80)
                game.command('effect clear @s')
                before, inv_before = after, inv_after
            if not got_hunger:
                raise AssertionError((item, '30% Hunger effect not observed in 15 trials', rows,
                                      'last player snapshot', after))
        game.stop()
        return {'mode': mode, 'status': 'passed', 'attempts': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-food-regenerated/mods/farmersdelight/carry')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    for n in ['carry', 'bin_dir', 'resources', 'out']:
        setattr(a, n, getattr(a, n).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    results = [run(a, mode) for mode in ['in-frame', 'server-thread']]
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps({'status': 'passed', 'scope': 'three imported vanilla-class foods; real game consumption, not whole-mod parity', 'cases': results}, indent=2) + '\n')
    print('both real-game modes passed')


if __name__ == '__main__':
    main()

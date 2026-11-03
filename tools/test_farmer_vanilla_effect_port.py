#!/usr/bin/env python3
"""Real-game checks for the imported ConsumableItem vanilla-effect group."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package

ROOT = Path(__file__).resolve().parents[1]
IDS = [
    'farmersdelight:apple_pie_slice', 'farmersdelight:cake_slice',
    'farmersdelight:chocolate_pie_slice', 'farmersdelight:fruit_salad',
    'farmersdelight:glow_berry_custard', 'farmersdelight:mixed_salad',
    'farmersdelight:nether_salad', 'farmersdelight:pumpkin_pie_slice',
    'farmersdelight:sweet_berry_cheesecake_slice',
]


def count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def state(game):
    return game.call('state')['player'], game.call('inventory')


def prepare_hungry(game):
    game.command('effect give @s hunger 120 255')
    game.step(80)
    game.command('effect clear @s')
    player, _ = state(game)
    if player['food'] >= 20:
        raise AssertionError(f'hunger setup failed: {player}')
    return player


def consume(game, item_id, declaration, effect_name):
    before = prepare_hungry(game)
    game.command(f'item replace entity @s hotbar.0 with {item_id} 1')
    game.call('hotbar', '1')
    _, inventory_before = state(game)
    remainder = declaration.get('use_remainder')
    previous_remainder = count(inventory_before, remainder) if remainder else 0
    seconds = declaration['food']['consume_seconds']
    game.call('use-item', '--hold', str(seconds + 0.2))
    after, inventory_after = state(game)
    expected_food = min(20, before['food'] + declaration['food']['nutrition'])
    expected_saturation = min(expected_food, before['saturation'] + declaration['food']['saturation'])
    if after['food'] != expected_food or abs(after['saturation'] - expected_saturation) > 1e-4:
        raise AssertionError((item_id, 'food data', before, after, declaration['food']))
    if count(inventory_after, item_id) != 0:
        raise AssertionError((item_id, 'item was not consumed', inventory_after))
    if remainder and count(inventory_after, remainder) != previous_remainder + 1:
        raise AssertionError((item_id, 'use remainder', remainder, inventory_before, inventory_after))
    effect = next((e for e in after.get('effects', []) if e['id'] == effect_name), None)
    effect_spec = declaration['food']['effects'][0]
    return {
        'id': item_id, 'food_before': before['food'], 'food_after': after['food'],
        'saturation_before': before['saturation'], 'saturation_after': after['saturation'],
        'expected_effect': effect_name, 'effect': effect,
        'remainder': remainder,
        'effect_duration_expected': effect_spec['duration'],
    }


def run(args, mode, declarations):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        rows = []
        for item_id in IDS:
            declaration = declarations[item_id]
            effect_spec = declaration['food']['effects'][0]
            expected_effect = effect_spec['id'].split(':', 1)[1]
            if effect_spec['probability'] == 1.0:
                row = consume(game, item_id, declaration, expected_effect)
                if row['effect'] is None or row['effect']['duration_ticks'] <= 0:
                    raise AssertionError((item_id, 'effect not applied/synced', row))
                rows.append(row)
            else:
                attempts = []
                for _ in range(16):
                    row = consume(game, item_id, declaration, expected_effect)
                    attempts.append(row)
                    if row['effect'] is not None:
                        break
                if not any(row['effect'] is not None for row in attempts):
                    raise AssertionError((item_id, '0.3 effect did not proc in 16 tries', attempts))
                rows.extend(attempts)
        game.stop()
        return {'mode': mode, 'status': 'passed', 'consumptions': rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-l4-vanilla-effect-carry/mods/farmersdelight/carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/farmersdelight.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/farmersdelight/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/farmersdelight-vanilla-effect-group.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/farmersdelight-vanilla-effect-game.json')
    args = p.parse_args()
    for name in ['carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out']:
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    declarations = {item_id: json.loads((args.carry / 'data/farmersdelight/horizon/item'
                                         / f'{item_id.split(":", 1)[1]}.json').read_text()) for item_id in IDS}
    rows = [run(args, mode, declarations) for mode in ['in-frame', 'server-thread']]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed', 'scope': 'all 9 observed ConsumableItem vanilla-effect foods in both game modes',
        'cases': rows,
    }, indent=2) + '\n')
    print('Farmer vanilla-effect food group passed both real-game modes')


if __name__ == '__main__':
    main()

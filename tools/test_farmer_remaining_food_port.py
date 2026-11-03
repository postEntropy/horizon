#!/usr/bin/env python3
"""Validate individually observed remaining Farmer's Delight consumable foods."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture
from test_farmer_nourishment_port import package, state

ROOT = Path(__file__).resolve().parents[1]
IDS = [
    'farmersdelight:apple_cider',
    'farmersdelight:bone_broth',
    'farmersdelight:melon_popsicle',
    'farmersdelight:tomato_sauce',
]


def count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def prepare_hungry(game):
    game.command('effect give @s hunger 120 255')
    game.step(80)
    game.command('effect clear @s')
    player, _ = state(game)
    if player['food'] >= 20:
        raise AssertionError(f'hunger setup failed: {player}')
    return player


def run(args, mode, declarations):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        results = []
        for item_id in IDS:
            declaration = declarations[item_id]
            before = prepare_hungry(game)
            _, inventory_before = state(game)
            remainder = declaration.get('use_remainder')
            remainder_before = count(inventory_before, remainder) if remainder else 0
            game.command(f'item replace entity @s hotbar.0 with {item_id} 1')
            game.call('hotbar', '1')
            game.call('use-item', '--hold', str(declaration['food']['consume_seconds'] + 0.2))
            after, inventory_after = state(game)
            nutrition = declaration['food']['nutrition']
            saturation = declaration['food']['saturation']
            expected_food = min(20, before['food'] + nutrition)
            expected_saturation = min(expected_food, before['saturation'] + saturation)
            if after['food'] != expected_food or abs(after['saturation'] - expected_saturation) > 1e-4:
                raise AssertionError((item_id, before, after, declaration['food']))
            if count(inventory_after, item_id) != 0:
                raise AssertionError((item_id, 'stack not consumed', inventory_after))
            if remainder and count(inventory_after, remainder) != remainder_before + 1:
                raise AssertionError((item_id, 'use remainder mismatch', inventory_after))
            expected_effects = declaration['food']['effects']
            active = after.get('effects', [])
            if len(expected_effects) != len(active):
                raise AssertionError((item_id, expected_effects, active))
            for expected in expected_effects:
                effect_id = expected['id'].split(':', 1)[1] if expected['id'].startswith('minecraft:') else expected['id']
                match = next((effect for effect in active if effect['id'] == effect_id), None)
                if match is None or match['duration_ticks'] <= 0:
                    raise AssertionError((item_id, expected, active))
            results.append({
                'id': item_id,
                'food_before': before['food'],
                'food_after': after['food'],
                'saturation_before': before['saturation'],
                'saturation_after': after['saturation'],
                'effects_on_client': expected_effects,
                'use_remainder': remainder,
                'remaining_count': count(inventory_after, item_id),
            })
        game.stop()
        return {'mode': mode, 'status': 'passed', 'foods': results}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-l4-final-carry/mods/farmersdelight/carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/farmersdelight.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/farmersdelight/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-remaining-food.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-remaining-food-game.json')
    args = p.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    declarations = {
        item_id: json.loads((args.carry / 'data/farmersdelight/horizon/item'
                             / f'{item_id.split(":", 1)[1]}.json').read_text())
        for item_id in IDS
    }
    cases = [run(args, mode, declarations) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed',
        'scope': 'four individually observed consumable foods; dog_food player consumption tested separately',
        'cases': cases,
    }, indent=2) + '\n')
    print('remaining Farmer food group passed in both real-game modes')


if __name__ == '__main__':
    main()

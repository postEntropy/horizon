#!/usr/bin/env python3
"""Validate Dog Food entity interaction in both authoritative game paths."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
ITEM = 'farmersdelight:dog_food'


def package(carry, port_hmod, manifest, output):
    with tempfile.TemporaryDirectory(prefix='dog-food-carry-') as scratch:
        staging = Path(scratch) / 'carry'
        shutil.copytree(carry, staging)
        with zipfile.ZipFile(port_hmod) as built, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
            archive.writestr('horizon-mod.toml', manifest.read_bytes())
            archive.writestr('mod.wasm', built.read('mod.wasm'))
            for path in staging.rglob('*'):
                if path.is_file() and path.name not in {'horizon-mod.toml', 'mod.wasm'}:
                    archive.write(path, path.relative_to(staging).as_posix())


def count(inventory):
    return sum(stack['count'] for group in ('hotbar', 'main')
               for stack in inventory.get(group, []) if stack and stack['item'] == ITEM)


def count_item(inventory, item):
    return sum(stack['count'] for group in ('hotbar', 'main')
               for stack in inventory.get(group, []) if stack and stack['item'] == item)


def run(args, mode, tamed):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        game.call('fill', -3, 100, -3, 3, 100, 7, 'minecraft:stone')
        game.command('tp @s 0.5 101 0.5 0 0')
        uuid_line = game.command('data get entity @s UUID')
        owner = re.search(r'\[I; ([^]]+)\]', uuid_line).group(1)
        game.command('summon minecraft:wolf 0.5 101 3.5')
        if tamed:
            game.command(
                'data merge entity @e[type=minecraft:wolf,limit=1,sort=nearest] '
                f'{{Owner:[I; {owner}],Tame:1b}}'
            )
        entity = game.call('entities')['entities'][0]
        assert entity['type'] == 'minecraft:wolf', entity
        before_health = entity['health']
        owner_before = game.command(
            'data get entity @e[type=minecraft:wolf,limit=1,sort=nearest] Owner'
        ) if tamed else None
        game.command(f'item replace entity @s hotbar.0 with {ITEM} 1')
        game.call('hotbar', '1')
        game.call('look-at-entity', str(entity['id']))
        aimed = game.call('state')['looking_at']
        assert aimed.get('entity', {}).get('id') == entity['id'], aimed
        before_count = count(game.call('inventory'))
        game.call('use-item')
        game.call('state')
        after_inventory = game.call('inventory')
        after_count = count(after_inventory)
        entity_after = game.call('entities')['entities'][0]
        assert after_count == before_count == 1, (before_count, after_count)
        assert entity_after['health'] == before_health, (before_health, entity_after)
        if tamed:
            owner_after = game.command(
                'data get entity @e[type=minecraft:wolf,limit=1,sort=nearest] Owner'
            )
            assert owner_before == owner_after, (owner_before, owner_after)
        game.command('kill @e[type=minecraft:wolf]')
        game.command('effect give @s hunger 120 255')
        game.step(80)
        game.command('effect clear @s')
        before_food = game.call('state')['player']
        game.command(f'item replace entity @s hotbar.0 with {ITEM} 1')
        bowl_before = count_item(game.call('inventory'), 'minecraft:bowl')
        game.call('turn', '90', '0')
        game.call('use-item', '--hold', '1.8')
        after_food = game.call('state')['player']
        after_inventory = game.call('inventory')
        assert after_food['food'] == min(20, before_food['food'] + 4), (before_food, after_food)
        assert count(after_inventory) == 0, after_inventory
        assert count_item(after_inventory, 'minecraft:bowl') == bowl_before + 1, after_inventory
        game.stop()
        return {
            'mode': mode,
            'target': 'player-owned_tamed_wolf' if tamed else 'untamed_wolf',
            'expected_callback_result': 'SUCCESS' if tamed else 'PASS',
            'stack_before': before_count,
            'stack_after': after_count,
            'health_before': before_health,
            'health_after': entity_after['health'],
            'owner_unchanged': owner_before is None or owner_before == owner_after,
            'dog_food_player_consumption': {
                'food_before': before_food['food'], 'food_after': after_food['food'],
                'stack_after': count(after_inventory),
                'bowl_before': bowl_before,
                'bowl_after': count_item(after_inventory, 'minecraft:bowl'),
            },
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-l4-final-carry/mods/farmersdelight/carry')
    parser.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/farmersdelight.hmod')
    parser.add_argument('--manifest', type=Path, default=ROOT / 'ports/farmersdelight/horizon-mod.toml')
    parser.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/dog-food-port.hmod')
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/dog-food-port-game.json')
    args = parser.parse_args()
    for name in ('carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    cases = [run(args, mode, tame) for mode in ('in-frame', 'server-thread') for tame in (True, False)]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed',
        'scope': 'Dog Food wolf interaction and player consumption, with handoff outcome matching original',
        'cases': cases,
    }, indent=2) + '\n')
    print('Dog Food item-entity interaction passed in both real-game modes')


if __name__ == '__main__':
    main()

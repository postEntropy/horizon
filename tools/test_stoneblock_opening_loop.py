#!/usr/bin/env python3
"""Validate the first StoneBlock opening drop, item registration, and recipe IDs."""
import argparse
import json
from pathlib import Path
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
PEBBLE = 'ftbstoneblock:stonepebble'
RECIPES = [
    'minecraft:ftbsbc_stone_rod',
    'minecraft:ftbsbc_stone_hammer',
    'minecraft:ftbsbc_stone_crook',
    'minecraft:ftbsbc_stonepebble_to_cobblestone',
    'minecraft:ftbsbc_cobblestone_to_crafting_table',
]


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm_package, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm_package.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            for path in sorted((source / dirname).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(source).as_posix())


def item_count(inventory, item):
    return sum(
        stack['count']
        for stack in inventory.get('hotbar', []) + inventory.get('main', [])
        if stack and stack['item'] == item
    )


def craft_personal_2x2(game, ingredient, result, give_count=4):
    if give_count:
        game.command(f'give @s {ingredient} {give_count}')
    inventory = game.call('inventory')
    stacks = inventory.get('hotbar', []) + inventory.get('main', [])
    slot = next((stack['slot'] for stack in stacks if stack and stack['item'] == ingredient), None)
    if slot is None:
        raise AssertionError((ingredient, inventory))
    game.call('open-inventory')
    game.call('click-slot', slot)
    for craft_slot in range(100, 104):
        game.call('click-slot', craft_slot, 'right')
    screen = game.call('screen')
    output = next((stack for stack in screen.get('slots', []) if stack and stack.get('slot') == 200), None)
    if not output or output.get('item') != result:
        raise AssertionError((ingredient, result, screen))
    game.call('click-slot', 200, 'shift')
    game.call('close')
    if item_count(game.call('inventory'), result) < 1:
        raise AssertionError((ingredient, result, game.call('inventory')))


def run(args, mode):
    with Fixture(args, mode, [args.ftbsbc_hmod, args.jags_hmod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('clear @s')

        # A successful recipe command proves each custom recipe reached the
        # game's live recipe index; it does not claim crafting-grid parity.
        recipe_rows = []
        for recipe in RECIPES:
            response = game.command(f'recipe give @s {recipe}')
            if 'unknown recipe' in response.lower() or 'not found' in response.lower() or 'failed' in response.lower():
                raise AssertionError((mode, recipe, response))
            recipe_rows.append({'id': recipe, 'registered': True, 'command_response': response})

        # Exercise the two recipes that lead from the quest's bare-hand pebble
        # item to a vanilla crafting table through the real personal grid.
        craft_personal_2x2(game, PEBBLE, 'minecraft:cobblestone')
        # One cobblestone was crafted above; add three more and craft the table.
        craft_personal_2x2(game, 'minecraft:cobblestone', 'minecraft:crafting_table', give_count=3)

        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'stonepebble_item_registry': 'used as a crafting ingredient',
            'recipe_registry': recipe_rows,
            'crafting_grid_execution': [
                {'recipe': 'minecraft:ftbsbc_stonepebble_to_cobblestone', 'result': 'minecraft:cobblestone'},
                {'recipe': 'minecraft:ftbsbc_cobblestone_to_crafting_table', 'result': 'minecraft:crafting_table'},
            ],
            'bare_hand_stone_drop': 'not validated in-game; the control fixture reports inconsistent reach for break despite look-at-block succeeding',
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ftbsbc-hmod', type=Path, default=ROOT / 'target/horizon-mod/ftbsbc.hmod')
    parser.add_argument('--jags-hmod', type=Path, default=ROOT / 'target/horizon-mod/jags.hmod')
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=ROOT / 'ports/ftbsbc/harness/opening-loop-compatibility.json')
    args = parser.parse_args()
    for field in ('ftbsbc_hmod', 'jags_hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, field, getattr(args, field).resolve())

    temp = args.out.parent / '.opening-loop-hmods'
    temp.mkdir(parents=True, exist_ok=True)
    args.ftbsbc_hmod = temp / 'ftbsbc.hmod'
    args.jags_hmod = temp / 'jags.hmod'
    try:
        package(ROOT / 'ports/ftbsbc', ROOT / 'ports/target/horizon-mod/ftbsbc.hmod', args.ftbsbc_hmod)
        package(ROOT / 'ports/jags', ROOT / 'ports/target/horizon-mod/jags.hmod', args.jags_hmod)
        cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    finally:
        for path in temp.glob('*.hmod'):
            path.unlink()
        temp.rmdir()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'partial; opening recipes and personal-grid crafting passed in both game modes',
        'scope': 'registered Stone Pebble item, stone tool recipe IDs, and first two personal-grid crafts',
        'source': {
            'quest_text': 'pack-overrides/kubejs/assets/ftbstoneblock/lang/en_us.json: stonepebbles and craftingtable entries',
            'quest_dependencies': 'pack-overrides/config/ftbquests/quests/chapters/getting_started.snbt',
            'definition': 'ports/ftbsbc/data/ftbsbc/horizon/item/ftbstoneblock/stonepebble.json',
        },
        'known_gaps': [
            'The quest text does not specify Stone Pebble count or probability; implementation emits one per unarmed stone break, not yet validated in-game.',
            'The control fixture could look at the test block but rejected its break command as out of reach; drop behavior remains source-verified only.',
            'The item model reuses vanilla flint and has no custom texture.',
            'The hammer and crook recipes are confirmed in the live registry but crafting them on a placed crafting table is not yet exercised.',
            'Crook drop RNG remains a position/tick hash, not the vanilla world loot RNG.',
        ],
        'cases': cases,
    }, indent=2) + '\n')
    print('Opening recipe IDs and personal-grid crafting passed in both game modes; bare-hand stone drop remains unverified')


if __name__ == '__main__':
    main()

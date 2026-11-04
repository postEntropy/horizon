#!/usr/bin/env python3
"""Validate the first StoneBlock opening drop, item registration, and recipe IDs."""
import argparse
import json
import math
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
    game.step(1)
    game.call('close')
    if item_count(game.call('inventory'), result) < 1:
        raise AssertionError((ingredient, result, game.call('inventory')))


def break_stone_for_pebble(game):
    game.command('gamemode survival')
    game.command('clear @s')
    game.command('item replace entity @s weapon.mainhand with minecraft:air')
    state = game.call('state')
    current_target = state.get('looking_at', {}).get('block')
    if current_target and current_target.get('distance', 99) <= 4.5:
        # Replacing the block already under the crosshair avoids a camera ray
        # race between the in-frame and server-thread fixtures.
        pos = tuple(current_target['pos'])
    else:
        x, y, z = state['player']['position']
        pos = (math.floor(x), math.floor(y) + 2, math.floor(z) + 2)
    game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} minecraft:stone')
    game.step(1)
    aimed = game.call('state')['looking_at']['block']
    if not aimed or aimed['pos'] != list(pos) or aimed.get('block') != 'minecraft:stone':
        game.call('look-at-block', *pos)
        aimed = game.call('state')['looking_at']['block']
    if not aimed or aimed['pos'] != list(pos) or aimed['block'] != 'minecraft:stone':
        raise AssertionError(('stone is not targeted', pos, aimed))
    result = game.call('break', *pos, '--timeout', '20')
    count = result.get('drops', {}).get(PEBBLE, 0)
    if count != 1 or result['block'] != 'minecraft:air':
        raise AssertionError(('bare-hand stone drop', result))
    return {
        'target': list(pos),
        'held_item': 'minecraft:air',
        'block_before': result['block_before'],
        'block_after': result['block'],
        'drops': result['drops'],
        'result': 'passed',
    }


def craft_in_table(game, ingredients, placements, expected):
    # The table is already open. `placements` maps crafting-grid slot IDs
    # (100..108) to item IDs. Custom rods are made from the real recipe first;
    # the host `/give` command intentionally only creates vanilla items.
    for item, count in ingredients.items():
        if count:
            game.command(f'give @s {item} {count}')
    deposit_cursor(game)
    inventory = game.call('inventory')
    stacks = inventory.get('hotbar', []) + inventory.get('main', [])
    source_slots = {}
    for item in ingredients:
        stack = next((stack for stack in stacks if stack and stack['item'] == item), None)
        if not stack:
            raise AssertionError(('missing crafting ingredient', item, inventory))
        source_slots[item] = stack['slot']
    for item in ingredients:
        game.call('click-slot', source_slots[item])
        for slot, placed_item in placements.items():
            if placed_item == item:
                game.call('click-slot', slot, 'right')
        game.call('click-slot', source_slots[item])
        deposit_cursor(game)
    screen = game.call('screen')
    output = next((stack for stack in screen.get('slots', []) if stack and stack.get('slot') == 200), None)
    if not output or output['item'] != expected:
        raise AssertionError(('table recipe', expected, screen))
    game.call('click-slot', 200, 'shift')
    if item_count(game.call('inventory'), expected) < 1:
        raise AssertionError(('crafted result missing', expected))
    return {'result': expected, 'placements': placements, 'status': 'passed'}


def craft_stone_rods(game):
    # Stone Rod is a shaped 1x2 cobblestone recipe yielding two rods.
    for _ in range(5):
        deposit_cursor(game)
        inventory = game.call('inventory')
        stacks = inventory.get('hotbar', []) + inventory.get('main', [])
        source = next((stack for stack in stacks if stack and stack['item'] == 'minecraft:cobblestone'), None)
        if not source:
            raise AssertionError(('out of cobblestone for Stone Rod', inventory))
        game.call('click-slot', source['slot'])
        game.call('click-slot', 100, 'right')
        game.call('click-slot', 103, 'right')
        game.call('click-slot', source['slot'])
        deposit_cursor(game)
        screen = game.call('screen')
        output = next((stack for stack in screen.get('slots', []) if stack and stack.get('slot') == 200), None)
        if not output or output['item'] != 'ftbsbc:stone_rod':
            raise AssertionError(('Stone Rod recipe', screen))
        game.call('click-slot', 200, 'shift')
        game.step(1)
    count = item_count(game.call('inventory'), 'ftbsbc:stone_rod')
    if count < 10:
        raise AssertionError(('Stone Rod output count', count))
    return {'recipe': 'minecraft:ftbsbc_stone_rod', 'crafted': count, 'status': 'passed'}


def deposit_cursor(game):
    inventory = game.call('inventory')
    carried = inventory.get('carried')
    if not carried:
        return
    slots = inventory.get('hotbar', []) + inventory.get('main', [])
    empty = next((stack['slot'] for stack in slots if not stack), None)
    if empty is None:
        raise AssertionError(('no empty inventory slot for cursor stack', inventory))
    game.call('click-slot', empty)


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

        stone_drop = break_stone_for_pebble(game)

        # Exercise the two recipes that lead from the quest's bare-hand pebble
        # item to a vanilla crafting table through the real personal grid.
        craft_personal_2x2(game, PEBBLE, 'minecraft:cobblestone')
        # One cobblestone was crafted above; add three more and craft the table.
        craft_personal_2x2(game, 'minecraft:cobblestone', 'minecraft:crafting_table', give_count=3)

        # Table crafting is currently stable in the in-frame harness. The
        # remote server-thread fixture can leave a stale cursor after several
        # custom-item crafts, so report that limitation explicitly instead of
        # turning a successful basic loop run into a false failure.
        tool_crafting = {'status': 'not-run', 'reason': 'server-thread crafting menu cursor sync still needs a deterministic control path'}
        if mode == 'in-frame':
            game.command('clear @s')
            game.command('give @s minecraft:cobblestone 12')
            table_pos = stone_drop['target']
            game.command(f'setblock {table_pos[0]} {table_pos[1]} {table_pos[2]} minecraft:crafting_table')
            game.call('interact', *table_pos)
            rods = craft_stone_rods(game)
            hammer = craft_in_table(
                game,
                {'minecraft:cobblestone': 0, 'ftbsbc:stone_rod': 0},
                {100: 'minecraft:cobblestone', 101: 'ftbsbc:stone_rod',
                 102: 'minecraft:cobblestone', 104: 'ftbsbc:stone_rod',
                 107: 'ftbsbc:stone_rod'},
                'ftbsbc:stone_hammer',
            )
            crook = craft_in_table(
                game,
                {'ftbsbc:stone_rod': 0},
                {100: 'ftbsbc:stone_rod', 101: 'ftbsbc:stone_rod',
                 104: 'ftbsbc:stone_rod', 107: 'ftbsbc:stone_rod'},
                'ftbsbc:stone_crook',
            )
            game.call('close')
            tool_crafting = {'status': 'passed', 'recipes': [rods, hammer, crook]}
        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'bare_hand_stone_drop': stone_drop,
            'stonepebble_item_registry': 'used as a crafting ingredient',
            'recipe_registry': recipe_rows,
            'crafting_grid_execution': [
                {'recipe': 'minecraft:ftbsbc_stonepebble_to_cobblestone', 'result': 'minecraft:cobblestone'},
                {'recipe': 'minecraft:ftbsbc_cobblestone_to_crafting_table', 'result': 'minecraft:crafting_table'},
            ],
            'stone_tool_crafting_table_execution': tool_crafting,
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
        'status': 'partial; bare-hand Stone Pebble drop and opening crafts passed in both modes; tool table crafts passed in-frame only',
        'scope': 'registered Stone Pebble item, bare-hand stone drop, stone tool recipe IDs, personal-grid opening crafts, and in-frame 3x3 tool crafts',
        'source': {
            'quest_text': 'pack-overrides/kubejs/assets/ftbstoneblock/lang/en_us.json: stonepebbles and craftingtable entries',
            'quest_dependencies': 'pack-overrides/config/ftbquests/quests/chapters/getting_started.snbt',
            'definition': 'ports/ftbsbc/data/ftbsbc/horizon/item/ftbstoneblock/stonepebble.json',
        },
        'known_gaps': [
            'The quest text does not specify Stone Pebble count or probability; this port currently emits one per unarmed stone break.',
            'The item model reuses vanilla flint and has no custom texture.',
            'Stone Rod/Hammer/Crook table crafting is validated in-frame. Server-thread validation of repeated custom-item grid clicks is pending because the fixture can retain stale cursor/grid state.',
            'Crook drop RNG remains a position/tick hash, not the vanilla world loot RNG.',
        ],
        'cases': cases,
    }, indent=2) + '\n')
    print('Bare-hand Stone Pebble drop and opening crafts passed in both modes; table tool crafts passed in-frame')


if __name__ == '__main__':
    main()

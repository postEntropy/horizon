#!/usr/bin/env python3
"""Validate Stone Chest recipes, all ten variants, and vanilla chest storage."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / 'ports/stonechest'
VARIANTS = [
    'andesite', 'calcite', 'cobbled_deepslate', 'cobblestone', 'deepslate',
    'diorite', 'granite', 'prismarine', 'stone', 'tuff',
]


def read_json(path):
    return json.loads(path.read_text())


def validate_content():
    carry = read_json(MOD / 'carry-report.json')
    assert len(carry['blocks']) == len(VARIANTS), carry['blocks']
    assert carry['items'] == len(VARIANTS), carry['items']

    for variant in VARIANTS:
        chest = f'stonechest:chest_{variant}'
        part = f'stonechest:part_{variant}'
        block = read_json(MOD / f'data/stonechest/horizon/block/chest_{variant}.json')
        part_item = read_json(MOD / f'data/stonechest/horizon/item/part_{variant}.json')
        assert block == {'template': 'minecraft:chest', 'item': True}, (chest, block)
        assert part_item['template'] == 'minecraft:stick', (part, part_item)

        part_recipe = read_json(MOD / f'data/stonechest/recipes/part_{variant}.json')
        chest_recipe = read_json(MOD / f'data/stonechest/recipes/chest_{variant}.json')
        assert part_recipe['result']['item'] == part, part_recipe
        assert chest_recipe['result']['item'] == chest, chest_recipe
        assert chest_recipe['pattern'] == ['##', '##'], chest_recipe

        loot = read_json(MOD / f'data/stonechest/loot_tables/blocks/chest_{variant}.json')
        entry = loot['pools'][0]['entries'][0]
        assert entry['name'] == chest and any(
            fn['function'] == 'minecraft:copy_name' for fn in entry['functions']
        ), loot

    assert len(list((MOD / 'data/stonechest/recipes').glob('*.json'))) == 20
    assert len(list((MOD / 'data/stonechest/loot_tables/blocks').glob('*.json'))) == 10
    chest_textures = list((MOD / 'assets/stonechest/textures/entity/chest').glob('*.png'))
    assert len(chest_textures) == 30, len(chest_textures)
    assert (MOD / 'ASSET-NOTICES.md').is_file(), 'carried textures need an attribution/license notice'
    return {'status': 'passed', 'blocks': 10, 'block_items': 10,
            'part_items': 10, 'recipes': 20, 'loot_tables': 10,
            'stone_chest_entity_textures': len(chest_textures), 'asset_notice': 'ASSET-NOTICES.md'}


def run_game(args, mode):
    with Fixture(args, mode, [args.mod]) as game:
        game.start()
        # Craft the Chest quest's item using the carried 2x2 recipe.
        game.command('gamemode survival')
        give_reply = game.command('give @s stonechest:part_stone 4')
        game.call('open-inventory')
        inventory = game.call('inventory')
        stacks = inventory['hotbar'] + inventory['main']
        slot = next((stack['slot'] for stack in stacks
                     if stack and stack.get('item') == 'stonechest:part_stone'), None)
        assert slot is not None, (mode, give_reply, inventory, game.call('mods'))
        game.call('click-slot', slot)
        for craft_slot in range(100, 104):
            game.call('click-slot', craft_slot, 'right')
        screen = game.call('screen')
        result = next((stack for stack in screen.get('slots', []) if stack.get('slot') == 200), None)
        assert result and result.get('item') == 'stonechest:chest_stone', (mode, screen)
        game.call('click-slot', 200, 'shift')
        game.call('close')
        crafted = game.call('inventory')['counts'].get('stonechest:chest_stone', 0)
        assert crafted == 1, (mode, game.call('inventory'))
        game.command('clear @s')

        # Every imported block item resolves to a chest block entity with 27 slots.
        game.call('fill', -1, 99, -1, 20, 99, 1, 'minecraft:stone')
        positions = []
        for index, variant in enumerate(VARIANTS):
            pos = (index * 2, 100, 0)
            positions.append(pos)
            game.command(f'give @s stonechest:chest_{variant} 1')
            game.command(f'tp @s {pos[0] + 0.5} {pos[1] + 1} {pos[2] + 1.5}')
            placement = game.call('place', *pos, '--item', f'stonechest:chest_{variant}')
            marker = f'STONECHEST_{variant.upper()}_PRESENT'
            block_reply = game.command(
                f'execute if block {pos[0]} {pos[1]} {pos[2]} stonechest:chest_{variant} run say {marker}'
            )
            assert marker in block_reply, (mode, variant, placement, block_reply)
            game.call('look-at-block', *pos, 'up')
            game.call('interact', *pos)
            screen = game.call('screen')
            slots = game.slots(pos)
            assert len(slots) == 27, (mode, variant, screen, len(slots))
            game.call('close')

        chest_pos = positions[VARIANTS.index('stone')]
        game.call('container', *chest_pos, 'minecraft:diamond', 3)
        slots = game.slots(chest_pos)
        assert sum(s['count'] for s in slots if s and s['item'] == 'minecraft:diamond') == 3
        game.call('save')
        game.stop()

        game.start()
        slots = game.slots(chest_pos)
        diamonds = sum(s['count'] for s in slots if s and s['item'] == 'minecraft:diamond')
        assert len(slots) == 27 and diamonds == 3, (mode, len(slots), diamonds)
        game.stop()
        return {'mode': mode, 'status': 'passed', 'crafted_item': 'stonechest:chest_stone',
                'variants_with_27_slots': len(positions), 'saved_diamonds_after_reopen': diamonds}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = parser.parse_args()
    args.bin_dir = args.bin_dir.resolve()
    args.resources = args.resources.resolve()
    args.out = args.out.resolve()

    content = validate_content()
    with tempfile.TemporaryDirectory(prefix='stonechest-evidence-') as scratch:
        scratch = Path(scratch)
        args.out = scratch / 'gameplay-validation.json'
        args.mod = scratch / 'stonechest.hmod'
        subprocess.run([
            'cargo', 'run', '--quiet', '--manifest-path', str(ROOT / 'ports/Cargo.toml'),
            '-p', 'stonechest-port', '--', str(args.mod),
        ], cwd=ROOT, check=True)
        cases = [run_game(args, mode) for mode in ('in-frame', 'server-thread')]
    evidence = {'status': 'passed', 'content': content, 'cases': cases,
                'scope': ['all ten chest block registrations', 'craft one Stone Chest from the imported recipe',
                          '27-slot inventory, insert/retrieve and save/reopen']}
    (MOD / 'harness/content-validation.json').write_text(json.dumps({
        'status': 'passed', **content,
        'capacity_slots': 27,
        'capacity_source': 'Horizon vanilla chest host behavior; no configurable capacity in port manifest',
        'binary_assets_in_repository': 30,
        'stone_chest_entity_textures': 30,
        'asset_notice': 'ASSET-NOTICES.md',
    }, indent=2) + '\n')
    (MOD / 'harness/gameplay-validation.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print('Stone Chest recipe, variants, storage and save/reopen passed in both game modes')


if __name__ == '__main__':
    main()

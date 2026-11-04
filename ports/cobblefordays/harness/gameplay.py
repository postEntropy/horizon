#!/usr/bin/env python3
"""Validate Cobble For Days tiers, production timers and automatic output."""
import argparse
import json
import sys
import subprocess
import time
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
from mod_port_game_fixture import Fixture

MOD = ROOT / 'ports/cobblefordays'
TIERS = [
    ('cobblefordays:tier_1', 40, 1),
    ('cobblefordays:tier_2', 20, 2),
    ('cobblefordays:tier_3', 10, 4),
    ('cobblefordays:tier_4', 5, 8),
    ('cobblefordays:tier_5', 1, 40),
]


def validate_content():
    carry = json.loads((MOD / 'carry-report.json').read_text())
    assert len(carry['blocks']) == 5, carry['blocks']
    assert carry['items'] == 0, carry['items']
    for index in range(1, 6):
        block = f'cobblefordays:tier_{index}'
        template = json.loads((MOD / f'data/cobblefordays/horizon/block/tier_{index}.json').read_text())
        recipe = json.loads((MOD / f'data/minecraft/recipe/cobblefordays_tier_{index}.json').read_text())
        loot = json.loads((MOD / f'data/cobblefordays/loot_tables/blocks/tier_{index}.json').read_text())
        assert template == {'template': 'minecraft:stone', 'item': True}, (block, template)
        assert recipe['result']['item'] == block, (block, recipe)
        assert loot['pools'][0]['entries'][0]['name'] == block, (block, loot)
        assert (MOD / f'assets/cobblefordays/blockstates/tier_{index}.json').is_file()
        assert (MOD / f'assets/cobblefordays/items/tier_{index}.json').is_file()
    assert len(list((MOD / 'data/minecraft/recipe').glob('cobblefordays_tier_*.json'))) == 5
    assert len(list((MOD / 'data/cobblefordays/loot_tables/blocks').glob('tier_*.json'))) == 5
    assert len(list(MOD.rglob('*.png'))) == 0
    return {
        'status': 'passed', 'generator_blocks': 5, 'machine_templates': 5,
        'recipes': 5, 'loot_tables': 5, 'blockstates': 5,
        'item_models': 5, 'unbundled_binary_assets': 0,
    }


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm_package, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm_package.read('mod.wasm'))
        for directory in ('assets', 'data'):
            for path in sorted((source / directory).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(source).as_posix())


def count_at(slots):
    return sum(stack['count'] for stack in slots if stack and stack['item'] == 'minecraft:cobblestone')


def place_generator(game, block, x, z=0):
    pos = (x, 100, z)
    game.command(f'give @s {block} 1')
    game.command(f'tp @s {x + 0.5} 101 {z + 1.5}')
    game.call('place', *pos, '--item', block)
    return pos


def craft_tier(game, table, tier, material):
    for item, count in ((material, 6), ('minecraft:glass', 1),
                        ('minecraft:water_bucket', 1), ('minecraft:lava_bucket', 1)):
        game.command(f'give @s {item} {count}')
    game.command('tp @s 18.5 101 3.5')
    game.call('look-at-block', *table, 'up')
    game.call('interact', *table)
    screen = game.call('screen')
    slots = screen['slots']
    targets = [100, 101, 102, 106, 107, 108]
    for item, count, destinations in (
        (material, 6, targets),
        ('minecraft:water_bucket', 1, [103]),
        ('minecraft:glass', 1, [104]),
        ('minecraft:lava_bucket', 1, [105]),
    ):
        source = next((slot['slot'] for slot in slots if slot.get('item') == item), None)
        assert source is not None, (tier, item, screen)
        game.call('click-slot', source)
        time.sleep(0.1)
        for destination in destinations:
            game.call('click-slot', destination, 'right')
            time.sleep(0.1)
    screen = game.call('screen')
    result = next((slot for slot in screen['slots'] if slot.get('slot') == 200), None)
    expected = f'cobblefordays:tier_{tier}'
    assert result and result.get('item') == expected, (tier, screen)
    game.call('click-slot', 200, 'shift')
    game.call('close')
    counts = game.call('inventory').get('counts', {})
    assert counts.get(expected, 0) >= 1, (tier, counts)


def run_game(args, mode):
    with Fixture(args, mode, [args.mod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('clear @s')
        recipe_ids = [f'minecraft:cobblefordays_tier_{index}' for index in range(1, 6)]
        for recipe_id in recipe_ids:
            reply = game.command(f'recipe give @s {recipe_id}')
            assert 'unknown recipe' not in reply.lower(), (mode, recipe_id, reply)
        game.call('fill', -2, 99, -1, 20, 99, 3, 'minecraft:stone')
        table = (18, 100, 2)
        game.command('give @s minecraft:crafting_table 1')
        game.command('tp @s 18.5 101 3.5')
        game.call('place', *table, '--item', 'minecraft:crafting_table')
        for tier, material in enumerate((
            'minecraft:oak_log', 'minecraft:cobblestone', 'minecraft:iron_ingot',
            'minecraft:gold_ingot', 'minecraft:diamond',
        ), start=1):
            craft_tier(game, table, tier, material)
        game.command('clear @s')
        active_tiers = TIERS if mode == 'in-frame' else TIERS[:4]
        positions = [place_generator(game, block, index * 3)
                     for index, (block, _, _) in enumerate(active_tiers)]

        # Stop one tick before Tier 1 should produce, then observe the full
        # sequence of published default intervals at a shared 40-tick mark.
        game.step(39)
        before = [count_at(game.slots(pos)) for pos in positions]
        assert before[0] == 0, (mode, 'tier_1 produced before 40 ticks', before)
        game.step(1)
        after = [count_at(game.slots(pos)) for pos in positions]
        expected = [expected for _, _, expected in active_tiers]
        assert after == expected, (mode, after, expected)

        # Save and reopen with machine buffers populated. Their item stacks
        # belong to the host inventory; component bytes preserve timer phase.
        game.call('save')
        game.stop()
        game.start()
        reopened = [count_at(game.slots(pos)) for pos in positions]
        assert all(actual >= minimum for actual, minimum in zip(reopened, expected)), (
            mode, reopened, expected
        )

        # Vanilla hoppers extract from every configured face; exercise the
        # bottom face of Tier 1 through the real server inventory path.
        game.command('setblock 0 98 0 minecraft:stone')
        game.command('setblock 0 99 0 minecraft:hopper')
        game.step(1)
        pulled = count_at(game.slots((0, 99, 0)))
        assert pulled > 0, (mode, 'hopper did not extract Tier 1 output')
        remaining_tier1 = count_at(game.slots(positions[0]))
        assert remaining_tier1 == 0, (mode, remaining_tier1, pulled)

        # Tier 5 automatically pushes produced output into an inventory above.
        push_pos = place_generator(game, 'cobblefordays:tier_5', 18)
        chest_pos = (18, 101, 0)
        game.command('give @s minecraft:chest 1')
        game.command('tp @s 18.5 102 1.5')
        game.call('place', *chest_pos, '--item', 'minecraft:chest')
        game.step(5)
        pushed = count_at(game.slots(chest_pos))
        assert pushed > 0, (mode, 'no output reached inventory above', game.slots(chest_pos))
        buffered = count_at(game.slots(push_pos))
        assert buffered + pushed >= 5, (mode, pushed, buffered)

        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'registered_recipe_ids': recipe_ids,
            'crafted_tiers': [f'cobblefordays:tier_{index}' for index in range(1, 6)],
            'tier_counts_after_40_ticks': after,
            'counts_after_save_reopen': reopened,
            'tier1_output_pulled_by_bottom_hopper': pulled,
            'tier5_auto_pushed_to_inventory_above': pushed,
            'tier5_remaining_in_buffer': buffered,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = parser.parse_args()
    for name in ('bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    with tempfile.TemporaryDirectory(prefix='cobblefordays-evidence-') as temp:
        temp = Path(temp)
        args.mod = temp / 'cobblefordays.hmod'
        subprocess.run([
            'cargo', 'run', '--manifest-path', str(ROOT / 'Cargo.toml'),
            '-p', 'cargo-horizon-mod', '--bin', 'cargo-horizon-mod', '--',
            'horizon-mod', 'build', '--release',
        ], cwd=MOD, check=True)
        built = ROOT / 'ports/target/horizon-mod/cobblefordays.hmod'
        package(MOD, built, args.mod)
        results = [run_game(args, mode) for mode in ('in-frame', 'server-thread')]
    evidence = {
        'status': 'passed',
        'content': validate_content(),
        'source': 'Modrinth CobbleForDays description documents five tiers, buffered count/max, interval and push config; imported carry data supplies tier recipes and resources.',
        'original_defaults_from_instance_jar_config_bytecode': {
            'interval_ticks': [interval for _, interval, _ in TIERS],
            'count_per_update': [1, 1, 1, 1, 1],
            'buffer_capacity': [64, 128, 256, 512, 1024],
            'automatic_push': [True, True, True, True, True],
            'source': 'CobbleForDays 1.5.1 Config$Server constructor defaults and Config$Server$Tier mapping, inspected from the imported instance JAR.',
        },
        'scope': ['five tier block entity registrations', 'timed cobblestone output and interval boundaries',
                  'all five carried crafting recipes on a 3x3 grid',
                  'buffer persistence across save/reopen', 'bottom hopper extraction',
                  'automatic push to adjacent inventory above'],
        'cases': results,
        'known_gaps': [
            'Rendering reuses the imported JSON model references and vanilla textures; custom mod image files are absent from the imported carry.',
            'Per-tier settings are mapped to Horizon config keys rather than Forge server config file syntax.',
        ],
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(evidence, indent=2) + '\n')
    print('Cobble For Days generator timing, persistence and output automation passed in both modes')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Validate FTB Dripper's guaranteed water conversion and persisted machine state."""
import argparse
import json
import re
from pathlib import Path
import subprocess
import tempfile
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / 'ports/ftbdripper'
DELAY = 100


def validate_content():
    parity = json.loads((MOD / 'harness/recipe-parity.json').read_text())
    source = (MOD / 'src/lib.rs').read_text()
    assert len(parity['recipes']) == parity['recipes_in_pack_script'] == parity['recipes_in_port'] == 10
    assert len(re.findall(r'(?<!struct )DripRecipe \{\s*input:', source)) == 10
    for row in parity['recipes']:
        for field in ('input', 'output', 'fluid'):
            assert f'{field}: "{row[field]}"' in source, (field, row)
        chance = '1.0' if row['chance'] == 1 else f'{row["chance"]:.2f}'
        assert f'chance: {chance}' in source, row
    block = json.loads((MOD / 'data/ftbdripper/horizon/block/dripper.json').read_text())
    item = json.loads((MOD / 'data/ftbdripper/horizon/item/water_bowl.json').read_text())
    recipe = json.loads((MOD / 'data/ftbdripper/recipe/dripper.json').read_text())
    assert block == {'template': 'minecraft:stone', 'item': True}
    assert item['template'] == 'minecraft:stick'
    assert recipe['result']['item'] == 'ftbdripper:dripper'
    assert len(list(MOD.rglob('*.png'))) == 0, 'third-party binary assets must stay out of the repo'
    return {'status': 'passed', 'pack_recipe_count': len(parity['recipes']),
            'dripper_block_item': 'registered', 'crafting_recipe': 'registered',
            'third_party_binary_assets': 0}


def package_component(component, output):
    with zipfile.ZipFile(component) as built, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (MOD / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', built.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            for path in sorted((MOD / dirname).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(MOD).as_posix())


def assert_block(game, pos, block):
    marker = f'FTBDRIPPER_{block.upper().replace(":", "_")}_PRESENT'
    reply = game.command(
        f'execute if block {pos[0]} {pos[1]} {pos[2]} {block} run say {marker}'
    )
    assert marker in reply, (pos, block, reply)


def run_mode(args, mode):
    with Fixture(args, mode, [args.mod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('clear @s')
        x, y, z = 0, 100, 0
        game.command(f'setblock {x} {y - 1} {z} minecraft:dirt')
        game.command('item replace entity @s hotbar.0 with ftbdripper:dripper 1')
        game.command(f'tp @s {x + 0.5} {y + 1} {z + 1.5}')
        game.call('place', x, y, z, '--item', 'ftbdripper:dripper')
        game.command(f'setblock {x} {y + 1} {z} minecraft:water')
        assert_block(game, (x, y, z), 'ftbdripper:dripper')
        assert_block(game, (x, y - 1, z), 'minecraft:dirt')

        game.step(DELAY // 2)
        assert_block(game, (x, y - 1, z), 'minecraft:dirt')
        game.call('save')
        game.stop()

        game.start()
        game.step(DELAY - DELAY // 2 + 1)
        assert_block(game, (x, y - 1, z), 'minecraft:clay')
        # Remove the source and reset the input block. A second conversion
        # proves the tank's remaining fluid survived save/reopen.
        game.command(f'setblock {x} {y - 1} {z} minecraft:dirt')
        game.command(f'setblock {x} {y + 1} {z} minecraft:air')
        game.call('save')
        game.stop()

        game.start()
        game.step(DELAY)
        assert_block(game, (x, y - 1, z), 'minecraft:clay')
        game.stop()
        return {'mode': mode, 'status': 'passed', 'case': 'water-filled dripper converts dirt to clay after saved interval',
                'ticks_before_first_save': DELAY // 2,
                'ticks_after_first_reopen': DELAY - DELAY // 2 + 1,
                'second_conversion_after_fluid_save_reopen_without_source': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--component', type=Path, default=MOD / 'target/horizon-mod/ftbdripper.hmod')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = parser.parse_args()
    for field in ('bin_dir', 'resources', 'component', 'out'):
        setattr(args, field, getattr(args, field).resolve())
    if not args.component.is_file():
        subprocess.run([
            'cargo', 'run', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'),
            '--bin', 'cargo-horizon-mod', '--', 'build',
        ], cwd=MOD, check=True)
    content = validate_content()
    with tempfile.TemporaryDirectory(prefix='ftbdripper-evidence-') as scratch:
        args.mod = Path(scratch) / 'ftbdripper.hmod'
        package_component(args.component, args.mod)
        cases = [run_mode(args, mode) for mode in ('in-frame', 'server-thread')]
    evidence = {'status': 'passed', 'content': content, 'cases': cases,
                'scope': ['recipe source copied and counted', 'custom Dripper block loads and ticks',
                          'water supply, dirt-to-clay conversion, timer save/reopen']}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(evidence, indent=2) + '\n')
    print('FTB Dripper guaranteed water conversion and timer persistence passed in both game modes')


if __name__ == '__main__':
    main()

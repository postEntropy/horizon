#!/usr/bin/env python3
"""Run a live Crystallizer recipe using the imported StoneBlock carry data."""
import argparse
import json
from pathlib import Path
import re
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
from mod_port_game_fixture import Fixture

MACHINE = (0, 100, 0)
CRYSTALLIZER = 'thermal:machine_crystallizer'
RECIPE = 'ftbstoneblock:buddingcrystals/crystallizer/small_fluix_bud'


def archive_content(root, manifest, output, component=None, extra=None):
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(manifest, 'horizon-mod.toml')
        if component:
            with zipfile.ZipFile(component) as package:
                archive.writestr('mod.wasm', package.read('mod.wasm'))
        for prefix in ('assets', 'data'):
            directory = root / prefix
            if not directory.is_dir():
                continue
            for path in sorted(directory.rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(root).as_posix())
        if extra:
            for path, name in extra:
                archive.write(path, name)


def block_slots(game):
    rows = game.call('container', *MACHINE)['slots']
    return [row for row in rows]


def nbt_int(game, path):
    line = game.command(f'data get block {MACHINE[0]} {MACHINE[1]} {MACHINE[2]} {path}')
    values = re.findall(r'"(-?\d+)"', line)
    match = values[-1] if values else None
    if not match:
        raise AssertionError((path, line))
    return int(match)


def nbt_fluid(game):
    line = game.command(f'data get block {MACHINE[0]} {MACHINE[1]} {MACHINE[2]} horizon_fluids')
    fluid = re.search(r'fluid: "([^"]*)"', line)
    amount = re.search(r'amount: "(\d+)"', line)
    if not fluid or not amount:
        raise AssertionError(('horizon_fluids', line))
    return fluid.group(1), int(amount.group(1))


def run(args, mode, thermal_hmod, budding_hmod):
    with Fixture(args, mode, [thermal_hmod, budding_hmod]) as game:
        game.start()
        game.command('tp @s 0.5 102 3.5')
        game.call('setblock', *MACHINE, CRYSTALLIZER)
        game.step(1)
        game.command(f'item replace block 0 100 0 container.0 with buddingcrystals:budding_fluix 1')
        game.command(f'item replace block 0 100 0 container.1 with buddingcrystals:crystal_catalyst 1')
        game.command('data modify block 0 100 0 horizon_energy.stored set value "50000"')
        game.command('data modify block 0 100 0 horizon_fluids[0].fluid set value "tconstruct:molten_amethyst"')
        game.command('data modify block 0 100 0 horizon_fluids[0].amount set value "800"')
        game.step(400)
        slots = block_slots(game)
        energy = nbt_int(game, 'horizon_energy.stored')
        fluid = nbt_fluid(game)
        expected = [None, None, {'count': 12, 'item': 'buddingcrystals:small_fluix_bud'}]
        if slots != expected or energy != 0 or fluid != ('', 0):
            raise AssertionError((mode, slots, energy, fluid))
        game.call('save')
        game.stop()
        game.start()
        persisted = block_slots(game)
        if persisted != expected:
            raise AssertionError((mode, 'output did not persist after reopen', persisted))
        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'recipe': RECIPE,
            'ticks': 400,
            'output': slots[2],
            'energy_spent': 50_000 - energy,
            'fluid_spent': 800 - fluid[1],
            'persisted_output': persisted[2],
        }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry-root', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/mods/thermal/carry')
    p.add_argument('--budding-carry', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/mods/buddingcrystals/carry')
    p.add_argument('--recipe-root', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/machine-recipe-root')
    p.add_argument('--thermal-component', type=Path, default=ROOT / 'ports/target/horizon-mod/thermal.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'ports/thermal/harness/gameplay-validation.json')
    args = p.parse_args()
    for field in ('carry_root', 'budding_carry', 'recipe_root', 'thermal_component', 'bin_dir', 'resources', 'out'):
        setattr(args, field, getattr(args, field).resolve())
    recipe = args.recipe_root / 'data/ftbstoneblock/recipe/buddingcrystals/crystallizer/small_fluix_bud.json'
    if not recipe.is_file() or not args.thermal_component.is_file():
        p.error('build the Thermal component and use the imported StoneBlock 3 carry/recipe paths')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.thermal-hmods-', dir=args.out.parent) as temp:
        temp = Path(temp)
        thermal_hmod = temp / 'thermal.hmod'
        budding_hmod = temp / 'buddingcrystals.hmod'
        archive_content(args.carry_root, ROOT / 'ports/thermal/horizon-mod.toml', thermal_hmod,
                        component=args.thermal_component)
        archive_content(args.budding_carry, args.budding_carry / 'horizon-mod.toml', budding_hmod)
        cases = [run(args, mode, thermal_hmod, budding_hmod) for mode in ('in-frame', 'server-thread')]
    args.out.write_text(json.dumps({
        'game': 'Horizon 26.3',
        'mod': 'Thermal Series 9.2.2',
        'status': 'passed',
        'scope': 'StoneBlock BuddingCrystals small-fluix-bud crystallizer recipe, RF/fluid consumption, output save/reopen',
        'cases': cases,
        'note': 'Content HMODs are assembled from the user-local imported pack carry tree; no imported mod files are copied into this repository.',
    }, indent=2) + '\n')
    print('Thermal Crystallizer recipe passed in both game modes and survived save/reopen')


if __name__ == '__main__':
    main()

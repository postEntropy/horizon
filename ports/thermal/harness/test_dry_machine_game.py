#!/usr/bin/env python3
"""Run Thermal's carried bronze alloy Smelter recipe in both game modes."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[3]
MOD = ROOT / 'ports/thermal'
sys.path.insert(0, str(ROOT / 'tools'))
from mod_port_game_fixture import Fixture

MACHINE = (0, 100, 0)
BLOCK = 'thermal:machine_smelter'
RECIPE = 'thermal:smelter_alloy_bronze'


def package_carry(carry, component, output):
    with zipfile.ZipFile(component) as package:
        component_bytes = package.read('mod.wasm')
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(MOD / 'horizon-mod.toml', 'horizon-mod.toml')
        archive.writestr('mod.wasm', component_bytes)
        for prefix in ('assets', 'data'):
            names = {}
            for root in (carry / prefix, MOD / prefix):
                if root.is_dir():
                    for path in sorted(root.rglob('*')):
                        if path.is_file():
                            names[path.relative_to(root.parent).as_posix()] = path
            for name, path in names.items():
                archive.write(path, name)


def slots(game):
    return game.call('container', *MACHINE)['slots']


def energy(game):
    line = game.command('data get block 0 100 0 horizon_energy.stored')
    import re
    match = re.findall(r'"(-?\d+)"', line)
    if not match:
        raise AssertionError(('horizon_energy.stored', line))
    return int(match[-1])


def run(args, mode, hmod):
    with Fixture(args, mode, [hmod]) as game:
        game.start()
        game.command('tp @s 0.5 102 3.5')
        game.call('setblock', *MACHINE, BLOCK)
        game.step(1)
        diagnostic = game.command('thermal recipes')
        if 'thermal:smelter=0' in diagnostic or 'copper tag matches=0' in diagnostic:
            raise AssertionError(('Thermal machine recipes were not loaded', diagnostic))
        print('diagnostic:', diagnostic, flush=True)
        game.command('item replace block 0 100 0 container.0 with minecraft:copper_ingot 3')
        game.command('item replace block 0 100 0 container.1 with thermal:tin_ingot 1')
        game.command('data modify block 0 100 0 horizon_energy.stored set value "6400"')
        game.step(100)
        actual = slots(game)
        stored = energy(game)
        expected = [None, None, {'count': 4, 'item': 'thermal:bronze_ingot'}, None]
        if actual != expected or stored != 0:
            raise AssertionError((mode, actual, stored))
        game.call('save')
        game.stop()
        game.start()
        persisted = slots(game)
        if persisted != expected:
            raise AssertionError((mode, 'output did not persist after reopen', persisted))
        game.stop()
        return {'mode': mode, 'status': 'passed', 'recipe': RECIPE, 'ticks': 100,
                'output': actual[2], 'energy_spent': 6400 - stored,
                'persisted_output': persisted[2]}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/mods/thermal/carry')
    p.add_argument('--component', type=Path, default=ROOT / 'ports/target/horizon-mod/thermal.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=MOD / 'harness/dry-machine-gameplay-validation.json')
    args = p.parse_args()
    for field in ('carry', 'component', 'bin_dir', 'resources', 'out'):
        setattr(args, field, getattr(args, field).resolve())
    if not args.component.is_file():
        subprocess.run(['cargo', 'run', '--manifest-path', str(ROOT / 'Cargo.toml'), '-p', 'cargo-horizon-mod', '--bin', 'cargo-horizon-mod', '--', 'horizon-mod', 'build', '--release'], cwd=MOD, check=True)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='thermal-dry-machine-') as temp:
        hmod = Path(temp) / 'thermal.hmod'
        package_carry(args.carry, args.component, hmod)
        cases = [run(args, mode, hmod) for mode in ('in-frame', 'server-thread')]
    args.out.write_text(json.dumps({'game': 'Horizon 26.3', 'mod': 'Thermal Series 9.2.2',
        'status': 'passed', 'scope': 'StoneBlock carried bronze alloy recipe in Thermal Smelter; tag ingredients, RF use and output save/reopen',
        'cases': cases, 'note': 'The HMOD was assembled from user-local imported carry data; no imported content was copied into the repository.'}, indent=2) + '\n')
    print('Thermal Smelter bronze alloy recipe passed in both game modes and survived save/reopen')


if __name__ == '__main__':
    main()

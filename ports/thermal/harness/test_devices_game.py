#!/usr/bin/env python3
"""Exercise Thermal Rock Gen and fail-closed Insolator behavior in live games."""
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


def make_hmod(args, output):
    with zipfile.ZipFile(args.component) as package:
        wasm = package.read('mod.wasm')
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(MOD / 'horizon-mod.toml', 'horizon-mod.toml')
        archive.writestr('mod.wasm', wasm)
        for prefix in ('assets', 'data'):
            merged = {}
            for root in (args.thermal_carry / prefix,
                         args.expansion_carry / prefix,
                         MOD / 'carry' / prefix,
                         MOD / prefix):
                if root.is_dir():
                    for path in sorted(root.rglob('*')):
                        if path.is_file():
                            name = path.relative_to(root.parent).as_posix()
                            if name.startswith(('assets/thermal/', 'data/thermal/',
                                                'data/minecraft/tags/')):
                                merged[name] = path
            for name, path in merged.items():
                archive.write(path, name)


def get_block(game):
    row = game.call('block', *MACHINE)
    state = row.get('block', row.get('state', ''))
    return state.get('name', '') if isinstance(state, dict) else str(state)


def run(args, mode, hmod):
    with Fixture(args, mode, [hmod]) as game:
        game.start()
        game.command('tp @s 0.5 102 3.5')
        game.call('setblock', *MACHINE, 'thermal:device_rock_gen')
        game.call('setblock', 1, 100, 0, 'minecraft:water')
        game.step(20)
        generated = game.slots(MACHINE)
        if not generated or generated[0] != {'count': 1, 'item': 'minecraft:cobblestone'}:
            neighbor = game.call('block', 1, 100, 0)
            raise AssertionError((mode, 'Rock Gen water recipe', generated, neighbor))

        game.call('setblock', *MACHINE, 'thermal:device_rock_gen')
        game.call('setblock', 1, 100, 0, 'minecraft:packed_ice')
        game.call('setblock', 0, 99, 0, 'minecraft:magma_block')
        game.step(20)
        generated_deepslate = game.slots(MACHINE)
        if generated_deepslate[0] != {'count': 1, 'item': 'minecraft:deepslate'}:
            raise AssertionError((mode, 'Rock Gen packed ice/magma recipe', generated_deepslate))

        game.call('setblock', *MACHINE, 'thermal:machine_insolator')
        game.call('setblock', 1, 100, 0, 'minecraft:air')
        game.command('item replace block 0 100 0 container.0 with minecraft:oak_sapling 1')
        game.command('data modify block 0 100 0 horizon_energy.stored set value "50000"')
        game.command('data modify block 0 100 0 horizon_fluids[0].fluid set value "minecraft:water"')
        game.command('data modify block 0 100 0 horizon_fluids[0].amount set value "8000"')
        game.step(100)
        insolator = game.slots(MACHINE)
        if insolator[0] != {'count': 1, 'item': 'minecraft:oak_sapling'}:
            raise AssertionError((mode, 'uncalibrated Insolator unexpectedly consumed input', insolator))

        game.call('setblock', *MACHINE, 'thermal:machine_smelter[facing=north]')
        game.command('item replace block 0 100 0 container.0 with minecraft:cobblestone 1')
        game.command('data modify block 0 100 0 horizon_energy.stored set value "500"')
        game.command('item replace entity @s hotbar.0 with thermal:wrench 1')
        game.call('select', '1')
        game.call('look-at-block', *MACHINE, 'up')
        game.call('interact', *MACHINE)
        rotated = game.call('block', *MACHINE)
        rotated_text = json.dumps(rotated)
        rotated_slots = game.slots(MACHINE)
        if 'facing=east' not in rotated_text and 'east' not in rotated_text:
            raise AssertionError((mode, 'Crescent Hammer did not rotate facing', rotated))
        if rotated_slots[0] != {'count': 1, 'item': 'minecraft:cobblestone'}:
            raise AssertionError((mode, 'rotation did not preserve machine inventory', rotated_slots))

        game.command('item replace block 0 100 0 container.0 with minecraft:air')
        game.command('data modify block 0 100 0 horizon_energy.stored set value "0"')
        game.call('key', 'shift', 'down')
        game.call('interact', *MACHINE)
        game.call('key', 'shift', 'up')
        dismantled = get_block(game)
        if 'minecraft:air' not in dismantled:
            raise AssertionError((mode, 'sneaking Crescent Hammer did not dismantle empty machine', dismantled))
        game.stop()
        return {'mode': mode, 'status': 'passed',
                'rock_gen_water': generated[0],
                'rock_gen_packed_ice_magma': generated_deepslate[0],
                'insolator': 'blocked without verified energy_mod/water_mod costs',
                'crescent_hammer': 'rotation preserved inventory; sneak dismantle removed empty machine'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--thermal-carry', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/mods/thermal/carry')
    parser.add_argument('--expansion-carry', type=Path, default=Path.home() / '.local/share/horizon/ports/ftb-stoneblock-3/mods/thermal_expansion/carry')
    parser.add_argument('--component', type=Path, default=ROOT / 'ports/target/horizon-mod/thermal.hmod')
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/device-gameplay-validation.json')
    parser.add_argument('--mode', choices=('in-frame', 'server-thread', 'both'), default='both')
    args = parser.parse_args()
    for field in ('thermal_carry', 'expansion_carry', 'component', 'bin_dir', 'resources', 'out'):
        setattr(args, field, getattr(args, field).resolve())
    if not args.component.is_file():
        parser.error('build the Thermal component before running this harness')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='thermal-devices-') as temp:
        hmod = Path(temp) / 'thermal.hmod'
        make_hmod(args, hmod)
        modes = ('in-frame', 'server-thread') if args.mode == 'both' else (args.mode,)
        cases = [run(args, mode, hmod) for mode in modes]
    args.out.write_text(json.dumps({'game': 'Horizon 26.3', 'mod': 'Thermal Series 9.2.2',
        'status': 'passed', 'scope': 'Rock Gen adjacency recipes and fail-closed Insolator in both modes',
        'cases': cases}, indent=2) + '\n')
    print('Thermal Rock Gen and Insolator live cases passed in both modes')


if __name__ == '__main__':
    main()

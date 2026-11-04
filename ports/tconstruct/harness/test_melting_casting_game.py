#!/usr/bin/env python3
"""Verify TConstruct Heater -> Melter fluid save -> Faucet casting in two modes."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[3]
MOD = ROOT / 'ports/tconstruct'
MELTER = (0, 100, 0)
HEATER = (1, 100, 0)
FAUCET = (0, 100, 1)
TABLE = (0, 100, 2)


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            root = source / dirname
            if root.exists():
                for path in sorted(root.rglob('*')):
                    if path.is_file():
                        archive.write(path, path.relative_to(source).as_posix())


def block(game, pos):
    return game.call('getblock', *pos).get('block', '')


def run_case(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        for pos, block_id in ((MELTER, 'tconstruct:seared_melter'),
                              (HEATER, 'tconstruct:seared_heater'),
                              (FAUCET, 'tconstruct:seared_faucet'),
                              (TABLE, 'tconstruct:seared_table')):
            game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} {block_id}')
            assert block(game, pos).startswith(block_id), (mode, pos, block(game, pos))
        game.replace(HEATER, 0, 'minecraft:coal 1')
        game.replace(MELTER, 0, 'tconstruct:grout 1')
        game.step(85)
        assert not game.slots(MELTER)[0], (mode, 'grout did not melt', game.slots(MELTER))
        assert block(game, HEATER).find('lit=true') >= 0, (mode, 'heater did not remain lit', block(game, HEATER))
        game.call('save')
        game.stop()

        # The cast is deliberately inserted after restart: the resulting brick
        # proves the melter's fluid survived the world save rather than being
        # present only in the in-memory state.
        game.start()
        game.replace(TABLE, 0, 'tconstruct:ingot_sand_cast 1')
        game.step(2)
        slots = game.slots(TABLE)
        assert any(s and s['item'] == 'tconstruct:seared_brick' for s in slots), (mode, 'cast result missing after reload', slots)
        assert not slots[0], (mode, 'single-use sand cast was not consumed', slots)
        result = {'mode': mode, 'status': 'passed', 'melt_ticks': 85,
                  'world_reloaded_before_casting': True, 'output': 'tconstruct:seared_brick'}
        game.stop()
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = parser.parse_args()
    for name in ('bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.tconstruct-hmod-', dir=args.out.parent) as tmp:
        args.hmod = Path(tmp) / 'tconstruct.hmod'
        subprocess.run(['cargo', 'run', '--manifest-path', str(ROOT / 'Cargo.toml'),
                        '-p', 'cargo-horizon-mod', '--bin', 'cargo-horizon-mod', '--',
                        'horizon-mod', 'build', '--release'], cwd=MOD, check=True)
        package(MOD, ROOT / 'ports/target/horizon-mod/tconstruct.hmod', args.hmod)
        cases = [run_case(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.write_text(json.dumps({
        'game': 'Horizon 26.3', 'mod': "Tinkers' Construct 3.6.4.113", 'status': 'passed',
        'scope': 'Fuel burns in adjacent Heater, grout melts to 500 mB seared stone, fluid survives save/reload, and faucet casts one seared brick.',
        'known_gaps': ['Only seared stone and seared brick are supported by this first casting path.',
                       'Part Builder and Tinker Station are registered content and recipes, not tool-forging menus.',
                       'Multiblock smeltery, alloys, material tools, modifiers, and TConstruct slime-specific behavior remain incomplete.'],
        'cases': cases,
    }, indent=2) + '\n')
    print('TConstruct melting and casting passed in both game modes')


if __name__ == '__main__':
    main()

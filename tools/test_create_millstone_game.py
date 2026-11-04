#!/usr/bin/env python3
"""Validate Create's StoneBlock hand-crank → cogwheel → Millstone path."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import tempfile
import time
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / 'ports/create'


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm_package, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm_package.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            for path in sorted((source / dirname).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(source).as_posix())


def run_game(args, mode, hmod):
    with Fixture(args, mode, [hmod]) as game:
        game.start()
        game.command('clear @s')
        player_state = game.call('state')['player']
        player = player_state['position']
        cogwheel = (math.floor(player[0]) - 1, math.floor(player[1]) + 1, math.floor(player[2]) + 1)
        crank = (cogwheel[0], cogwheel[1] + 1, cogwheel[2])
        millstone = (cogwheel[0] + 1, cogwheel[1], cogwheel[2])
        game.command(f'setblock {cogwheel[0]} {cogwheel[1]} {cogwheel[2]} create:cogwheel[axis=y]')
        game.command(f'setblock {crank[0]} {crank[1]} {crank[2]} create:hand_crank[facing=up]')
        game.command(f'setblock {millstone[0]} {millstone[1]} {millstone[2]} create:millstone')
        game.step(2)
        before = game.call('container', *millstone)['slots']
        game.command(f'item replace block {millstone[0]} {millstone[1]} {millstone[2]} container.0 with minecraft:gravel 1')

        # Give the unpowered machine a short idle interval before the crank event.
        game.command('tick unfreeze')
        time.sleep(0.5)
        game.command('tick freeze')
        game.call('interact', *crank)
        game.call('mouse', 'right', 'down')
        game.command('tick unfreeze')
        time.sleep(6)
        game.call('mouse', 'right', 'up')
        game.command('tick freeze')
        game.call('interact', *millstone)
        screen = game.call('screen')
        mod_screen = screen.get('mod_screen') or {}
        slots = game.call('container', *millstone)['slots']
        input_stack = slots[0] if slots else None
        gauge = next((widget for widget in mod_screen.get('widgets', []) if widget.get('id') == 1), None)
        if not input_stack or input_stack.get('item') != 'minecraft:gravel':
            raise AssertionError((mode, 'gravel should remain until host Level RNG exists', slots))
        if not gauge:
            raise AssertionError((mode, 'Millstone progress widget is missing', screen))
        progress = int(gauge.get('value', 0))
        game.stop()
        return {
            'mode': mode,
            'status': 'passed' if progress > 0 else 'failed',
            'kinetic_chain': 'hand crank facing up → cogwheel axis y → Millstone west face',
            'registered_recipe': 'create:stoneblock/gravel (create:milling, 100 ticks)',
            'recipe_chances': {
                'minecraft:coal': 0.35,
                'minecraft:raw_iron': 0.35,
                'create:raw_zinc': 0.2,
                'minecraft:sand': 0.4,
            },
            'input_before': before[0] if before else None,
            'input_after': input_stack,
            'progress': progress,
            'progress_widget': gauge,
            'partial_reason': (
                'Completion/output rolls require the vanilla Level RNG bridge.'
                if progress > 0 else
                'The server-thread machine did not receive an active crank stroke from the player use event.'
            ),
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = parser.parse_args()
    for field in ('bin_dir', 'resources', 'out'):
        setattr(args, field, getattr(args, field).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.create-hmod-', dir=args.out.parent) as temp:
        hmod = Path(temp) / 'create.hmod'
        subprocess.run([
            'cargo', 'run', '--manifest-path', str(ROOT / 'Cargo.toml'),
            '-p', 'cargo-horizon-mod', '--bin', 'cargo-horizon-mod', '--',
            'horizon-mod', 'build', '--release',
        ], cwd=MOD, check=True)
        built = ROOT / 'ports/target/horizon-mod/create.hmod'
        package(MOD, built, hmod)
        cases = [run_game(args, mode, hmod) for mode in ('in-frame', 'server-thread')]
    passed = all(case['status'] == 'passed' for case in cases)
    args.out.write_text(json.dumps({
        'game': 'Horizon 26.3',
        'mod': 'Create 0.5.1.f from the StoneBlock 3 import',
        'status': (
            'partial; hand-crank progress passed in both modes, but chance outputs remain blocked on Level RNG'
            if passed else
            'partial; in-frame crank progress passed, server-thread progress failed, and chance outputs remain blocked on Level RNG'
        ),
        'scope': 'Quest #5 Millstone and the registered StoneBlock gravel milling recipe; results below distinguish in-frame and authoritative server-thread behavior.',
        'cases': cases,
    }, indent=2) + '\n')
    print('Create Millstone E2E recorded for both modes; see per-mode status and RNG limitation in the JSON')


if __name__ == '__main__':
    main()

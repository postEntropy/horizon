#!/usr/bin/env python3
"""Validate the CoFH Core Glossed Magma state transition in Horizon."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / 'ports/cofh_core'
POS = (0, 101, 3)
SPREAD_CENTER = (6, 101, 3)
NEIGHBORS = [(7, 101, 3), (5, 101, 3), (6, 101, 4), (6, 101, 2)]
CLUSTER = [(x, 101, z) for x in range(5, 8) for z in range(2, 5)]
NEIGHBOR_CENTER = (12, 101, 3)
NEIGHBOR_CLUSTER = [(x, 101, z) for x in range(11, 14) for z in range(2, 5)]


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm_package, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm_package.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            for path in sorted((source / dirname).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(source).as_posix())


def block(game, pos):
    return game.call('getblock', *pos).get('block', '')


def health(game):
    return float(game.call('state')['player']['health'])


def run_game(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('gamerule minecraft:random_tick_speed 0')
        game.command('setblock 0 101 3 minecraft:stone')
        game.command('tp @s 0.5 102 3.5')

        # A saturated cluster exercises the always-consumed nextInt(9), its
        # 1-in-9 melt branch, and age-3 propagation into adjacent blocks.
        for pos in CLUSTER:
            if pos != SPREAD_CENTER:
                game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} cofh_core:glossed_magma[age=0]')
        game.command(f'setblock {SPREAD_CENTER[0]} {SPREAD_CENTER[1]} {SPREAD_CENTER[2]} cofh_core:glossed_magma[age=3]')
        game.command('gamerule minecraft:random_tick_speed 1000')
        game.step(20)
        spread_center = block(game, SPREAD_CENTER)
        spread_neighbors = [block(game, pos) for pos in NEIGHBORS]
        assert spread_center.startswith('minecraft:lava'), (mode, 'saturated roll never reached the melt branch', spread_center)
        assert any(state.startswith('cofh_core:glossed_magma[age=1]') or state.startswith('minecraft:lava')
                   for state in spread_neighbors), (mode, 'age-3 spread did not affect neighbors', spread_neighbors)
        for pos in CLUSTER:
            game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} minecraft:air')

        # Fewer than two adjacent Glossed Magma blocks makes neighborChanged
        # turn the current block into lava immediately.
        game.command('gamerule minecraft:random_tick_speed 0')
        for pos in NEIGHBOR_CLUSTER:
            if pos != NEIGHBOR_CENTER:
                game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} cofh_core:glossed_magma[age=0]')
        game.command(f'setblock {NEIGHBOR_CENTER[0]} {NEIGHBOR_CENTER[1]} {NEIGHBOR_CENTER[2]} cofh_core:glossed_magma[age=1]')
        for pos in [(13, 101, 3), (11, 101, 3), (12, 101, 4)]:
            game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} minecraft:air')
        changed = block(game, NEIGHBOR_CENTER)
        assert changed.startswith('minecraft:lava'), (mode, 'neighborChanged threshold did not melt block', changed)
        for pos in NEIGHBOR_CLUSTER:
            if pos != NEIGHBOR_CENTER:
                game.command(f'setblock {pos[0]} {pos[1]} {pos[2]} minecraft:air')

        # Validate inherited magma damage on the actual player, then verify
        # the carried step_damage property and sneak suppression.
        game.command('tp @s 0.5 102 3.5')
        game.command(f'setblock {POS[0]} {POS[1]} {POS[2]} minecraft:magma_block')
        before_magma = health(game)
        game.step(15)
        after_magma = health(game)
        assert after_magma < before_magma, (mode, 'vanilla magma did not damage the player', before_magma, after_magma)
        game.command('effect give @s minecraft:instant_health 1 5')
        game.step(1)
        game.command(f'setblock {POS[0]} {POS[1]} {POS[2]} cofh_core:glossed_magma[age=0]')
        before_glossy = health(game)
        game.step(15)
        after_glossy = health(game)
        assert after_glossy < before_glossy, (mode, 'Glossed Magma step_damage did not hurt the player', before_glossy, after_glossy)
        game.command('effect give @s minecraft:instant_health 1 5')
        game.step(1)
        game.call('key', 'shift', 'down')
        before_sneak = health(game)
        game.step(15)
        after_sneak = health(game)
        game.call('key', 'shift', 'up')
        assert after_sneak == before_sneak, (mode, 'sneaking did not suppress Glossed Magma damage', before_sneak, after_sneak)

        game.command('setblock 2 101 3 minecraft:stone')
        game.command('tp @s 2.5 102 3.5')
        game.command('gamerule minecraft:random_tick_speed 1000')
        game.command(f'setblock {POS[0]} {POS[1]} {POS[2]} cofh_core:glossed_magma[age=0]')
        initial = block(game, POS)
        assert initial.startswith('cofh_core:glossed_magma[age=0]'), (mode, initial)

        # An isolated Glossed Magma block has fewer than four same-block
        # neighbors, so each vanilla random tick advances one age. At age 3
        # the original block replaces itself with source lava.
        melted = block(game, POS)
        ticks = 0
        while not melted.startswith('minecraft:lava') and ticks < 200:
            game.step(20)
            ticks += 20
            melted = block(game, POS)
        assert melted.startswith('minecraft:lava'), (mode, melted)

        # The imported CoFH loot table intentionally has no pools; mining it
        # gives no block item (the original class also returns an empty stack).
        game.command(f'setblock {POS[0]} {POS[1]} {POS[2]} cofh_core:glossed_magma[age=0]')
        before = game.call('inventory').get('counts', {}).get('cofh_core:glossed_magma', 0)
        game.command(f'loot give @s mine {POS[0]} {POS[1]} {POS[2]}')
        after = game.call('inventory').get('counts', {}).get('cofh_core:glossed_magma', 0)
        assert after == before, (mode, 'Glossed Magma unexpectedly dropped an item', before, after)

        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'initial_state': initial,
            'state_after_random_ticks': melted,
            'ticks_advanced': ticks,
            'saturated_spread': {'center': spread_center, 'neighbors': spread_neighbors},
            'neighbor_change': changed,
            'step_damage': {'magma': [before_magma, after_magma], 'glossed_magma': [before_glossy, after_glossy],
                            'sneaking': [before_sneak, after_sneak]},
            'mining_loot': {'before': before, 'after': after, 'expected_drop': None},
        }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=MOD / 'harness/gameplay-validation.json')
    args = p.parse_args()
    for name in ('bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.cofh-core-hmod-', dir=args.out.parent) as temp:
        args.hmod = Path(temp) / 'cofh_core.hmod'
        subprocess.run([
            'cargo', 'run', '--manifest-path', str(ROOT / 'Cargo.toml'),
            '-p', 'cargo-horizon-mod', '--bin', 'cargo-horizon-mod', '--',
            'horizon-mod', 'build', '--release',
        ], cwd=MOD, check=True)
        built = ROOT / 'ports/target/horizon-mod/cofh_core.hmod'
        package(MOD, built, args.hmod)
        cases = [run_game(args, mode) for mode in ('in-frame', 'server-thread')]

    args.out.write_text(json.dumps({
        'game': 'Horizon 26.3',
        'mod': 'CoFH Core 9.2.3.47+mc1.18.2',
        'status': 'passed',
        'scope': 'Inline saturated 1-in-9 melt and spread, scheduled age progression, neighbor-change melt, inherited hot_floor damage/sneak suppression, and empty loot.',
        'source': 'Imported CoFH Core 9.2.3.47 behavior audit plus carried age=0..3 block states and empty Glossed Magma loot table.',
        'known_gaps': [
            'The other CoFH Core air blocks, Holding enchantment, status effects, commands and helper integrations are not ported.',
        ],
        'cases': cases,
    }, indent=2) + '\n')
    print('CoFH Core Glossed Magma state transition passed in both game modes')


if __name__ == '__main__':
    main()

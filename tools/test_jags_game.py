#!/usr/bin/env python3
"""Validate JAGS planting and grass drops in a real Horizon game."""
import argparse
import json
from pathlib import Path
import tempfile
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
SEED = 'jags:jags'
SUPPORT = (0, 101, 3)
DIRT = (0, 102, 4)
GRASS = DIRT


def count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def package(source, built, output):
    with zipfile.ZipFile(built) as wasm_package, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', (source / 'horizon-mod.toml').read_bytes())
        archive.writestr('mod.wasm', wasm_package.read('mod.wasm'))
        for dirname in ('assets', 'data'):
            for path in sorted((source / dirname).rglob('*')):
                if path.is_file():
                    archive.write(path, path.relative_to(source).as_posix())


def block(game, pos):
    result = game.call('block', *pos)
    state = result.get('block', result.get('state', ''))
    return state.get('name', '') if isinstance(state, dict) else str(state)


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('tp @s 0.5 102 3.5')
        game.command(f'setblock {SUPPORT[0]} {SUPPORT[1]} {SUPPORT[2]} minecraft:stone')
        game.command('setblock 0 101 4 minecraft:stone')
        game.command(f'setblock {DIRT[0]} {DIRT[1]} {DIRT[2]} minecraft:dirt')
        game.command(f'item replace entity @s hotbar.0 with {SEED} 2')
        game.call('hotbar', '1')
        game.call('look-at-block', *DIRT, 'up')
        game.call('use-item')
        if mode == 'server-thread':
            game.step(1)
        planted_block = block(game, DIRT)
        after_plant = count(game.call('inventory'), SEED)
        assert planted_block.startswith('minecraft:grass_block'), (mode, planted_block)
        assert after_plant == 1, (mode, 'seed was not consumed exactly once', after_plant)

        # Non-dirt use passes through and must leave the stack unchanged.
        game.command(f'setblock {DIRT[0]} {DIRT[1]} {DIRT[2]} minecraft:stone')
        game.call('look-at-block', *DIRT, 'up')
        game.call('use-item')
        if mode == 'server-thread':
            game.step(1)
        after_failed_target = count(game.call('inventory'), SEED)
        assert block(game, DIRT).startswith('minecraft:stone'), (mode, block(game, DIRT))
        assert after_failed_target == 1, (mode, 'failed target consumed seed', after_failed_target)
        game.command(f'setblock {DIRT[0]} {DIRT[1]} {DIRT[2]} minecraft:air')

        # Repeatedly roll fresh short-grass loot through the real /loot mine
        # path, which uses the block's named vanilla loot random sequence.
        before_loot = count(game.call('inventory'), SEED)
        observed_grass_drops = 0
        loot_outputs = []
        for _ in range(args.trials):
            game.command(f'setblock {GRASS[0]} {GRASS[1]} {GRASS[2]} minecraft:short_grass')
            loot_output = game.command(f'loot give @s mine {GRASS[0]} {GRASS[1]} {GRASS[2]}')
            if len(loot_outputs) < 2:
                loot_outputs.append(loot_output)
            current = count(game.call('inventory'), SEED)
            if current > before_loot:
                observed_grass_drops += current - before_loot
                before_loot = current
        total_seeds = count(game.call('inventory'), SEED)
        extra = total_seeds - after_failed_target
        stats = game.command('jags stats')
        min_extra, max_extra = ((0, 0) if args.trials < 20 else (1, 15))
        assert min_extra <= extra <= max_extra, (mode, '10% loot sample outside broad trial bounds', extra,
                                                 stats)
        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'planting': {'block_after': planted_block, 'held_before': 2, 'held_after': after_plant},
            'failed_target': {'block_after': 'minecraft:stone', 'held_after': after_failed_target},
            'short_grass': {'trials': args.trials, 'additional_jags_seeds': extra,
                             'observed_increments': observed_grass_drops, 'expected_chance': 0.1,
                             'sample_outputs': loot_outputs, 'mod_stats': stats},
        }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/jags.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / 'ports/jags/harness/gameplay-validation.json')
    p.add_argument('--seed', type=int, default=2)
    p.add_argument('--trials', type=int, default=60)
    args = p.parse_args()
    for name in ('hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.jags-hmod-', dir=args.out.parent) as temp:
        args.hmod = Path(temp) / 'jags.hmod'
        package(ROOT / 'ports/jags', ROOT / 'ports/target/horizon-mod/jags.hmod', args.hmod)
        cases = [run(args, mode) for mode in ('in-frame', 'server-thread')]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'game': 'Horizon 26.3',
        'mod': 'JAGS 1.3.3-build.16+mc1.18.2',
        'status': 'passed',
        'scope': 'grass planting with atomic seed consumption; rejected target preserves seed; short-grass loot chance',
        'cases': cases,
    }, indent=2) + '\n')
    print('JAGS planting, seed consumption and loot passed in both game modes')


if __name__ == '__main__':
    main()

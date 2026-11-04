#!/usr/bin/env python3
"""Vacuum item pickup transaction against the actual mod runtime and game."""
import argparse
import json
import os
import re
from pathlib import Path
from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]


def count_items(game, item):
    game.command('scoreboard players set #count router_items 0')
    game.command(f'execute as @e[type=minecraft:item,nbt={{Item:{{id:"{item}"}}}}] run scoreboard players add #count router_items 1')
    result = game.command('scoreboard players get #count router_items')
    values = re.findall(r'-?\d+', result)
    if not values:
        raise AssertionError(f'could not parse item count from: {result!r}')
    return int(values[-1])


def ensure_objective(game):
    try:
        game.command('scoreboard objectives add router_items dummy')
    except AssertionError:  # The objective may survive a restart in other save paths.
        pass


def stationary_drop(game, item, x, y, z):
    game.command(
        f'summon minecraft:item {x} {y} {z} '
        f'{{NoGravity:1b,Motion:[0.0d,0.0d,0.0d],Item:{{id:"{item}",count:1}}}}'
    )


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        ensure_objective(game)
        game.command('tp @s 32 102 4')
        game.call('fill', -8, 98, -8, 72, 98, 8, 'minecraft:stone')
        pos = (0, 100, 0)
        diag = (16, 100, 0)
        edge = (32, 100, 0)
        priority_a = (48, 100, 0)
        priority_b = (64, 100, 0)
        # Dynamic block placement only joins the block ticker after the next
        # chunk-tick scan. Stage every router before the first server tick.
        for router in [pos, diag, edge, priority_a, priority_b]:
            game.call('setblock', *router, 'modularrouters:modular_router')
            game.replace(router, 1, 'modularrouters:vacuum_module')
        # Matches the original oracle fixture's offset from its block origin.
        game.command('summon minecraft:item 0.5 100.3 -2.5 {Item:{id:"minecraft:diamond",count:1,components:{"minecraft:custom_name":{text:"vacuum test"}}}}')
        game.step(20)
        assert game.total([pos], 'minecraft:diamond') == 1
        assert count_items(game, 'minecraft:diamond') == 0
        assert 'vacuum test' in game.components(pos)
        game.call('save')
        game.stop()
        game.start()
        ensure_objective(game)
        assert game.total([pos], 'minecraft:diamond') == 1
        assert 'vacuum test' in game.components(pos)
        # Incompatible/full buffer: failure must leave the original entity intact.
        game.replace(pos, 0, 'minecraft:gold_ingot 64')
        game.command('summon minecraft:item 0.5 100.3 -2.5 {Item:{id:"minecraft:diamond",count:1}}')
        game.step(20)
        assert game.total([pos], 'minecraft:gold_ingot') == 64
        assert count_items(game, 'minecraft:diamond') == 1
        # Once space exists, the next server transaction collects exactly once.
        game.replace(pos, 0, 'minecraft:air')
        game.step(40)
        assert game.total([pos], 'minecraft:diamond') == 1, game.slots(pos)
        assert count_items(game, 'minecraft:diamond') == 0
        game.call('save')
        game.stop()
        game.start()
        ensure_objective(game)
        assert game.total([pos], 'minecraft:diamond') == 1
        assert count_items(game, 'minecraft:diamond') == 0
        game.replace(pos, 0, 'minecraft:gold_ingot 64')
        # Match the original block-bounds inflate(6) query. This diagonal is
        # inside the box and outside a sphere of the same 6.5-center radius.
        stationary_drop(game, 'minecraft:diamond', diag[0] + 6.5, 100.3, -5.5)
        game.step(20)
        assert game.total([diag], 'minecraft:diamond') == 1
        assert count_items(game, 'minecraft:diamond') == 0
        stationary_drop(game, 'minecraft:diamond', edge[0] + 0.5, 100.3, -6.125)
        game.step(20)
        assert game.total([edge], 'minecraft:diamond') == 0
        assert count_items(game, 'minecraft:diamond') == 1
        game.command('kill @e[type=minecraft:item]')
        # The original chooses the first matching entity from world query order,
        # even when a later candidate is closer.
        stationary_drop(game, 'minecraft:diamond', priority_a[0] + 0.5, 100.3, -4.5)
        stationary_drop(game, 'minecraft:emerald', priority_a[0] + 0.5, 100.3, -1.5)
        game.step(20)
        assert game.total([priority_a], 'minecraft:diamond') == 1
        assert count_items(game, 'minecraft:emerald') == 1
        game.command('kill @e[type=minecraft:item]')
        stationary_drop(game, 'minecraft:emerald', priority_b[0] + 0.5, 100.3, -1.5)
        stationary_drop(game, 'minecraft:diamond', priority_b[0] + 0.5, 100.3, -4.5)
        game.step(20)
        assert game.total([priority_b], 'minecraft:emerald') == 1
        assert count_items(game, 'minecraft:diamond') == 1
        game.command('kill @e[type=minecraft:item]')
        # Move beyond render distance to force the source chunk out, then return.
        unload = pos
        for blocked in [pos, diag, edge, priority_a, priority_b]:
            game.replace(blocked, 0, 'minecraft:gold_ingot 64')
        stationary_drop(game, 'minecraft:diamond', unload[0] + 0.5, 100.3, -2.5)
        game.step(20)
        assert game.total([unload], 'minecraft:gold_ingot') == 64
        game.command('tp @s 512 102 512')
        game.step(40)
        # The distant ticket unloads the source chunk; its item is absent from
        # the active entity set until returning and loading that chunk again.
        assert count_items(game, 'minecraft:diamond') == 0
        game.command('tp @s 0 102 4')
        game.call('wait-ready', '180')
        game.step(20)
        assert game.total([unload], 'minecraft:gold_ingot') == 64
        assert count_items(game, 'minecraft:diamond') == 1
        game.replace(unload, 0, 'minecraft:air')
        game.step(20)
        assert game.total([unload], 'minecraft:diamond') == 1
        assert count_items(game, 'minecraft:diamond') == 0
        game.stop()
        return {'mode': mode, 'status': 'passed', 'checks': [
            'observed drop collected with components retained',
            'diagonal outside sphere is collected; touching outer AABB edge is excluded',
            'first world-query candidate wins over a nearer later candidate in both orders',
            'full buffer leaves drop untouched', 'successful retry conserves count',
            'save/reopen retains custom components and collected count without a duplicate',
            'drop survives forced chunk unload/reload and is collected once']}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--hmod', type=Path, required=True)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--mode', choices=['in-frame', 'server-thread', 'both'], default='both')
    args = parser.parse_args()
    for field in ['hmod', 'bin_dir', 'resources', 'out']:
        setattr(args, field, getattr(args, field).resolve())
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:6])
    os.nice(max(0, 10 - os.getpriority(os.PRIO_PROCESS, 0)))
    modes = ['in-frame', 'server-thread'] if args.mode == 'both' else [args.mode]
    report = {'scope': 'Vacuum only; authoritative vanilla item-entity to mod-buffer transaction',
              'results': [run(args, mode) for mode in modes]}
    args.out.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()

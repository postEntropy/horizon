#!/usr/bin/env python3
"""Real router transfer conservation, components, blocked output and reopen."""
import argparse
import json
import os
from pathlib import Path
import time
from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
ROUTER = 'modularrouters:modular_router'


def process_memory_mib(pid):
    values = {}
    for line in Path(f'/proc/{pid}/status').read_text().splitlines():
        if line.startswith(('VmRSS:', 'VmHWM:')):
            name, value, *_ = line.split()
            values[name[:-1].lower()] = round(int(value) / 1024, 2)
    return values


def router(game, pos, modules):
    game.call('setblock', *pos, 'minecraft:air')
    game.call('setblock', *pos, ROUTER)
    for slot, (kind, face) in enumerate(modules, 1):
        game.replace(pos, slot, f'modularrouters:{kind}[minecraft:custom_data={{"modularrouters:direction":"{face}"}}]')


def run(args, mode):
    started = time.perf_counter()
    mods = [args.hmod, args.reader]
    if args.peer:
        mods.append(args.peer)
    with Fixture(args, mode, mods) as game:
        game.start()
        memory_started = process_memory_mib(game.process.pid)
        game.command('tp @s 0 102 4')
        game.call('fill', -8, 98, -4, 8, 98, 4, 'minecraft:stone')
        source, left, right = (0, 100, 0), (-1, 100, 0), (1, 100, 0)
        game.call('setblock', *source, 'minecraft:chest')
        router(game, left, [('puller_module_1', 'east')])
        router(game, right, [('puller_module_1', 'west')])
        game.step(40)
        assert game.total([left, source, right], 'minecraft:cobblestone') == 0
        game.replace(source, 0, 'minecraft:diamond')
        game.step(20)
        assert game.total([source], 'minecraft:diamond') == 0
        assert game.total([left, right], 'minecraft:diamond') == 1
        for pos in [left, right]:
            actual = game.total([pos], 'minecraft:diamond')
            summary = game.command(f'machine_api item_slots minecraft:overworld:{pos[0]},{pos[1]},{pos[2]}')
            assert (f'0=minecraft:diamond*{actual}' in summary) if actual else ('0=' not in summary), summary
        game.replace(source, 0, 'minecraft:diamond 17')
        game.step(20)
        assert game.total([left, source, right], 'minecraft:diamond') == 18
        # Full and incompatible buffers refuse extraction from the source.
        for pos in [left, right]:
            game.replace(pos, 0, 'minecraft:diamond 64')
        game.replace(source, 0, 'minecraft:diamond 17')
        game.step(40)
        assert game.total([source], 'minecraft:diamond') == 17
        for pos in [left, right]:
            game.replace(pos, 0, 'minecraft:gold_ingot')
        game.step(40)
        assert game.total([source], 'minecraft:diamond') == 17
        # A pull then push in one module cycle uses the actual newly moved stack.
        inlet, machine, outlet = (-5, 100, 0), (-4, 100, 0), (-3, 100, 0)
        for pos in [inlet, outlet]:
            game.call('setblock', *pos, 'minecraft:chest')
        router(game, machine, [('puller_module_1', 'west'), ('sender_module_1', 'east')])
        item = 'minecraft:iron_sword[minecraft:custom_name="Route sword",minecraft:enchantments={"minecraft:sharpness":3}]'
        game.replace(inlet, 0, item)
        game.command('data modify block -5 100 0 Items[0].components."horizon:energy" set value 1234L')
        original = game.components(inlet)
        game.step(20)
        assert game.total([inlet, machine, outlet], 'minecraft:iron_sword') == 1
        assert game.total([outlet], 'minecraft:iron_sword') == 1
        assert game.components(outlet) == original
        # Block a second output, leaving the first routed stack in the buffer.
        inlet2, machine2, outlet2 = (3, 100, 0), (4, 100, 0), (5, 100, 0)
        for pos in [inlet2, outlet2]:
            game.call('setblock', *pos, 'minecraft:chest')
        for slot in range(27):
            game.replace(outlet2, slot, 'minecraft:cobblestone 64')
        router(game, machine2, [('puller_module_1', 'west'), ('sender_module_1', 'east')])
        game.replace(inlet2, 0, item)
        game.command('data modify block 3 100 0 Items[0].components."horizon:energy" set value 1234L')
        game.step(40)
        assert game.total([machine2], 'minecraft:iron_sword') == 1
        assert game.total([inlet2], 'minecraft:iron_sword') == 0
        assert game.components(machine2) == original
        assert game.total([outlet2], 'minecraft:cobblestone') == 27 * 64
        game.call('save')
        game.stop()
        game.start()
        assert game.components(machine2) == original
        game.replace(outlet2, 0, 'minecraft:air')
        game.step(20)
        assert game.total([machine2], 'minecraft:iron_sword') == 0
        assert game.total([outlet2], 'minecraft:iron_sword') == 1
        assert game.components(outlet2) == original
        # Unsupported world/entity modules must not pretend to perform an action.
        router(game, left, [('breaker_module', 'east')])
        game.replace(left, 0, 'minecraft:diamond')
        game.step(40)
        assert game.total([left], 'minecraft:diamond') == 1
        router(game, right, [('vacuum_module', 'west')])
        game.step(40)
        assert game.total([right], 'minecraft:cobblestone') == 0
        # A foreign machine disallows extraction of its input from the north.
        protected, puller = (8, 100, 0), (8, 100, -1)
        game.call('setblock', *protected, 'test_machine:smelter')
        game.replace(protected, 0, 'minecraft:diamond')
        router(game, puller, [('puller_module_1', 'south')])
        game.step(40)
        assert game.total([protected], 'minecraft:diamond') == 1
        assert game.total([puller], 'minecraft:diamond') == 0
        # A machine guest issues a push request into a block entity owned by a
        # separately loaded guest. The host must conserve the stack and retain
        # its component data across the ownership boundary.
        transfer_elapsed = None
        transfer_memory_before = None
        transfer_memory_after = None
        if args.peer:
            cross_source, cross_peer = (10, 100, 0), (11, 100, 0)
            game.call('setblock', *cross_source, 'test_machine:smelter')
            game.call('setblock', *cross_peer, 'test_peer:cell')
            cross_stack = 'minecraft:diamond[minecraft:custom_name="Cross-owner transfer"]'
            game.replace(cross_source, 0, cross_stack)
            game.command('data modify block 10 100 0 Items[0].components."horizon:energy" set value 1234L')
            game.command('data modify block 10 100 0 Items[0].components."minecraft:custom_data" set value {atm10:{owner:"test_machine",sequence:17}}')
            game.command('data modify block 10 100 0 Items[0].components."minecraft:enchantments" set value {levels:{"minecraft:unbreaking":2}}')
            game.step(1)
            cross_components = game.components(cross_source)
            assert '1234' in cross_components and 'unbreaking' in cross_components and 'test_machine' in cross_components
            transfer_started = time.perf_counter()
            transfer_memory_before = process_memory_mib(game.process.pid)
            game.command('machine_api move_push')
            game.step(1)
            assert game.total([cross_source, cross_peer], 'minecraft:diamond') == 1
            assert game.total([cross_source], 'minecraft:diamond') == 0
            assert game.total([cross_peer], 'minecraft:diamond') == 1
            assert game.components(cross_peer) == cross_components
            transfer_elapsed = round(time.perf_counter() - transfer_started, 3)
            transfer_memory_after = process_memory_mib(game.process.pid)
            game.call('save')
            game.stop()
            game.start()
            assert game.total([cross_source], 'minecraft:diamond') == 0
            assert game.total([cross_peer], 'minecraft:diamond') == 1
            assert game.total([cross_source, cross_peer], 'minecraft:diamond') == 1
            assert game.components(cross_peer) == cross_components
        # Two endpoints of one canonical inventory must not route to themselves.
        a, b = (0, 100, 3), (1, 100, 3)
        for pos in [a, b]:
            game.call('setblock', *pos, 'test_machine:shared_router')
        game.replace(a, 0, 'minecraft:emerald')
        game.command('machine_api move_pull')
        game.step(40)
        assert game.total([a], 'minecraft:emerald') == 1
        assert game.total([b], 'minecraft:emerald') == 1
        # An earlier queued world action removes a push destination. No source
        # debit is permitted when the inventory is gone by application time.
        emitter, removed = (0, 100, -3), (1, 100, -3)
        game.call('setblock', *emitter, 'test_machine:smelter')
        game.call('setblock', *removed, 'minecraft:chest')
        game.replace(emitter, 0, 'minecraft:diamond')
        game.command('machine_api move_push_remove')
        game.step(1)
        assert game.total([emitter], 'minecraft:diamond') == 1
        game.stop()
        return {'mode': mode, 'status': 'passed', 'checks': [
            'empty source creates nothing', 'two routers conserve last item', 'partial extraction conserves 18 items',
            'full/incompatible buffer leaves source unchanged', 'pull/push execute in order',
            'name, energy and enchantments preserved', 'full output retains buffer', 'save/reopen with blocked output',
            'direction components persist', 'unsupported modules do not create/delete buffer items',
            'foreign capability snapshot refreshed immediately', 'foreign sided extraction honored',
            *(['cross-mod guest-to-guest transfer conserves stack, custom name, energy, enchantment and custom data',
               'cross-mod transferred stack survives save/reopen exactly once with all components'] if args.peer else []),
            'same canonical inventory cannot route into itself', 'destination removed before application leaves source intact'],
            'runtime_measurement': {
                'whole_fixture_wall_s': round(time.perf_counter() - started, 3),
                'game_memory_started_mib': memory_started,
                'cross_mod_transfer_wall_s_including_one_controlled_tick_and_queries': transfer_elapsed,
                'cross_mod_transfer_memory_before_mib': transfer_memory_before,
                'cross_mod_transfer_memory_after_mib': transfer_memory_after,
                'limitation': 'single run per mode; wall time includes command/control overhead and one stepped game tick; cross-transfer RSS snapshots are from the same process, but game_memory_started_mib belongs to the pre-save/reopen process; snapshots are not an isolated operation cost'},
            'remaining': ['remote/world/entity modules', 'filters/augments/config UI', 'redstone and configurable energy costs']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hmod', type=Path, required=True)
    parser.add_argument('--reader', type=Path, required=True, help='test_machine fixture hmod with smelter/shared_router content templates')
    parser.add_argument('--peer', type=Path, help='second mod with a one-slot test_peer:cell target')
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--mode', choices=['in-frame', 'server-thread', 'both'], default='both')
    args = parser.parse_args()
    for field in ['hmod', 'reader', 'bin_dir', 'resources', 'out']:
        setattr(args, field, getattr(args, field).resolve())
    if args.peer:
        args.peer = args.peer.resolve()
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:6])
    os.nice(max(0, 10 - os.getpriority(os.PRIO_PROCESS, 0)))
    modes = ['in-frame', 'server-thread'] if args.mode == 'both' else [args.mode]
    report = {'scope': 'actual WASM and real game inventories, basic adjacent puller/sender only',
              'results': [run(args, mode) for mode in modes]}
    args.out.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()

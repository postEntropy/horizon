#!/usr/bin/env python3
"""Shared-port inventories through real hoppers and world reopen, in both single-player paths.

Requires a packaged port with carried resources; never changes an installed pack.
Run under a CPU affinity/priority limit, e.g. taskset -c 0-5 nice -n 10 python3 ...
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def run_case(args, mode):
    with tempfile.TemporaryDirectory(prefix=f'horizon-shared-{mode}-') as scratch:
        scratch = Path(scratch)
        env = dict(os.environ, HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                   HORIZON_TELEMETRY_URL='off', RAYON_NUM_THREADS='6')
        for name in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            env[name] = tempfile.mkdtemp(prefix=name.lower() + '-', dir=scratch)
        socket = scratch / 'ctl.sock'
        ctl = [str(args.bin_dir / 'horizon-ctl'), '--socket', str(socket)]
        process = None
        logs = []

        def call(*words):
            print(f'{mode}: {" ".join(map(str, words))}', flush=True)
            result = subprocess.run(ctl + list(map(str, words)), env=env, cwd=scratch,
                                    capture_output=True, text=True, timeout=180)
            lines = result.stdout.strip().splitlines()
            reply = json.loads(lines[-1]) if lines else {'error': result.stderr.strip()}
            if result.returncode or 'error' in reply:
                raise AssertionError(f'{words}: {reply}')
            return reply

        def start():
            nonlocal process
            log = scratch / f'game-{len(logs)}.log'
            logs.append(log)
            command = [str(args.bin_dir / 'horizon-client'), '--resources', str(args.resources),
                       '--dev-mod', str(args.hmod), '--world', 'SharedInventoryFixture',
                       '--test-terrain', '--creative', '--render-distance', '2', '--offscreen',
                       '--width', '320', '--height', '240', '--control-socket', str(socket)]
            if mode == 'in-frame':
                command.append('--in-frame')
            with log.open('w') as stream:
                process = subprocess.Popen(command, cwd=scratch, env=env, stdout=stream,
                                           stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 180
            while not socket.exists():
                if process.poll() is not None:
                    raise AssertionError(log.read_text(errors='replace')[-6000:])
                if time.monotonic() > deadline:
                    raise AssertionError('control socket did not open')
                time.sleep(0.2)
            call('wait-ready', '180')
            call('command', 'tick freeze')
            print(f'{mode}: world ready', flush=True)

        def stop():
            nonlocal process
            call('quit')
            process.wait(timeout=45)
            assert process.returncode == 0, process.returncode
            process = None
            socket.unlink(missing_ok=True)

        def slots(pos):
            return call('container', *pos)['slots']

        def total(positions, item='minecraft:diamond'):
            return sum(stack['count'] for pos in positions for stack in slots(pos)
                       if stack and stack['item'] == item)

        def step(ticks):
            call('command', f'tick step {ticks}')
            # The wall clock bound allows the real 20 TPS server to finish the
            # requested simulation even when the render client has its own clock.
            time.sleep(ticks / 20 + 1)

        a, b = (0, 100, 0), (4, 100, 0)
        ha, hb = (0, 99, 0), (4, 99, 0)
        oa, ob = (-1, 99, 0), (5, 99, 0)
        try:
            start()
            call('command', 'tp @s 2 102 4')
            call('fill', -8, 98, -8, 8, 98, 8, 'minecraft:stone')
            for pos in [a, b]:
                call('setblock', *pos, args.block)
            for pos in [oa, ob]:
                call('setblock', *pos, 'minecraft:chest')
            call('setblock', *ha, 'minecraft:hopper[facing=west]')
            call('setblock', *hb, 'minecraft:hopper[facing=east]')
            call('container', *a, 'minecraft:diamond', 1)
            assert total([b]) == 1, 'second endpoint did not see insertion immediately'
            step(40)
            assert total([a]) == 0, 'diamond was restored after hopper extraction'
            assert total([ha, hb, oa, ob]) == 1, 'two hoppers duplicated the last item'
            call('container', *b, 'minecraft:diamond', 17)
            step(40)
            assert total([a, ha, hb, oa, ob]) == 18, 'partial extraction did not conserve items'
            step(160)
            assert total([a]) == 0
            assert total([ha, hb, oa, ob]) == 18
            step(40)
            assert total([a]) == 0, 'items reappeared on an idle tick'
            assert total([ha, hb, oa, ob]) == 18
            print(f'{mode}: hopper conservation passed', flush=True)
            # Remove automation before testing reopen (the frozen tick rate is
            # deliberately not relied upon as persistent world state).
            for pos in [ha, hb]:
                call('setblock', *pos, 'minecraft:air')
            call('container', *a, 'minecraft:gold_ingot', 11)
            for pos in [a, b]:
                call('setblock', *pos, 'minecraft:air')
            call('save')
            stop()
            start()
            for pos in [b, a]:
                call('setblock', *pos, args.block)
            assert total([a], 'minecraft:gold_ingot') == 11
            assert total([b], 'minecraft:gold_ingot') == 11
            call('save')
            stop()
            start()
            assert total([a], 'minecraft:gold_ingot') == 11
            assert total([b], 'minecraft:gold_ingot') == 11
            stop()
            return {'mode': mode, 'status': 'passed', 'checks': [
                'immediate insertion across endpoints', 'last item and two real hoppers',
                'partial and full hopper extraction', 'idle does not restore items',
                'last endpoint removed then world reopened', 'loaded endpoints survive world reopen'],
                'diamond_total': 18, 'gold_in_canonical_channel': 11}
        finally:
            if process is not None:
                try:
                    call('quit')
                    process.wait(timeout=15)
                except (AssertionError, subprocess.TimeoutExpired):
                    process.kill()
                    process.wait()
            args.out.parent.mkdir(parents=True, exist_ok=True)
            for index, log in enumerate(logs):
                (args.out.parent / f'{args.out.stem}-{mode}-{index}.log').write_text(
                    log.read_text(errors='replace') if log.exists() else '')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hmod', type=Path, required=True)
    parser.add_argument('--block', required=True)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--mode', choices=['in-frame', 'server-thread', 'both'], default='both')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    for field in ['hmod', 'bin_dir', 'resources', 'out']:
        setattr(args, field, getattr(args, field).resolve())
    modes = ['in-frame', 'server-thread'] if args.mode == 'both' else [args.mode]
    result = {'block': args.block, 'results': [run_case(args, mode) for mode in modes]}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()

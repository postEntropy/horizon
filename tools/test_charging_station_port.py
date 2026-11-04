#!/usr/bin/env python3
"""Real fuel, item charging, hopper components and reopen in both game paths.

Inputs are scratch hmods: Charging Gadgets with resources, and test_machine
with its registered battery item. Never use a player's world or install path.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def run_case(args, mode):
    with tempfile.TemporaryDirectory(prefix='horizon-charge-game-') as folder:
        scratch = Path(folder)
        env = dict(os.environ, HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                   HORIZON_TELEMETRY_URL='off', RAYON_NUM_THREADS='6')
        for name in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            env[name] = tempfile.mkdtemp(prefix=name.lower(), dir=scratch)
        socket = scratch / 'ctl.sock'
        ctl = [str(args.bin_dir / 'horizon-ctl'), '--socket', str(socket)]
        process = None
        logs = []

        def call(*words):
            print(mode, *words, flush=True)
            result = subprocess.run(ctl + list(map(str, words)), env=env, cwd=scratch,
                                    capture_output=True, text=True, timeout=180)
            lines = result.stdout.strip().splitlines()
            reply = json.loads(lines[-1]) if lines else {'error': result.stderr.strip()}
            if result.returncode or 'error' in reply:
                raise AssertionError((words, reply))
            return reply

        def command(text):
            return next(iter(call('command', text).get('chat', [])), '')

        def start():
            nonlocal process
            log = scratch / f'game-{len(logs)}.log'
            logs.append(log)
            argv = [str(args.bin_dir / 'horizon-client'), '--resources', str(args.resources),
                    '--dev-mod', str(args.charger), '--dev-mod', str(args.provider),
                    '--world', 'ChargingFixture', '--test-terrain', '--creative',
                    '--render-distance', '2', '--offscreen', '--width', '320', '--height', '240',
                    '--control-socket', str(socket)]
            if mode == 'in-frame':
                argv.append('--in-frame')
            with log.open('w') as stream:
                process = subprocess.Popen(argv, cwd=scratch, env=env, stdout=stream, stderr=stream)
            deadline = time.monotonic() + 180
            while not socket.exists():
                if process.poll() is not None or time.monotonic() > deadline:
                    raise AssertionError(log.read_text(errors='replace')[-6000:])
                time.sleep(0.2)
            call('wait-ready', '180')
            command('tick freeze')

        def stop():
            nonlocal process
            call('quit')
            process.wait(timeout=45)
            assert process.returncode == 0
            process = None
            socket.unlink(missing_ok=True)

        def step(ticks):
            command(f'tick step {ticks}')
            time.sleep(ticks / 20 + 1)

        def item_energy(pos):
            data = command(f'data get block {pos} Items')
            assert 'test_machine:battery' in data, data
            match = re.search(r'"?horizon:energy"?: (\d+)[lL]', data)
            assert match, data
            return int(match[1])

        try:
            start()
            command('tp @s 1 102 4')
            call('fill', -4, 98, -4, 4, 98, 4, 'minecraft:stone')
            call('setblock', 0, 100, 0, 'charginggadgets:charging_station')
            command('item replace block 0 100 0 container.1 with test_machine:battery')
            # No resources: no charge. Then real vanilla furnace fuel.
            step(5)
            data = command('data get block 0 100 0 Items')
            assert 'horizon:energy' not in data, data
            command('item replace block 0 100 0 container.0 with minecraft:coal')
            step(80)
            assert item_energy('0 100 0') == 2500
            # Item's receive capacity blocks further charging.
            step(20)
            assert item_energy('0 100 0') == 2500
            call('setblock', 1, 99, 0, 'minecraft:chest')
            call('setblock', 0, 99, 0, 'minecraft:hopper[facing=east]')
            step(40)
            assert item_energy('1 99 0') == 2500
            stations = call('container', 0, 100, 0)['slots']
            assert not any(stack and stack['item'] == 'test_machine:battery' for stack in stations)
            outputs = call('container', 1, 99, 0)['slots']
            assert sum(stack['count'] for stack in outputs if stack and stack['item'] == 'test_machine:battery') == 1
            call('save')
            stop()
            start()
            assert item_energy('1 99 0') == 2500
            stop()
            return {'mode': mode, 'status': 'passed', 'battery_count': 1, 'stored_fe': 2500,
                    'checks': ['no fuel does not charge', 'real coal fuel', 'full item blocks more charge',
                               'hopper preserves charged ItemStack', 'no duplicate battery', 'world reopen preserves charge']}
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
                (args.out.parent / f'{args.out.stem}-{mode}-{index}.log').write_text(log.read_text(errors='replace'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--charger', type=Path, required=True)
    parser.add_argument('--provider', type=Path, required=True)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--mode', choices=['in-frame', 'server-thread', 'both'], default='both')
    args = parser.parse_args()
    for name in ['charger', 'provider', 'bin_dir', 'resources', 'out']:
        setattr(args, name, getattr(args, name).resolve())
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:6])
    os.nice(max(0, 10 - os.getpriority(os.PRIO_PROCESS, 0)))
    modes = ['in-frame', 'server-thread'] if args.mode == 'both' else [args.mode]
    result = {'scope': 'actual game; test battery provider; not certification of other tool mods',
              'results': [run_case(args, mode) for mode in modes]}
    args.out.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()

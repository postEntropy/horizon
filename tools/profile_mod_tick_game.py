#!/usr/bin/env python3
"""Profile the real mod block-entity loop in both single-player paths."""
import argparse
import json
import os
from pathlib import Path
import re
import statistics
import time

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 100, 0)
LINE = re.compile(r"mod_tick_profile dimension=(\S+) machines=(\d+) create_ns=(\d+) view_copy_ns=(\d+) capability_ns=(\d+) callback_ns=(\d+) serialize_ns=(\d+) total_ns=(\d+)")
FIELDS = ('create_ns', 'view_copy_ns', 'capability_ns', 'callback_ns', 'serialize_ns', 'total_ns')


def region_points(count):
    points = [POS]
    if count >= 100:
        points.extend((x, y, z) for x in range(2, 5) for y in range(100, 111) for z in range(2, 5))
    if count >= 1000:
        points.extend((x, y, z) for x in range(-10, 0) for y in range(100, 109) for z in range(-5, 5))
    return points


def summarize(rows):
    result = {}
    for field in FIELDS:
        values = [row[field] for row in rows]
        ordered = sorted(values)
        result[field] = {
            'p50_ns': statistics.median(values),
            'p95_ns': ordered[min(len(ordered) - 1, int(len(ordered) * .95))],
            'max_ns': max(values),
        }
    return result


def read_rows(log_path, expected, minimum, timeout=180):
    deadline = time.monotonic() + timeout
    rows = []
    while time.monotonic() < deadline:
        rows = []
        for match in LINE.finditer(log_path.read_text(errors='replace')):
            row = dict(zip(('dimension', 'machines', *FIELDS), match.groups()))
            row['machines'] = int(row['machines'])
            for field in FIELDS:
                row[field] = int(row[field])
            if row['machines'] == expected:
                rows.append(row)
        if len(rows) >= minimum:
            break
        time.sleep(1)
    if len(rows) < minimum:
        raise AssertionError(f'expected {minimum} samples for {expected} machines, found {len(rows)}')
    return rows[-minimum:]


def process_memory_mib(pid):
    values = {}
    for line in Path(f'/proc/{pid}/status').read_text().splitlines():
        if line.startswith(('VmRSS:', 'VmHWM:')):
            name, value, *_ = line.split()
            values[name[:-1].lower()] = round(int(value) / 1024, 2)
    return values


def run_mode(args, mode):
    os.environ['HORIZON_MOD_TICK_PROFILE'] = '1'
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('tp @s 1 102 4')
        game.call('fill', -4, 98, -4, 4, 98, 4, 'minecraft:stone')
        game.call('setblock', *POS, 'actuallyadditions:crusher')
        samples = []
        for count in (1, 100, 1000):
            if count == 100:
                game.call('fill', 2, 100, 2, 4, 110, 4, 'actuallyadditions:crusher')
            elif count == 1000:
                game.call('fill', -10, 100, -5, -1, 108, 4, 'actuallyadditions:crusher')
            points = region_points(count)
            game.step(40)
            idle = read_rows(game.logs[0], count, 40)
            samples.append({'machines': count, 'state': 'idle', 'sample_count': len(idle),
                            'memory_mib': process_memory_mib(game.process.pid), 'stats': summarize(idle)})
            for x, y, z in points:
                game.command(f'item replace block {x} {y} {z} container.0 with minecraft:iron_ore 1')
            game.step(40)
            active = read_rows(game.logs[0], count, 40)
            samples.append({'machines': count, 'state': 'input-present-no-energy',
                            'sample_count': len(active), 'memory_mib': process_memory_mib(game.process.pid),
                            'stats': summarize(active)})
            for x, y, z in points:
                game.command(f'item replace block {x} {y} {z} container.0 with air')
            game.step(1)
        game.stop()
        return {'mode': mode, 'measurements': samples}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/actuallyadditions-with-carry.hmod')
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/mod-tick-profile-game.json')
    parser.add_argument('--modes', nargs='+', choices=('in-frame', 'server-thread'),
                        default=('in-frame', 'server-thread'))
    args = parser.parse_args()
    for name in ('hmod', 'bin_dir', 'resources', 'out'):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')
    results = [run_mode(args, mode) for mode in args.modes]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'scenario': 'Actually Additions crusher; empty vs input present without FE',
                                    'samples_per_case': 40, 'cases': results}, indent=2) + '\n')
    print(f'Profile written to {args.out}')


if __name__ == '__main__':
    main()

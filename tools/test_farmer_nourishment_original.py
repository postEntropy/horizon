#!/usr/bin/env python3
"""Black-box the original Nourishment food path and its expiration."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_JAR = Path('/home/leonardo/modpacks/Instances/All the Mods 10 - ATM10/mods/FarmersDelight-1.21.1-1.3.4.jar')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jar', type=Path, default=DEFAULT_JAR)
    parser.add_argument('--raw', type=Path, default=ROOT / '.local/atm10-resource-validation/nourishment-consumption-raw.json')
    parser.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/nourishment-consumption.json')
    args = parser.parse_args()
    args.jar = args.jar.resolve()
    args.raw = args.raw.resolve()
    args.out = args.out.resolve()
    if not args.jar.is_file():
        parser.error(f'original mod jar not found: {args.jar}')
    if not args.raw.is_relative_to(ROOT / '.local') or not args.out.is_relative_to(ROOT / '.local'):
        parser.error('raw and report outputs must stay under repository .local/')
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')

    scenarios = [
        {'name': 'control_without_effect', 'effect': '', 'duration': 0,
         'ticks': 100, 'food': 10, 'saturation': 0, 'exhaustion_per_tick': 0.2},
        {'name': 'consume_cooked_rice_and_expire', 'effect': 'farmersdelight:nourishment',
         'duration': 600, 'ticks': 650, 'food': 4, 'saturation': 0,
         'exhaustion_per_tick': 0.2, 'eat': 'farmersdelight:cooked_rice', 'count': 2},
    ]
    args.raw.parent.mkdir(parents=True, exist_ok=True)
    scenario_path = args.raw.with_name(args.raw.stem + '-scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    args.raw.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-nourishment-original-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'nourishment-consume-{uuid.uuid4().hex}', [args.jar], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(args.raw),
            'horizon.oracle.food': str(scenario_path),
        }, timeout=90)
    if not args.raw.exists() or args.raw.stat().st_size < 100:
        raise RuntimeError(f'original oracle produced no trace:\n{log[-4000:]}')

    rows = json.loads(args.raw.read_text())['scenarios']
    control, consumed = rows
    assert not control['effect_registered']
    assert consumed['effect_registered']
    eaten = consumed['consumed']
    assert eaten['remaining_count'] == 1, eaten
    assert (eaten['food_after'], eaten['saturation_after']) == (10, 4.8), eaten
    assert eaten['active_effects'] == [{
        'id': 'farmersdelight:nourishment', 'duration': 600, 'amplifier': 0,
        'ambient': False, 'visible': False, 'show_icon': False,
    }], eaten
    trace = {entry['tick']: entry for entry in consumed['trace']}
    assert trace[599]['active_effects'] == 1 and trace[599]['effect_duration'] == 1, trace[599]
    assert trace[600]['active_effects'] == 0, trace[600]
    assert trace[599]['food'] == 10 and trace[599]['saturation'] == 4.8, trace[599]
    assert trace[620]['saturation'] < trace[599]['saturation'], (trace[599], trace[620])

    report = {
        'status': 'passed',
        'scope': 'original mod public-API item completion, server effect ticks, expiration and post-expiration food data',
        'jar_sha256': hashlib.sha256(args.jar.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(args.raw.read_bytes()).hexdigest(),
        'raw_path': str(args.raw.relative_to(ROOT)),
        'scenario_path': str(scenario_path.relative_to(ROOT)),
        'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'nice': os.getpriority(os.PRIO_PROCESS, 0),
        'oracle_wall_seconds': seconds,
        'consumption': eaten,
        'samples': {str(t): trace[t] for t in [0, 1, 599, 600, 601, 620, 640, 650]},
        'limits': ['uses FakePlayer.finishUsingItem and public MobEffectInstance/FoodData ticks; does not exercise real input timing'],
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'original cooked_rice consumption and Nourishment expiration passed ({seconds:.2f}s)')


if __name__ == '__main__':
    main()

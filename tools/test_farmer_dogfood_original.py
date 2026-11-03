#!/usr/bin/env python3
"""Observe DogFoodItem player consumption and wolf interaction in the original."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
JAR = Path('/home/leonardo/modpacks/Instances/All the Mods 10 - ATM10/mods/FarmersDelight-1.21.1-1.3.4.jar')
RAW = ROOT / '.local/atm10-resource-validation/dog-food-original-raw.json'
REPORT = ROOT / '.local/atm10-resource-validation/dog-food-original.json'


def main():
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        raise SystemExit('run with taskset -c 0-5 nice -n 10')
    scenarios = [
        {'name': 'tamed_wounded_wolf', 'eat': 'farmersdelight:dog_food', 'count': 1,
         'target_wolf': True, 'wolf_health': 8.0, 'effect': '', 'duration': 0,
         'ticks': 100, 'food': 4, 'saturation': 0},
        {'name': 'untamed_wounded_wolf', 'eat': 'farmersdelight:dog_food', 'count': 1,
         'target_wolf': False, 'wolf_health': 8.0, 'effect': '', 'duration': 0,
         'ticks': 100, 'food': 4, 'saturation': 0},
        {'name': 'tamed_full_health_wolf', 'eat': 'farmersdelight:dog_food', 'count': 1,
         'target_wolf': True, 'wolf_health': 20.0, 'effect': '', 'duration': 0,
         'ticks': 100, 'food': 4, 'saturation': 0},
    ]
    scenario_path = RAW.with_name(RAW.stem + '-scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-dog-food-original-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'dog-food-{uuid.uuid4().hex}', [JAR], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(RAW), 'horizon.oracle.food': str(scenario_path),
        }, timeout=120)
    if not RAW.exists() or RAW.stat().st_size < 300:
        raise RuntimeError(f'original oracle produced no trace:\n{log[-4000:]}')
    rows = json.loads(RAW.read_text())['scenarios']
    assert len(rows) == len(scenarios)
    for row in rows:
        assert 'wolf_interaction' in row, row
    REPORT.write_text(json.dumps({
        'status': 'observed', 'jar_sha256': hashlib.sha256(JAR.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(RAW.read_bytes()).hexdigest(), 'raw_path': str(RAW.relative_to(ROOT)),
        'scenario_path': str(scenario_path.relative_to(ROOT)), 'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'nice': os.getpriority(os.PRIO_PROCESS, 0), 'oracle_wall_seconds': seconds,
        'method': 'public Item.interactLivingEntity, Wolf.tame and Item.finishUsingItem APIs; FakePlayer',
        'scenarios': rows,
    }, indent=2) + '\n')
    print(f'original DogFoodItem player/wolf behavior observed ({seconds:.2f}s)')


if __name__ == '__main__':
    main()

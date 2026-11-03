#!/usr/bin/env python3
"""Observe cooking-pot ingredient remainders in the original mod, black-box only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
FACTS = ROOT / '.local/atm10-resource-validation/farmer-container-facts.json'
TARGETS = {'farmersdelight:milk_bottle', 'farmersdelight:tomato_sauce'}


def scenarios():
    facts = json.loads(FACTS.read_text())
    result = []
    for recipe_id, recipe in sorted(facts['recipes'].items()):
        if recipe.get('type') != 'farmersdelight:cooking':
            continue
        groups = recipe['ingredients']
        target_slots = [(i, next((item for item in group if item in TARGETS), None))
                        for i, group in enumerate(groups)]
        target_slots = [(i, item) for i, item in target_slots if item]
        for target_slot, target in target_slots:
            selected = [next((item for item in group if item == target), group[0])
                        if i == target_slot else group[0]
                        for i, group in enumerate(groups)]
            slots = {str(i): [item, 1] for i, item in enumerate(selected)}
            scenario = {'name': recipe_id + '/' + target.rsplit(':', 1)[-1],
                        'recipe': recipe_id, 'result': recipe['result'],
                        'target_ingredient': target, 'target_slot': target_slot,
                        'block': 'farmersdelight:cooking_pot', 'menu_slots': 9,
                        'slots': slots,
                        'neighbors': [{'x': 0, 'y': -1, 'z': 0,
                                       'block': 'minecraft:magma_block'}],
                        'fields': ['cookingProgress'], 'ticks': 220}
            result.append(scenario)
            # A full/incompatible served-output slot must not roll back cooking
            # or duplicate/erase the ingredient remainder.
            blocked = dict(scenario)
            blocked['name'] += '/blocked_served_output'
            blocked['slots'] = dict(slots)
            blocked['slots']['8'] = ['minecraft:diamond', 64]
            result.append(blocked)
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--jar', type=Path, required=True)
    p.add_argument('--raw', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    a.jar, a.raw, a.out = (x.resolve() for x in (a.jar, a.raw, a.out))
    if not a.raw.is_relative_to(ROOT / '.local'):
        p.error('--raw must be under repository .local/')
    a.raw.parent.mkdir(parents=True, exist_ok=True)
    scenario_path = a.raw.with_suffix('.scenarios.json')
    case_list = scenarios()
    scenario_path.write_text(json.dumps(case_list, indent=2) + '\n')
    a.raw.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-farmer-remainders-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance('farmer-remainders-' + uuid.uuid4().hex,
                                    [a.jar], [exporter])
        seconds, log = oracle.start(home, run, {'horizon.oracle.out': a.raw,
                                                'horizon.oracle.machines': scenario_path},
                                    timeout=120)
    if not a.raw.exists():
        raise RuntimeError(f'oracle produced no trace: {run}\n{log[-3000:]}')
    traces = json.loads(a.raw.read_text())
    report = {'status': 'observed', 'scope': 'original Farmer cooking pot via NeoForge public menu/world APIs; no mod source/class inspection',
              'jar_sha256': hashlib.sha256(a.jar.read_bytes()).hexdigest(),
              'raw_sha256': hashlib.sha256(a.raw.read_bytes()).hexdigest(),
              'scenario_sha256': hashlib.sha256(scenario_path.read_bytes()).hexdigest(),
              'raw_path': str(a.raw.relative_to(ROOT)), 'scenario_path': str(scenario_path.relative_to(ROOT)),
              'cpu_affinity': sorted(os.sched_getaffinity(0)),
              'nice': os.getpriority(os.PRIO_PROCESS, 0), 'oracle_wall_seconds': seconds,
              'traces': traces}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'observed {len(traces)} original cooking-pot scenarios in {seconds:.2f}s')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Observe every loaded cooking recipe's container/service behavior in original Farmer's Delight."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
RECIPES = ROOT / '.local/atm10-resource-validation/farmer-regenerated-tags-only-recipes.json'
FACTS = ROOT / '.local/atm10-resource-validation/farmer-container-facts.json'
SERVING_OVERRIDES = {
    'minecraft:beetroot_soup': 'minecraft:bowl',
    'minecraft:mushroom_stew': 'minecraft:bowl',
    'minecraft:rabbit_stew': 'minecraft:bowl',
    'farmersdelight:stuffed_pumpkin_block': 'minecraft:pumpkin',
}


def make_scenarios():
    recipes = json.loads(RECIPES.read_text())
    items = json.loads(FACTS.read_text())['items']
    scenarios = []
    for recipe in recipes:
        if recipe['kind'] != 'farmersdelight:cooking':
            continue
        name = recipe['id']
        inputs = {str(i): [group['item_candidates'][0], group['count']]
                  for i, group in enumerate(recipe['inputs'])}
        output = recipe['outputs'][0]['item']
        # The public item crafting remainder is the observed serving container
        # for most Farmer foods. Vanilla soups and the stuffed pumpkin are
        # exceptions; exercise the port's current explicit serving candidates
        # against the original runtime rather than treating those as facts.
        container = (items.get(output, {}).get('crafting_remainder')
                     or SERVING_OVERRIDES.get(output))
        ticks = recipe['ticks'] + 20
        neighbors = [{'x': 0, 'y': -1, 'z': 0, 'block': 'minecraft:magma_block'}]
        cases = [('container_available', dict(inputs),
                  [{'tick': recipe['ticks'], 'slot': 7,
                    'stack': [container, 1]}] if container else []),
                 ('blocked_output', {**inputs, '8': ['minecraft:diamond', 64]},
                  [{'tick': recipe['ticks'], 'slot': 7,
                    'stack': [container, 1]}] if container else [])]
        if container:
            cases.append(('missing_container', dict(inputs), []))
        for label, slots, events in cases:
            scenarios.append({'name': f'{name}/{label}', 'recipe': name,
                              'result': output, 'container': container,
                              'case': label, 'block': 'farmersdelight:cooking_pot',
                              'menu_slots': 9, 'slots': slots,
                              'neighbors': neighbors, 'events': events,
                              'ticks': ticks})
    return scenarios


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
    cases = make_scenarios()
    scenario_path.write_text(json.dumps(cases, indent=2) + '\n')
    a.raw.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-farmer-cooking-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance('farmer-cooking-' + uuid.uuid4().hex,
                                    [a.jar], [exporter])
        seconds, log = oracle.start(home, run, {'horizon.oracle.out': a.raw,
                                                'horizon.oracle.machines': scenario_path},
                                    timeout=180)
    if not a.raw.exists():
        raise RuntimeError(f'oracle produced no trace: {run}\n{log[-3000:]}')
    trace = json.loads(a.raw.read_text())
    report = {'status': 'observed',
              'scope': '28 loaded cooking recipes; original mod through public NeoForge menu/world APIs; no mod source/class inspection',
              'jar_sha256': hashlib.sha256(a.jar.read_bytes()).hexdigest(),
              'raw_sha256': hashlib.sha256(a.raw.read_bytes()).hexdigest(),
              'scenario_sha256': hashlib.sha256(scenario_path.read_bytes()).hexdigest(),
              'raw_path': str(a.raw.relative_to(ROOT)),
              'scenario_path': str(scenario_path.relative_to(ROOT)),
              'cpu_affinity': sorted(os.sched_getaffinity(0)),
              'nice': os.getpriority(os.PRIO_PROCESS, 0),
              'oracle_wall_seconds': seconds, 'traces': trace}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'observed {len(trace)} recipe-service scenarios in {seconds:.2f}s')


if __name__ == '__main__':
    main()

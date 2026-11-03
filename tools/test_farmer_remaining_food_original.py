#!/usr/bin/env python3
"""Black-box the remaining special Farmer's Delight foods via public APIs."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
JAR = Path('/home/leonardo/modpacks/Instances/All the Mods 10 - ATM10/mods/FarmersDelight-1.21.1-1.3.4.jar')
IDS = [
    'farmersdelight:apple_cider', 'farmersdelight:bone_broth',
    'farmersdelight:dog_food', 'farmersdelight:melon_popsicle',
    'farmersdelight:tomato_sauce',
]
AUDIT = ROOT / '.local/atm10-resource-validation/farmersdelight-l4-audit.json'
RAW = ROOT / '.local/atm10-resource-validation/remaining-food-group-raw.json'
REPORT = ROOT / '.local/atm10-resource-validation/remaining-food-group.json'


def main():
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        raise SystemExit('run with taskset -c 0-5 nice -n 10')
    audit = json.loads(AUDIT.read_text())
    items = {item['id']: item for item in audit['items'] if item['id'] in IDS}
    facts = json.loads((ROOT / '.local/atm10-resource-validation/farmer-food-regenerated/oracle/facts.json').read_text())['items']
    scenarios = []
    for item_id in IDS:
        item = items[item_id]
        duration = item['effects'][0]['duration'] if item['effects'] else 0
        scenarios.append({
            'name': item_id, 'effect': item['effects'][0]['id'] if item['effects'] else '',
            'duration': duration, 'ticks': max(100, duration) + 40,
            'food': 4, 'saturation': 0, 'exhaustion_per_tick': 0.2,
            'eat': item_id, 'count': 1,
        })
    scenario_path = RAW.with_name(RAW.stem + '-scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-fd-special-foods-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'fd-special-foods-{uuid.uuid4().hex}', [JAR], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(RAW), 'horizon.oracle.food': str(scenario_path),
        }, timeout=180)
    if not RAW.exists() or RAW.stat().st_size < 500:
        raise RuntimeError(f'original oracle produced no trace:\n{log[-4000:]}')
    rows = json.loads(RAW.read_text())['scenarios']
    summaries = []
    for item_id, item, row in zip(IDS, [items[i] for i in IDS], rows, strict=True):
        facts_food = facts[item_id]['food']
        consumed = row['consumed']
        expected_food = min(20, 4 + facts_food['nutrition'])
        expected_saturation = min(expected_food, facts_food['saturation'])
        assert consumed['food_after'] == expected_food, (item_id, consumed, facts_food)
        assert abs(consumed['saturation_after'] - expected_saturation) < 1e-5, (item_id, consumed, facts_food)
        expected_effects = [{
            'id': effect['id'], 'duration': effect['duration'], 'amplifier': effect['amplifier'],
            'ambient': effect['ambient'], 'visible': effect['visible'], 'show_icon': effect['show_icon'],
        } for effect in item['effects']]
        assert consumed['active_effects'] == expected_effects, (item_id, consumed, expected_effects)
        summaries.append({
            'id': item_id, 'class': item['class'], 'food': consumed['food_after'],
            'saturation': consumed['saturation_after'], 'duration': item['eat_seconds'],
            'effects': consumed['active_effects'], 'returned_item': consumed['remaining_item'],
            'returned_count': consumed['remaining_count'],
        })
    REPORT.write_text(json.dumps({
        'status': 'passed', 'items': summaries,
        'jar_sha256': hashlib.sha256(JAR.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(RAW.read_bytes()).hexdigest(), 'raw_path': str(RAW.relative_to(ROOT)),
        'scenario_path': str(scenario_path.relative_to(ROOT)), 'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'nice': os.getpriority(os.PRIO_PROCESS, 0), 'oracle_wall_seconds': seconds,
        'method': 'public Item.finishUsingItem, MobEffectInstance and FoodData tick APIs; FakePlayer',
        'limits': ['DogFoodItem player consumption only; wolf-target interaction requires a separate observation'],
    }, indent=2) + '\n')
    print(f'remaining special foods passed public-API consumption: {len(rows)} ({seconds:.2f}s)')


if __name__ == '__main__':
    main()

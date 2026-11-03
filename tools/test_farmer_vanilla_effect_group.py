#!/usr/bin/env python3
"""Black-box actual consumption for Farmer's Delight vanilla-effect foods."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
JAR = Path('/home/leonardo/modpacks/Instances/All the Mods 10 - ATM10/mods/FarmersDelight-1.21.1-1.3.4.jar')
AUDIT = ROOT / '.local/atm10-resource-validation/farmersdelight-l4-audit.json'
RAW = ROOT / '.local/atm10-resource-validation/vanilla-effect-food-group-raw.json'
REPORT = ROOT / '.local/atm10-resource-validation/vanilla-effect-food-group.json'


def main():
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        raise SystemExit('run with taskset -c 0-5 nice -n 10')
    audit = json.loads(AUDIT.read_text())
    facts = json.loads((ROOT / '.local/atm10-resource-validation/farmer-food-regenerated/oracle/facts.json').read_text())['items']
    foods = [item for item in audit['items'] if
             item['class'] == 'vectorwing.farmersdelight.common.item.ConsumableItem'
             and item['effects'] and all(e['id'].startswith('minecraft:') for e in item['effects'])]
    scenarios = []
    scenario_expectations = []
    for item in foods:
        effect = item['effects'][0]
        samples = 20 if effect['probability'] < 1 else 1
        for sample in range(samples):
            name = f"{item['id']}#{sample + 1}"
            scenarios.append({
                'name': name, 'effect': effect['id'], 'duration': effect['duration'],
                'ticks': effect['duration'] + 40, 'food': 4, 'saturation': 0,
                'exhaustion_per_tick': 0.2, 'eat': item['id'], 'count': 1,
            })
            scenario_expectations.append((item, effect, sample + 1))
    scenario_path = RAW.with_name(RAW.stem + '-scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-vanilla-effects-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'vanilla-effect-foods-{uuid.uuid4().hex}', [JAR], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(RAW), 'horizon.oracle.food': str(scenario_path),
        }, timeout=180)
    if not RAW.exists() or RAW.stat().st_size < 1000:
        raise RuntimeError(f'original oracle produced no trace:\n{log[-4000:]}')
    rows = json.loads(RAW.read_text())['scenarios']
    if len(rows) != len(scenario_expectations):
        raise AssertionError(f'expected {len(scenario_expectations)} traces, got {len(rows)}')
    summaries = {}
    for (item, effect, sample), row in zip(scenario_expectations, rows, strict=True):
        consumed = row['consumed']
        assert consumed['remaining_count'] in (0, 1), (item['id'], consumed)
        expected_food = min(20, 4 + facts[item['id']]['food']['nutrition'])
        expected_saturation = min(expected_food, facts[item['id']]['food']['saturation'])
        assert consumed['food_after'] == expected_food, (item['id'], expected_food, consumed)
        assert abs(consumed['saturation_after'] - expected_saturation) < 1e-5, (item['id'], expected_saturation, consumed)
        assert consumed['active_effects'] in ([], [{
            'id': effect['id'], 'duration': effect['duration'], 'amplifier': effect['amplifier'],
            'ambient': effect['ambient'], 'visible': effect['visible'], 'show_icon': effect['show_icon'],
        }]), (item['id'], consumed)
        proc = bool(consumed['active_effects'])
        trace = {entry['tick']: entry for entry in row['trace']}
        if effect['probability'] == 1:
            assert proc, (item['id'], consumed)
        if proc:
            assert trace[effect['duration'] - 1]['active_effects'] == 1, (item['id'], trace[effect['duration'] - 1])
            assert trace[effect['duration']]['active_effects'] == 0, (item['id'], trace[effect['duration']])
        after = trace[effect['duration'] + 40]
        summaries.setdefault(item['id'], {
            'class': item['class'], 'effect': effect, 'samples': [],
        })['samples'].append({
            'sample': sample, 'proc': proc, 'food': consumed['food_after'],
            'saturation': consumed['saturation_after'], 'remainder': consumed['remaining_item'],
            'remainder_count': consumed['remaining_count'],
            'post_expiration': {'food': after['food'], 'saturation': after['saturation']},
        })
    for item in foods:
        effects = summaries[item['id']]['samples']
        probability = item['effects'][0]['probability']
        if probability < 1:
            assert any(row['proc'] for row in effects) and any(not row['proc'] for row in effects), (item['id'], effects)
    REPORT.write_text(json.dumps({
        'status': 'passed', 'observed_items': len(foods), 'jar_sha256': hashlib.sha256(JAR.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(RAW.read_bytes()).hexdigest(), 'raw_path': str(RAW.relative_to(ROOT)),
        'scenario_path': str(scenario_path.relative_to(ROOT)), 'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'nice': os.getpriority(os.PRIO_PROCESS, 0), 'oracle_wall_seconds': seconds,
        'method': 'public Item.finishUsingItem, MobEffectInstance and FoodData tick APIs; FakePlayer',
        'items': summaries,
    }, indent=2) + '\n')
    print(f'original vanilla-effect foods passed: {len(foods)} ({seconds:.2f}s)')


if __name__ == '__main__':
    main()

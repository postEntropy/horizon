#!/usr/bin/env python3
"""Black-box actual consumption for foods carrying Farmer's Delight Nourishment."""
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
RAW = ROOT / '.local/atm10-resource-validation/nourishment-group-raw.json'
REPORT = ROOT / '.local/atm10-resource-validation/nourishment-group.json'


def main():
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        raise SystemExit('run with taskset -c 0-5 nice -n 10')
    audit = json.loads(AUDIT.read_text())
    foods = [item for item in audit['items'] if any(
        effect['id'] == 'farmersdelight:nourishment' for effect in item['effects'])
        and item['class'] == 'vectorwing.farmersdelight.common.item.ConsumableItem']
    scenarios = []
    for item in foods:
        effect = next(e for e in item['effects'] if e['id'] == 'farmersdelight:nourishment')
        scenarios.append({
            'name': item['id'], 'effect': effect['id'], 'duration': effect['duration'],
            'ticks': effect['duration'] + 40, 'food': 4, 'saturation': 0,
            'exhaustion_per_tick': 0.2, 'eat': item['id'], 'count': 1,
        })
    scenario_path = RAW.with_name(RAW.stem + '-scenarios.json')
    scenario_path.write_text(json.dumps(scenarios, indent=2) + '\n')
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='horizon-nourishment-group-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'nourishment-group-{uuid.uuid4().hex}', [JAR], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(RAW), 'horizon.oracle.food': str(scenario_path),
        }, timeout=180)
    if not RAW.exists() or RAW.stat().st_size < 1000:
        raise RuntimeError(f'original oracle produced no trace:\n{log[-4000:]}')
    rows = json.loads(RAW.read_text())['scenarios']
    if len(rows) != len(foods):
        raise AssertionError(f'expected {len(foods)} foods, got {len(rows)}')
    summaries = []
    for item, row in zip(foods, rows, strict=True):
        expected = next(e for e in item['effects'] if e['id'] == 'farmersdelight:nourishment')
        consumed = row['consumed']
        assert (consumed['remaining_item'], consumed['remaining_count']) == ('minecraft:bowl', 1), (item['id'], consumed)
        assert consumed['active_effects'] == [{
            'id': 'farmersdelight:nourishment', 'duration': expected['duration'],
            'amplifier': expected['amplifier'], 'ambient': expected['ambient'],
            'visible': expected['visible'], 'show_icon': expected['show_icon'],
        }], (item['id'], consumed)
        trace = {entry['tick']: entry for entry in row['trace']}
        assert trace[expected['duration'] - 1]['active_effects'] == 1, item['id']
        assert trace[expected['duration']]['active_effects'] == 0, item['id']
        assert trace[expected['duration'] - 1]['food'] == consumed['food_after'], item['id']
        assert trace[expected['duration'] - 1]['saturation'] == consumed['saturation_after'], item['id']
        after = trace[expected['duration'] + 40]
        assert (after['food'], after['saturation']) != (
            trace[expected['duration'] - 1]['food'], trace[expected['duration'] - 1]['saturation']), item['id']
        summaries.append({
            'id': item['id'], 'class': item['class'], 'eat_seconds': item['eat_seconds'],
            'duration': expected['duration'], 'food': consumed['food_after'],
            'saturation': consumed['saturation_after'], 'remaining_count': consumed['remaining_count'],
            'remaining_item': consumed['remaining_item'],
            'post_expiration': {'food': after['food'], 'saturation': after['saturation']},
        })
    REPORT.write_text(json.dumps({
        'status': 'passed', 'observed_items': len(rows), 'jar_sha256': hashlib.sha256(JAR.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(RAW.read_bytes()).hexdigest(), 'raw_path': str(RAW.relative_to(ROOT)),
        'scenario_path': str(scenario_path.relative_to(ROOT)), 'cpu_affinity': sorted(os.sched_getaffinity(0)),
        'nice': os.getpriority(os.PRIO_PROCESS, 0), 'oracle_wall_seconds': seconds,
        'method': 'public Item.finishUsingItem, MobEffectInstance and FoodData tick APIs; FakePlayer; 0.2 exhaustion/tick',
        'items': summaries,
    }, indent=2) + '\n')
    print(f'original Nourishment foods passed consumption and expiration: {len(rows)} ({seconds:.2f}s)')


if __name__ == '__main__':
    main()

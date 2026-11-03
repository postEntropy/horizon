#!/usr/bin/env python3
"""Observe real Mining Gadgets FE through NeoForge's public capability API."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid

import neoforge_oracle as oracle

ROOT = Path(__file__).resolve().parents[1]
JAR = Path('/home/leonardo/modpacks/Instances/All the Mods 10 - ATM10/mods/mininggadgets-1.18.8.jar')
IDS = [
    'mininggadgets:mininggadget_simple',
    'mininggadgets:mininggadget',
    'mininggadgets:mininggadget_fancy',
    'mininggadgets:upgrade_battery_1',
    'mininggadgets:upgrade_battery_2',
    'mininggadgets:upgrade_battery_3',
]
OUT = ROOT / '.local/atm10-resource-validation/mininggadgets-energy-original.json'
RAW = ROOT / '.local/atm10-resource-validation/mininggadgets-energy-original-raw.json'


def main():
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        raise SystemExit('run with taskset -c 0-5 nice -n 10')
    scenarios = [{'item': item, 'count': 1, 'request': 1_500_000} for item in IDS]
    scenarios.append({'item': 'mininggadgets:mininggadget', 'count': 2, 'request': 1_500_000})
    source = RAW.with_name(RAW.stem + '-scenarios.json')
    source.write_text(json.dumps(scenarios, indent=2) + '\n')
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix='mininggadgets-energy-oracle-') as scratch:
        for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance(f'mininggadgets-energy-{uuid.uuid4().hex}', [JAR], [exporter])
        seconds, log = oracle.start(home, run, {
            'horizon.oracle.out': str(RAW), 'horizon.oracle.energy': str(source),
        }, timeout=120)
    if not RAW.exists():
        raise RuntimeError(f'NeoForge oracle produced no result:\n{log[-4000:]}')
    results = json.loads(RAW.read_text())['scenarios']
    if len(results) != len(scenarios):
        raise AssertionError(results)
    OUT.write_text(json.dumps({
        'status': 'observed', 'jar_sha256': hashlib.sha256(JAR.read_bytes()).hexdigest(),
        'raw_sha256': hashlib.sha256(RAW.read_bytes()).hexdigest(),
        'raw_path': str(RAW.relative_to(ROOT)), 'scenario_path': str(source.relative_to(ROOT)),
        'cpu_affinity': sorted(os.sched_getaffinity(0)), 'nice': os.getpriority(os.PRIO_PROCESS, 0),
        'oracle_wall_seconds': seconds,
        'method': 'ItemStack.getCapability(Capabilities.EnergyStorage.ITEM), public IEnergyStorage queries and simulated/real transfers',
        'items': results,
        'limits': ['unupgraded gadgets; battery-upgrade combinations are not applied to a gadget'],
    }, indent=2) + '\n')
    print(f'observed public FE capabilities for {len(results)} real Mining Gadgets items ({seconds:.1f}s)')


if __name__ == '__main__':
    main()

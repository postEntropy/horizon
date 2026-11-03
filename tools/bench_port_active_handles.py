#!/usr/bin/env python3
"""Compare native Mod::tick_block_entities before/after using saved source snapshots.

This isolates guest Rust loops, without WASM crossings or game inventories. It
must not be reported as real gameplay throughput. The baseline directory holds
ars_technica/, botanypots/ and cabletiers/ copied before editing their tick loops.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
MODS = {
    'ars_technica': ('ArsTechnica', '''
        let saved = if operating { 1_000_000u32 } else { 0 }.to_le_bytes();
        port.create_block_entity(handle, "ars_technica:source_motor", &saved);
''', '''
        let port_impl::Machine::Motor(machine) = &port.machines[&handle] else { unreachable!() };
        serde_json::json!({"source_buffer":machine.source_buffer,"running":machine.running})
'''),
    'botanypots': ('BotanyPots', '''
        port.create_block_entity(handle, "botanypots:terracotta_botany_pot", &[]);
        if operating {
            let pot = port.pots.get_mut(&handle).unwrap();
            pot.set_soil("minecraft:dirt");
            pot.plant_crop("minecraft:wheat_seeds");
            pot.growth_ticks = pot.max_growth_ticks - 1;
        }
''', '''
        let pot = &port.pots[&handle];
        serde_json::json!({"growth_ticks":pot.growth_ticks,"output":pot.output_inventory})
'''),
    'cabletiers': ('CableTiers', '''
        port.create_block_entity(handle, "cabletiers:creative_importer", &[]);
        port.machines.get_mut(&handle).unwrap().enabled = operating;
''', '''
        let machine = &port.machines[&handle];
        serde_json::json!({"energy":machine.energy,"simulated_total_moved":machine.total_moved,"cooldown":machine.cooldown})
'''),
}
TEMPLATE = '''
use horizon_mod_sdk::Mod;
use port_impl::@TYPE@ as Port;
use std::{hint::black_box,time::Instant};
fn state(port: &Port, handle:u32) -> serde_json::Value { @STATE@ }
fn main() {
    let args:Vec<_> = std::env::args().skip(1).collect();
    let loaded:u32 = args[0].parse().unwrap();
    let active:u32 = args[1].parse().unwrap();
    let operating = args[2] == "active";
    let samples:usize = args[3].parse().unwrap();
    let mut port = Port::default();
    for index in 0..loaded {
        let handle = index * 3 + 11;
        @SETUP@
    }
    let handles:Vec<_> = (0..active).rev().map(|index| index * 3 + 11).collect();
    let sleeping:Vec<_> = (active..loaded).map(|index| { let h=index*3+11; (h,state(&port,h)) }).collect();
    let mut timings = Vec::new();
    let batch = 64;
    for tick in 0..(samples+10) {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(&mut port).tick_block_entities(black_box(&handles));
        }
        let us = start.elapsed().as_secs_f64()*1e6 / batch as f64;
        if tick >= 10 { timings.push(us); }
    }
    timings.sort_by(f64::total_cmp);
    let active_states:Vec<_> = handles.iter().map(|&h| (h,state(&port,h))).collect();
    let inactive_changed = sleeping.iter().filter(|(h,old)| state(&port,*h) != *old).count();
    let report=serde_json::json!({
        "scope":"native guest Rust tick only; not WASM or real game inventory transfers",
        "mod":"@MOD@","loaded":loaded,"received_handles":active,"operating":operating,
        "samples":samples,"warmup_batches":10,"ticks_per_batch":batch,"p50_us":timings[samples/2],
        "p95_us":timings[(samples-1)*95/100],"max_us":timings[samples-1],
        "active_states":active_states,"inactive_changed":inactive_changed
    });
    std::fs::write(&args[4],serde_json::to_vec(&report).unwrap()).unwrap();
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-dir', type=Path, required=True)
    parser.add_argument('--work', type=Path, default=ROOT / '.local/atm10-idle-work-validation/native-bench')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    work, out = args.work.resolve(), args.out.resolve()
    work.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    cpus = sorted(os.sched_getaffinity(0))[:6]
    os.sched_setaffinity(0, cpus)
    if os.getpriority(os.PRIO_PROCESS, 0) < 10:
        os.nice(10 - os.getpriority(os.PRIO_PROCESS, 0))
    env = dict(os.environ, CARGO_BUILD_JOBS='6', RAYON_NUM_THREADS='6')
    members, hashes = [], {}
    for mod, (type_name, setup, state) in MODS.items():
        before = work / 'sources' / mod
        before.mkdir(parents=True, exist_ok=True)
        baseline = args.baseline_dir.resolve() / mod
        shutil.copytree(baseline / 'src', before / 'src', dirs_exist_ok=True)
        manifest = (baseline / 'Cargo.toml').read_text()
        package = tomllib.loads(manifest)['package']['name']
        before_package = package + '-idle-before'
        manifest = manifest.replace('name = "' + package + '"', 'name = "' + before_package + '"', 1)
        manifest = manifest.replace('../../crates/horizon-mod-sdk', str(ROOT / 'crates/horizon-mod-sdk'))
        (before / 'Cargo.toml').write_text(manifest)
        hashes[mod] = {}
        for phase, source, pkg in [('before', before, before_package), ('after', ROOT / 'ports' / mod, package)]:
            name = f'bench-{mod.replace("_", "-")}-{phase}'
            folder = work / name
            (folder / 'src').mkdir(parents=True, exist_ok=True)
            members.append(name)
            (folder / 'Cargo.toml').write_text(f'''[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
[dependencies]
port_impl = {{ package = "{pkg}", path = "{source}" }}
horizon-mod-sdk = {{ path = "{ROOT / 'crates/horizon-mod-sdk'}" }}
serde_json = "1"
''')
            code = TEMPLATE.replace('@TYPE@', type_name).replace('@STATE@', state).replace('@SETUP@', setup).replace('@MOD@', mod)
            (folder / 'src/main.rs').write_text(code)
            hashes[mod][phase] = hashlib.sha256((source / 'src/lib.rs').read_bytes()).hexdigest()
    (work / 'Cargo.toml').write_text('[workspace]\nresolver = "2"\nmembers = ' + json.dumps(members) + '\n')
    # Preserve the current SDK's dependency versions in the diagnostic workspace.
    if not (work / 'Cargo.lock').exists():
        shutil.copyfile(ROOT / 'ports/Cargo.lock', work / 'Cargo.lock')
    subprocess.run(['cargo', 'build', '--release', '--manifest-path', str(work / 'Cargo.toml')], env=env, check=True)
    rows = []
    for mod in MODS:
        for loaded in (1, 100, 1000):
            for active in sorted({1, loaded}):
                for scenario in ('idle', 'active'):
                    pair = []
                    for phase in ('before', 'after'):
                        binary = work / 'target/release' / f'bench-{mod.replace("_", "-")}-{phase}'
                        path = out / f'{phase}-{mod}-{loaded}-{active}-{scenario}.json'
                        subprocess.run([str(binary), str(loaded), str(active), scenario, '200', str(path)], env=env, check=True)
                        pair.append(json.loads(path.read_text()))
                    assert pair[0]['active_states'] == pair[1]['active_states'], (mod, loaded, active, scenario, 'production changed')
                    assert pair[1]['inactive_changed'] == 0, (mod, 'inactive entity ticked')
                    rows.append({'mod': mod, 'loaded': loaded, 'received_handles': active, 'scenario': scenario,
                                 'p50_before_us': pair[0]['p50_us'], 'p50_after_us': pair[1]['p50_us'],
                                 'p95_before_us': pair[0]['p95_us'], 'p95_after_us': pair[1]['p95_us'],
                                 'inactive_changed_before': pair[0]['inactive_changed'], 'inactive_changed_after': pair[1]['inactive_changed']})
                    print(rows[-1], flush=True)
    (out / 'summary.json').write_text(json.dumps({'scope': 'native guest Rust loops only; no WASM or game inventory throughput',
        'cpu_affinity': cpus, 'nice': os.getpriority(os.PRIO_PROCESS, 0),
        'samples': 200, 'warmup_batches': 10, 'ticks_per_batch': 64,
        'source_sha256': hashes, 'results': rows}, indent=2) + '\n')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Profile actual powered crusher processing at 1, 100 and 1,000 machines."""
import argparse
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import re
import statistics
import time

from mod_port_game_fixture import Fixture
from profile_mod_tick_game import region_points, process_memory_mib

ROOT = Path(__file__).resolve().parents[1]
TICK = re.compile(r"mod_tick_profile dimension=(\S+) machines=(\d+) create_ns=(\d+) view_copy_ns=(\d+) capability_ns=(\d+) callback_ns=(\d+) serialize_ns=(\d+) total_ns=(\d+)")
CALLS = re.compile(r"mod_machine_profile mod=(\S+) machines=(\d+) get_position_calls=(\d+) get_position_ns=(\d+) get_slot_calls=(\d+) get_slot_ns=(\d+) get_energy_calls=(\d+) get_energy_ns=(\d+) set_energy_calls=(\d+) set_energy_ns=(\d+) process_items_calls=(\d+) process_items_ns=(\d+)")
DISPATCH = re.compile(r"mod_dispatch_profile machines=(\d+) seed_ns=(\d+) grouping_ns=(\d+) guest_ns=(\d+) return_views_ns=(\d+) sync_ns=(\d+) total_ns=(\d+)")


def stats(rows, fields):
    result = {}
    for field in fields:
        vals = [r[field] for r in rows]
        ordered = sorted(vals)
        result[field] = {
            "p50_ns": statistics.median(vals),
            "p95_ns": ordered[min(len(ordered) - 1, int(len(ordered) * .95))],
            "max_ns": max(vals),
        }
    return result


def parse_rows(log, pattern, expected, names, minimum=40, timeout=180):
    deadline = time.monotonic() + timeout
    rows = []
    while time.monotonic() < deadline:
        rows = []
        for match in pattern.finditer(log.read_text(errors="replace")):
            row = dict(zip(names, match.groups()))
            if int(row["machines"]) != expected:
                continue
            for key, value in row.items():
                if key not in ("dimension", "mod"):
                    row[key] = int(value)
            rows.append(row)
        if len(rows) >= minimum:
            return rows[-minimum:]
        time.sleep(0.5)
    raise AssertionError(f"need {minimum} samples for {expected} machines; got {len(rows)}")


def one_run(args, count):
    os.environ["HORIZON_MOD_TICK_PROFILE"] = "1"
    os.environ["HORIZON_MOD_MACHINE_PROFILE"] = "1"
    run_args = argparse.Namespace(**vars(args))
    run_args.out = args.out.with_name(f"{args.out.stem}-{count}.json")
    with Fixture(run_args, args.mode, [args.hmod]) as game:
        game.start()
        memory_stages = {"started": process_memory_mib(game.process.pid)}
        game.command("tp @s 1 102 4")
        game.call("fill", -4, 98, -4, 4, 98, 4, "minecraft:stone")
        points = region_points(count)
        if count == 1:
            game.call("setblock", *points[0], "actuallyadditions:crusher")
        elif count == 100:
            game.call("setblock", *points[0], "actuallyadditions:crusher")
            game.call("fill", 2, 100, 2, 4, 110, 4, "actuallyadditions:crusher")
        else:
            game.call("setblock", *points[0], "actuallyadditions:crusher")
            game.call("fill", 2, 100, 2, 4, 110, 4, "actuallyadditions:crusher")
            game.call("fill", -10, 100, -5, -1, 108, 4, "actuallyadditions:crusher")
        # First tick creates machine handles and their game-owned energy buffers.
        game.step(1)
        with redirect_stdout(io.StringIO()):
            for x, y, z in points:
                game.command(f"item replace block {x} {y} {z} container.0 with minecraft:iron_ore 1")
                game.command(f'data modify block {x} {y} {z} horizon_energy.stored set value "10000"')
        # Data mutation rebuilds a mod block entity with handle 0; this tick
        # rebinds its live handle and performs one active processing tick.
        game.step(1)
        game.step(40)
        log_text = game.logs[0]
        tick_names = ("dimension", "machines", "create_ns", "view_copy_ns", "capability_ns", "callback_ns", "serialize_ns", "total_ns")
        call_names = ("mod", "machines", "get_position_calls", "get_position_ns", "get_slot_calls", "get_slot_ns", "get_energy_calls", "get_energy_ns", "set_energy_calls", "set_energy_ns", "process_items_calls", "process_items_ns")
        dispatch_names = ("machines", "seed_ns", "grouping_ns", "guest_ns", "return_views_ns", "sync_ns", "total_ns")
        tick_rows = parse_rows(log_text, TICK, count, tick_names)
        call_rows = parse_rows(log_text, CALLS, count, call_names)
        dispatch_rows = parse_rows(log_text, DISPATCH, count, dispatch_names)
        fields_tick = tick_names[2:]
        fields_calls = tuple(x for x in call_names if x not in ("mod", "machines"))
        fields_dispatch = dispatch_names[1:]
        sample = game.command(f"data get block {points[0][0]} {points[0][1]} {points[0][2]}")
        slots = game.slots(points[0])
        memory = process_memory_mib(game.process.pid)
        memory_stages[str(count)] = memory
        game.stop()
        return {
            "machines": count,
            "ticks_profiled": len(tick_rows),
            "machine_import_profiled_ticks": len(call_rows),
            "dispatch_profiled_ticks": len(dispatch_rows),
            "tick_stats": stats(tick_rows, fields_tick),
            "host_import_stats": stats(call_rows, fields_calls),
            "dispatch_stats": stats(dispatch_rows, fields_dispatch),
            "sample_machine_after_42_active_ticks": {"data": sample, "slots": slots},
            "memory_mib_after_profile": memory,
            "host_import_profile_first_tick": call_rows[0],
            "mode": args.mode,
            "memory_by_count_mib": memory_stages,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hmod", type=Path, default=ROOT / ".local/atm10-resource-validation/actuallyadditions-with-carry.hmod")
    parser.add_argument("--bin-dir", type=Path, default=ROOT / "target/fast")
    parser.add_argument("--resources", type=Path, default=ROOT / ".local/horizon-client/26.3")
    parser.add_argument("--out", type=Path, default=ROOT / "docs/research/atm10-resource-validation/l7-active-profile-before.json")
    parser.add_argument("--mode", choices=("in-frame", "server-thread"), default="server-thread")
    args = parser.parse_args()
    for name in ("hmod", "bin_dir", "resources", "out"):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error("run with taskset -c 0-5 nice -n 10")
    results = [one_run(args, count) for count in (1, 100, 1000)]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        "scenario": "Actually Additions crusher with iron ore and 10,000 FE per machine; 41 active ticks total, 40 profiled at fixed count",
        "profile_environment": ["HORIZON_MOD_TICK_PROFILE=1", "HORIZON_MOD_MACHINE_PROFILE=1"],
        "runs": results,
    }, indent=2) + "\n")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()

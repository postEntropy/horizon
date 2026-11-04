#!/usr/bin/env python3
"""Measure the mod property cache as machine count grows and returns to zero."""
import argparse
import json
import os
from pathlib import Path
import re
import statistics
import time

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
POS = (0, 100, 0)
CACHE = re.compile(r"mod_property_cache machines=(\d+) entries=(\d+) map_capacity=(\d+) values=(\d+) estimated_capacity_bytes=(\d+)")


def memory_mib(pid):
    rows = {}
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith(("VmRSS:", "VmHWM:")):
            name, value, *_ = line.split()
            rows[name[:-1].lower()] = round(int(value) / 1024, 2)
    return rows


def snapshot(game, stage, machines):
    time.sleep(1)
    mem = [memory_mib(game.process.pid) for _ in range(5)]
    cache_rows = [tuple(map(int, m.groups())) for m in CACHE.finditer(game.logs[0].read_text(errors="replace"))]
    if not cache_rows:
        raise AssertionError("cache profiler emitted no sample")
    latest = cache_rows[-1]
    return {
        "stage": stage,
        "machines": machines,
        "rss_mib_samples": [m["vmrss"] for m in mem],
        "rss_mib_median": statistics.median(m["vmrss"] for m in mem),
        "vmhwm_mib": max(m["vmhwm"] for m in mem),
        "cache": {
            "active_machines_at_profile": latest[0],
            "entries": latest[1],
            "map_capacity": latest[2],
            "cached_values": latest[3],
            "estimated_capacity_bytes": latest[4],
            "estimate_note": "HashMap entry slots plus String/Vec capacities; excludes allocator metadata and hash control bytes",
        },
    }


def run(args, run_index):
    os.environ["HORIZON_MOD_TICK_PROFILE"] = "1"
    os.environ["HORIZON_MOD_PROPERTY_CACHE_PROFILE"] = "1"
    run_args = argparse.Namespace(**vars(args))
    run_args.out = args.out.with_name(f"{args.out.stem}-run{run_index}.json")
    with Fixture(run_args, "server-thread", [args.hmod]) as game:
        game.start()
        game.command("tp @s 1 102 4")
        game.call("fill", -4, 98, -4, 4, 98, 4, "minecraft:stone")
        result = [snapshot(game, "before-machines", 0)]
        game.call("setblock", *POS, "actuallyadditions:crusher")
        game.step(40)
        result.append(snapshot(game, "after-create", 1))
        game.call("fill", 2, 100, 2, 4, 110, 4, "actuallyadditions:crusher")
        game.step(40)
        result.append(snapshot(game, "after-create", 100))
        game.call("fill", -10, 100, -5, -1, 108, 4, "actuallyadditions:crusher")
        game.step(40)
        result.append(snapshot(game, "after-create", 1000))
        game.call("fill", -10, 100, -5, -1, 108, 4, "minecraft:air")
        game.call("fill", 2, 100, 2, 4, 110, 4, "minecraft:air")
        game.call("setblock", *POS, "minecraft:air")
        game.step(40)
        result.append(snapshot(game, "after-remove", 0))
        game.stop()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hmod", type=Path, default=ROOT / ".local/atm10-resource-validation/actuallyadditions-with-carry.hmod")
    parser.add_argument("--bin-dir", type=Path, default=ROOT / "target/fast")
    parser.add_argument("--resources", type=Path, default=ROOT / ".local/horizon-client/26.3")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=2)
    args = parser.parse_args()
    for name in ("hmod", "bin_dir", "resources", "out"):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error("run with taskset -c 0-5 nice -n 10")
    runs = [run(args, index + 1) for index in range(args.repeats)]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"runs": runs}, indent=2) + "\n")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()

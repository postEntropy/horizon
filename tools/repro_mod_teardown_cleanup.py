#!/usr/bin/env python3
"""A/B teardown reproduction; the only changed condition is input cleanup."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

from mod_port_game_fixture import Fixture
from profile_mod_tick_game import region_points

ROOT = Path(__file__).resolve().parents[1]


def run(args, cleanup, run_index):
    run_args = argparse.Namespace(**vars(args))
    run_args.out = args.out.with_name(f"{args.out.stem}-run{run_index}.json")
    os.environ["HORIZON_MOD_TICK_PROFILE"] = "1"
    result = {"run": run_index, "cleanup_before_exit": cleanup, "scenario": "1/100/1000 crushers, iron ore in all 1000 inputs, 40 ticks at final size"}
    with Fixture(run_args, "server-thread", [args.hmod]) as game:
        game.start()
        game.command("tp @s 1 102 4")
        game.call("fill", -4, 98, -4, 4, 98, 4, "minecraft:stone")
        game.call("setblock", 0, 100, 0, "actuallyadditions:crusher")
        game.step(40)
        game.call("fill", 2, 100, 2, 4, 110, 4, "actuallyadditions:crusher")
        game.step(40)
        game.call("fill", -10, 100, -5, -1, 108, 4, "actuallyadditions:crusher")
        points = region_points(1000)
        for x, y, z in points:
            game.command(f"item replace block {x} {y} {z} container.0 with minecraft:iron_ore 1")
        game.step(40)
        result["prepared_machine_count"] = 1000
        result["input_count"] = len(points)
        result["profile_lines_1000"] = sum("machines=1000 " in line for line in game.logs[0].read_text(errors="replace").splitlines())
        if cleanup:
            for x, y, z in points:
                game.command(f"item replace block {x} {y} {z} container.0 with air")
            game.step(1)
        result["quit_attempted"] = True
        try:
            game.call("quit")
        except (AssertionError, subprocess.TimeoutExpired) as exc:
            result["quit_rpc_error"] = str(exc)[:500]
        try:
            game.process.wait(timeout=45)
            result["returncode"] = game.process.returncode
        except subprocess.TimeoutExpired:
            game.process.kill()
            game.process.wait()
            result["returncode"] = game.process.returncode
            result["forced_kill"] = True
        result["clean_exit"] = result["returncode"] == 0
        game.process = None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hmod", type=Path, default=ROOT / ".local/atm10-resource-validation/actuallyadditions-with-carry.hmod")
    parser.add_argument("--bin-dir", type=Path, default=ROOT / "target/fast")
    parser.add_argument("--resources", type=Path, default=ROOT / ".local/horizon-client/26.3")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    for name in ("hmod", "bin_dir", "resources", "out"):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error("run with taskset -c 0-5 nice -n 10")
    matrix = [(False, 1), (True, 2), (True, 3), (False, 4)]
    results = [run(args, cleanup, index) for cleanup, index in matrix]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"one_variable": "clear 1000 container.0 inputs and advance one tick before shutdown", "results": results}, indent=2) + "\n")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Check active crusher conservation, blocked output, persistence and position ownership."""
import argparse
import json
import os
from pathlib import Path
import re

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
GOOD = (0, 100, 0)
BLOCKED = (1, 100, 0)
STORED = re.compile(r'stored: "(\d+)"')


def stored(game, pos):
    text = game.command(f"data get block {pos[0]} {pos[1]} {pos[2]} horizon_energy")
    match = STORED.search(text)
    if not match:
        raise AssertionError(f"could not read stored FE at {pos}: {text}")
    return int(match.group(1))


def slots(game, pos):
    return game.slots(pos)


def run(args, mode):
    checks = []
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command("tp @s 1 102 4")
        game.call("fill", -4, 98, -4, 4, 98, 4, "minecraft:stone")
        game.call("setblock", *GOOD, "actuallyadditions:crusher")
        game.call("setblock", *BLOCKED, "actuallyadditions:crusher")
        game.command("item replace block 0 100 0 container.0 with minecraft:iron_ore 2")
        game.command("item replace block 1 100 0 container.0 with minecraft:iron_ore 1")
        game.command("item replace block 1 100 0 container.1 with minecraft:dirt 1")
        game.step(1)  # Create handles and default capability records.
        for pos in (GOOD, BLOCKED):
            game.command(f'data modify block {pos[0]} {pos[1]} {pos[2]} horizon_energy.stored set value "10000"')
        game.step(1)  # Rebind the data-edited block entities and begin processing.
        game.step(99)

        good_slots = slots(game, GOOD)
        blocked_slots = slots(game, BLOCKED)
        good_energy = stored(game, GOOD)
        blocked_energy = stored(game, BLOCKED)
        expected_good = [
            {"count": 1, "item": "minecraft:iron_ore"},
            {"count": 2, "item": "minecraft:raw_iron"},
            None,
        ]
        expected_blocked = [
            {"count": 1, "item": "minecraft:iron_ore"},
            {"count": 1, "item": "minecraft:dirt"},
            None,
        ]
        if good_slots != expected_good or good_energy != 6_000:
            raise AssertionError(("successful recipe did not conserve expected stacks/FE", good_slots, good_energy))
        if blocked_slots != expected_blocked or blocked_energy != 10_000:
            raise AssertionError(("blocked output changed inventory or energy", blocked_slots, blocked_energy))
        checks.append({
            "stage": "after-100-active-ticks",
            "successful_recipe": {"slots": good_slots, "stored_fe": good_energy},
            "blocked_output": {"slots": blocked_slots, "stored_fe": blocked_energy},
        })

        game.call("save")
        game.stop()
        game.start()
        loaded_good_slots = slots(game, GOOD)
        loaded_blocked_slots = slots(game, BLOCKED)
        loaded_good_energy = stored(game, GOOD)
        loaded_blocked_energy = stored(game, BLOCKED)
        if loaded_good_slots != expected_good or loaded_blocked_slots != expected_blocked:
            raise AssertionError(("inventory did not survive save/load", loaded_good_slots, loaded_blocked_slots))
        # Startup ticks may run before tick freeze takes effect; each consumes 40 FE.
        startup_spend = good_energy - loaded_good_energy
        if not 0 <= startup_spend <= 20 * 40 or startup_spend % 40:
            raise AssertionError(("successful crusher energy did not persist", good_energy, loaded_good_energy))
        if loaded_blocked_energy != 10_000:
            raise AssertionError(("blocked crusher energy changed on load", loaded_blocked_energy))
        checks.append({
            "stage": "after-save-load",
            "successful_recipe": {"slots": loaded_good_slots, "stored_fe": loaded_good_energy,
                                  "startup_ticks_observed": startup_spend // 40},
            "blocked_output": {"slots": loaded_blocked_slots, "stored_fe": loaded_blocked_energy},
        })

        # Breaking and recreating a machine must release and then re-index its
        # coordinate so the new handle can write its own energy capability.
        game.call("setblock", *BLOCKED, "minecraft:air")
        game.call("setblock", *BLOCKED, "actuallyadditions:crusher")
        game.step(1)
        before = stored(game, BLOCKED)
        game.command('data modify block 1 100 0 horizon_energy.stored set value "500"')
        game.step(1)
        after = stored(game, BLOCKED)
        if before != 0 or after != 500:
            raise AssertionError(("new handle position index failed after removal", before, after))
        checks.append({"stage": "remove-recreate", "energy_before_write": before, "energy_after_write": after})
        game.stop()
    return {"mode": mode, "status": "passed", "checks": checks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hmod", type=Path, default=ROOT / ".local/atm10-resource-validation/actuallyadditions-active-profile.hmod")
    parser.add_argument("--bin-dir", type=Path, default=ROOT / "target/fast")
    parser.add_argument("--resources", type=Path, default=ROOT / ".local/horizon-client/26.3")
    parser.add_argument("--out", type=Path, default=ROOT / "docs/research/atm10-resource-validation/actuallyadditions-active-crusher-game.json")
    args = parser.parse_args()
    for name in ("hmod", "bin_dir", "resources", "out"):
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error("run with taskset -c 0-5 nice -n 10")
    rows = [run(args, mode) for mode in ("in-frame", "server-thread")]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"status": "passed", "scope": "active crusher conversion, blocked output, energy and save/load; ownership index after break/recreate", "cases": rows}, indent=2) + "\n")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()

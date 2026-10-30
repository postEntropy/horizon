#!/usr/bin/env python3
"""Exercise mod hooks and the API 0.4 machine/container path in isolated single-player worlds.

Builds two test WASM mods and Horizon, then checks per-block explosion filtering,
XP hooks, cross-mod FE transfer and vanilla hopper input/output. Runs both
in-frame and integrated-server single-player, and reopens each saved world to
check that machine capabilities and transferred items survive. Needs prepared
Minecraft resources in .local/horizon-client and a virtual screen on Linux.

    python3 tools/mod_api_e2e.py
    python3 tools/mod_api_e2e.py --mode integrated
    python3 tools/mod_api_e2e.py --bin-dir target/fast --mod target/mods-test/mods/horizon_mods_test_machine.hmod --peer-mod target/mods-test/mods/horizon_mods_test_machine_peer.hmod
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import agent_control_e2e

ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / '.local/horizon-client/26.3'
MOD = ROOT / 'target/mods-test/mods/horizon_mods_test_machine.hmod'
PEER_MOD = ROOT / 'target/mods-test/mods/horizon_mods_test_machine_peer.hmod'
MACHINE = (-31, 70, -9)
INPUT = (-31, 70, -10)
OUTPUT = (-31, 69, -9)


class CheckFailed(RuntimeError):
    pass


def run(command, *, cwd=ROOT, timeout=900):
    print('+', ' '.join(map(str, command)), flush=True)
    subprocess.run(command, cwd=cwd, check=True, timeout=timeout)


def ctl(ctl_bin, socket, cwd, env, *words):
    result = subprocess.run(
        [str(ctl_bin), '--socket', str(socket), *map(str, words)],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        timeout=180,
    )
    lines = result.stdout.strip().splitlines()
    reply = json.loads(lines[-1]) if lines else {'error': result.stderr.strip()}
    shown = reply
    if words and words[0] == 'wait-ready':
        shown = {key: reply.get(key) for key in ('screen', 'ticks', 'dimension', 'world')}
    elif words and words[0] == 'command' and reply.get('chat'):
        shown = {'ok': reply.get('ok'), 'chat': reply['chat'][:1]}
    print(f'[ctl] {words} -> {json.dumps(shown)}', flush=True)
    if result.returncode or 'error' in reply:
        raise CheckFailed(f'{words}: exit {result.returncode}: {reply}')
    return reply


def command(ctl_bin, socket, cwd, env, text):
    reply = ctl(ctl_bin, socket, cwd, env, 'command', text)
    # In-frame mode can retain earlier messages in the response; the current
    # command's feedback is first in its chat array.
    return next(iter(reply.get('chat', [])), '')


def stack_count(items, item_id):
    pattern = rf'\{{count: (\d+), Slot: \d+b, id: "{re.escape(item_id)}"\}}'
    return sum(map(int, re.findall(pattern, items)))


def machine_capability_state(nbt):
    energy = re.search(r'horizon_energy: \{[^}]*stored: "(\d+)"', nbt)
    water = re.search(r'horizon_fluids: \[\{[^}]*amount: "(\d+)"[^}]*fluid: "minecraft:water"', nbt)
    if not energy or not water:
        raise CheckFailed(f'machine capability state missing from block data: {nbt}')
    return int(energy.group(1)), int(water.group(1))


def assert_machine_owned_capacities(nbt):
    energy_capacity = re.search(r'horizon_energy: \{[^}]*capacity: "(\d+)"', nbt)
    water_tank = re.search(
        r'horizon_fluids: \[\{[^}]*fluid: "([^"]+)"[^}]*capacity: "(\d+)"', nbt)
    if not energy_capacity or not water_tank:
        raise CheckFailed(f'machine capability definitions missing from block data: {nbt}')
    assert int(energy_capacity.group(1)) == 10_000, nbt
    assert water_tank.group(1) == 'minecraft:water' and int(water_tank.group(2)) == 1_000, nbt


def energy_state(nbt):
    energy = re.search(r'horizon_energy: \{[^}]*stored: "(\d+)"', nbt)
    if not energy:
        raise CheckFailed(f'machine energy state missing from block data: {nbt}')
    return int(energy.group(1))


def fluid_state(nbt, fluid_id):
    pattern = rf'horizon_fluids: \[\{{[^}}]*amount: "(\d+)"[^}}]*fluid: "{re.escape(fluid_id)}"[^}}]*capacity: "(\d+)"'
    fluid = re.search(pattern, nbt)
    if not fluid:
        raise CheckFailed(f'{fluid_id} tank missing from block data: {nbt}')
    return int(fluid.group(1)), int(fluid.group(2))


def start_game(client, ctl_bin, mods, work, env, *, in_frame):
    socket = work / 'runtime/control.sock'
    args = [
        str(client), '--resources', str(RESOURCES), '--world', 'ModApiE2E',
        '--creative', '--control', '--control-socket', str(socket),
    ]
    for mod in mods:
        args.extend(['--dev-mod', str(mod)])
    if in_frame:
        args.append('--in-frame')
    if sys.platform == 'darwin':
        args.append('--offscreen')
    log = (work / 'game.log').open('w')
    process = subprocess.Popen(args, cwd=work / 'game', env=env, stdout=log, stderr=subprocess.STDOUT)
    deadline = time.monotonic() + 300
    while not socket.exists():
        if process.poll() is not None:
            log.close()
            raise CheckFailed(f'game exited {process.returncode}: {tail(work / "game.log")}')
        if time.monotonic() >= deadline:
            process.terminate()
            log.close()
            raise CheckFailed(f'game did not open control socket: {tail(work / "game.log")}')
        time.sleep(0.25)
    try:
        ctl(ctl_bin, socket, work / 'game', env, 'wait-ready', 300)
    except Exception:
        process.terminate()
        try:
            process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        log.close()
        raise
    return process, log, socket


def tail(path):
    try:
        return '\n'.join(path.read_text(errors='replace').splitlines()[-30:])
    except OSError:
        return '(no game log)'


def game_environment(mode):
    if sys.platform == 'darwin' or shutil.which('vscreen'):
        return dict(agent_control_e2e.game_env(f'mod-api-{mode}'))
    if os.environ.get('DISPLAY'):
        env = dict(os.environ)
        env.pop('WAYLAND_DISPLAY', None)
        return env
    raise CheckFailed('this run needs vscreen or an available X11 DISPLAY')


def stop_game(process, log, ctl_bin, socket, cwd, env):
    if process.poll() is None:
        try:
            ctl(ctl_bin, socket, cwd, env, 'save')
            ctl(ctl_bin, socket, cwd, env, 'quit')
            process.wait(timeout=60)
        except (CheckFailed, subprocess.TimeoutExpired):
            process.terminate()
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    log.close()


def wait_for_block(ctl_bin, socket, cwd, env, pos, expected, *, attempts=10):
    last = None
    for _ in range(attempts):
        last = ctl(ctl_bin, socket, cwd, env, 'block', *pos)
        if last.get('block') == expected:
            return
        ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    raise AssertionError(f'expected block {expected} at {pos}, got {last}')


def verify_mod_screen(ctl_bin, socket, cwd, env, *, verify_restored_attachment=False):
    command(ctl_bin, socket, cwd, env, 'setblock -31 69 -11 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..8]')
    command(ctl_bin, socket, cwd, env, 'tp @s -31 70 -11')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..8]')
    command(ctl_bin, socket, cwd, env, 'clear @s')
    ctl(ctl_bin, socket, cwd, env, 'look-at-block', -31, 70, -9, 'north')
    target = ctl(ctl_bin, socket, cwd, env, 'looking-at')
    hit = target.get('block') or {}
    assert target.get('kind') == 'block' and hit.get('pos') == [-31, 70, -9] and hit.get('face') == 'north', target
    command(ctl_bin, socket, cwd, env,
            'item replace block -31 70 -9 container.0 with minecraft:diamond 4')
    ctl(ctl_bin, socket, cwd, env, 'mouse', 'right', 'click')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    screen = ctl(ctl_bin, socket, cwd, env, 'screen')
    assert screen.get('screen') == 'container', f'mod machine screen did not open: {screen}'
    mod_screen = screen.get('mod_screen')
    assert mod_screen, f'mod machine screen contents are missing: {screen}'
    assert mod_screen.get('title') == 'Smelter', mod_screen
    assert [widget.get('id') for widget in mod_screen.get('widgets', [])] == [1, 2, 3, 4], mod_screen
    assert mod_screen.get('backgrounds'), mod_screen
    assert {'slot': 2, 'item': 'minecraft:iron_ingot', 'count': 3} in mod_screen.get('slots', []), mod_screen

    origin = screen.get('mod_screen_origin')
    assert origin and len(origin) == 2, screen
    if verify_restored_attachment:
        restore_button = next(widget for widget in mod_screen['widgets']
                              if widget.get('id') == 4)
        restore_x = origin[0] + restore_button['x'] + restore_button['width'] / 2
        restore_y = origin[1] + restore_button['y'] + restore_button['height'] / 2
        command(ctl_bin, socket, cwd, env,
                'setblock -26 70 -9 minecraft:stone')
        ctl(ctl_bin, socket, cwd, env, 'click-at', restore_x, restore_y)
        ctl(ctl_bin, socket, cwd, env, 'wait', 3)
        wait_for_block(ctl_bin, socket, cwd, env, (-26, 70, -9), 'minecraft:diamond_block')
    slot_x, slot_y = origin[0] + 16, origin[1] + 26
    ctl(ctl_bin, socket, cwd, env, 'click-at', slot_x, slot_y)
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    picked = ctl(ctl_bin, socket, cwd, env, 'screen')
    assert picked.get('carried') == {'item': 'minecraft:diamond', 'count': 4}, picked
    assert next(slot for slot in picked.get('slots', []) if slot.get('slot') == 300).get('item') is None, picked
    wait_for_block(ctl_bin, socket, cwd, env, (-28, 70, -9), 'minecraft:emerald_block')

    ctl(ctl_bin, socket, cwd, env, 'click-at', slot_x, slot_y, 'right')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    placed = ctl(ctl_bin, socket, cwd, env, 'screen')
    assert placed.get('carried') == {'item': 'minecraft:diamond', 'count': 3}, placed
    machine_slot = next(slot for slot in placed.get('slots', []) if slot.get('slot') == 300)
    assert machine_slot.get('item') == 'minecraft:diamond' and machine_slot.get('count') == 1, placed
    assert 'count: 1, Slot: 0b, id: "minecraft:diamond"' in command(
        ctl_bin, socket, cwd, env, 'data get block -31 70 -9 Items')

    # Shift-click the machine stack into the player inventory. The callback
    # marker is reset first so this also proves QuickMove dispatches the hook.
    ctl(ctl_bin, socket, cwd, env, 'command',
        'setblock -28 70 -9 minecraft:stone')
    ctl(ctl_bin, socket, cwd, env, 'key', 'shift', 'down')
    try:
        ctl(ctl_bin, socket, cwd, env, 'click-at', slot_x, slot_y)
    finally:
        ctl(ctl_bin, socket, cwd, env, 'key', 'shift', 'up')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    quick_moved = ctl(ctl_bin, socket, cwd, env, 'screen')
    assert quick_moved.get('carried') == {'item': 'minecraft:diamond', 'count': 3}, quick_moved
    assert next(slot for slot in quick_moved.get('slots', [])
                if slot.get('slot') == 300).get('item') is None, quick_moved
    inventory_diamonds = sum(
        slot.get('count', 0) for slot in quick_moved.get('slots', [])
        if 0 <= slot.get('slot', -1) < 36 and slot.get('item') == 'minecraft:diamond')
    assert inventory_diamonds == 1, quick_moved
    assert 'diamond' not in command(
        ctl_bin, socket, cwd, env, 'data get block -31 70 -9 Items')
    wait_for_block(ctl_bin, socket, cwd, env, (-28, 70, -9), 'minecraft:emerald_block')

    button = next(widget for widget in mod_screen['widgets'] if widget.get('id') == 3)
    button_x = origin[0] + button['x'] + button['width'] / 2
    button_y = origin[1] + button['y'] + button['height'] / 2
    command(ctl_bin, socket, cwd, env,
            'setblock -27 70 -9 minecraft:stone')
    ctl(ctl_bin, socket, cwd, env, 'click-at', button_x, button_y)
    closed = None
    for _ in range(10):
        ctl(ctl_bin, socket, cwd, env, 'wait', 2)
        closed = ctl(ctl_bin, socket, cwd, env, 'screen')
        if closed.get('screen') == 'playing' and 'mod_screen' not in closed:
            break
    assert closed and closed.get('screen') == 'playing' and 'mod_screen' not in closed, closed
    wait_for_block(ctl_bin, socket, cwd, env, (-27, 70, -9), 'minecraft:gold_block')


def verify_explosion_hook(ctl_bin, socket, cwd, env):
    center = (-31, 70, -12)
    stone = (-30, 70, -12)
    glass = (-32, 70, -12)
    command(ctl_bin, socket, cwd, env, f'setblock {center[0]} 69 {center[2]} minecraft:stone')
    command(ctl_bin, socket, cwd, env, f'setblock {stone[0]} {stone[1]} {stone[2]} minecraft:stone')
    command(ctl_bin, socket, cwd, env, f'setblock {glass[0]} {glass[1]} {glass[2]} minecraft:glass')
    command(ctl_bin, socket, cwd, env,
            f'summon minecraft:tnt {center[0]} {center[1]} {center[2]} {{fuse:1s}}')
    ctl(ctl_bin, socket, cwd, env, 'wait', 10)
    protected = ctl(ctl_bin, socket, cwd, env, 'block', *stone)
    destroyed = ctl(ctl_bin, socket, cwd, env, 'block', *glass)
    assert protected.get('block') == 'minecraft:stone', protected
    assert destroyed.get('block') == 'minecraft:air', destroyed


def verify_vanilla_hooks(ctl_bin, socket, cwd, env):
    command(ctl_bin, socket, cwd, env, 'setblock -33 69 -12 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'tp @s -33 70 -12')
    command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..6]')

    # 1. Container break loot-drop: Cancel (-33), Consume (-32), Pass (-34)
    command(ctl_bin, socket, cwd, env, 'setblock -33 70 -14 minecraft:chest')
    command(ctl_bin, socket, cwd, env,
            'item replace block -33 70 -14 container.0 with minecraft:gold_ingot 3')
    command(ctl_bin, socket, cwd, env, 'setblock -33 70 -14 minecraft:air destroy')
    cancelled_items = command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..6]')
    assert 'No entity was found' in cancelled_items, cancelled_items

    command(ctl_bin, socket, cwd, env, 'setblock -32 70 -14 minecraft:chest')
    command(ctl_bin, socket, cwd, env,
            'item replace block -32 70 -14 container.0 with minecraft:gold_ingot 3')
    command(ctl_bin, socket, cwd, env, 'setblock -32 70 -14 minecraft:air destroy')
    consumed_nbt = command(ctl_bin, socket, cwd, env, 'data get entity @e[type=item,distance=..6,limit=1] Item')
    assert 'id: "minecraft:diamond"' in consumed_nbt and 'count: 1' in consumed_nbt, consumed_nbt
    command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..6]')

    command(ctl_bin, socket, cwd, env, 'setblock -34 70 -14 minecraft:chest')
    command(ctl_bin, socket, cwd, env,
            'item replace block -34 70 -14 container.0 with minecraft:gold_ingot 3')
    command(ctl_bin, socket, cwd, env, 'setblock -34 70 -14 minecraft:air destroy')
    passed_kill = command(ctl_bin, socket, cwd, env, 'kill @e[type=item,distance=..6]')
    assert 'Killed 2 entities' in passed_kill, passed_kill

    # 2. /loot table drop intercepted by loot-drop hook (Consume -> 1x diamond)
    command(ctl_bin, socket, cwd, env, 'clear @s')
    command(ctl_bin, socket, cwd, env, 'loot give @s loot minecraft:blocks/gravel')
    inv_nbt = command(ctl_bin, socket, cwd, env, 'data get entity @s Inventory')
    assert 'id: "minecraft:diamond"' in inv_nbt, inv_nbt
    command(ctl_bin, socket, cwd, env, 'clear @s')

    # 3. Incoming player entity-damage: Cancel on cactus, 0.5x on generic
    command(ctl_bin, socket, cwd, env, 'setblock -31 69 -11 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'tp @s -31 70 -11')
    command(ctl_bin, socket, cwd, env, 'gamemode survival')
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    cactus_dmg = command(ctl_bin, socket, cwd, env, 'damage @s 8 minecraft:cactus')
    assert 'invulnerable' in cactus_dmg, cactus_dmg
    hp_after_cactus = command(ctl_bin, socket, cwd, env, 'data get entity @s Health')
    assert '20.0f' in hp_after_cactus, hp_after_cactus
    generic_dmg = command(ctl_bin, socket, cwd, env, 'damage @s 8 minecraft:generic')
    assert 'Applied 8.0 damage' in generic_dmg, generic_dmg
    hp_after_generic = command(ctl_bin, socket, cwd, env, 'data get entity @s Health')
    assert '16.0f' in hp_after_generic, hp_after_generic
    command(ctl_bin, socket, cwd, env, 'gamemode creative')
    command(ctl_bin, socket, cwd, env, 'effect give @s minecraft:instant_health 1 10')
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)

    # 4. Bone meal crop-grow multiplier (3.0x advances wheat[age=0] to age >= 6 in one use)
    command(ctl_bin, socket, cwd, env, 'setblock -31 69 -12 minecraft:farmland')
    command(ctl_bin, socket, cwd, env, 'setblock -31 70 -12 minecraft:wheat[age=0]')
    command(ctl_bin, socket, cwd, env,
            'item replace entity @s weapon.mainhand with minecraft:bone_meal 1')
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    ctl(ctl_bin, socket, cwd, env, 'look-at', -30.5, 70.06, -11.5)
    ctl(ctl_bin, socket, cwd, env, 'mouse', 'right', 'click')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    wheat = ctl(ctl_bin, socket, cwd, env, 'getblock', -31, 70, -12).get('block', '')
    assert wheat in ('minecraft:wheat[age=6]', 'minecraft:wheat[age=7]'), wheat
    command(ctl_bin, socket, cwd, env, 'clear @s')
    command(ctl_bin, socket, cwd, env, 'setblock -31 70 -12 minecraft:air')
    command(ctl_bin, socket, cwd, env, 'setblock -31 69 -12 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'kill @e[type=item]')


def verify_recipe_api(ctl_bin, socket, cwd, env):
    feedback = command(ctl_bin, socket, cwd, env, 'machine_api caps_and_recipes')
    recipes = re.search(r'\brecipes=([^ ]+)', feedback)
    assert recipes, f'recipe API callback did not report its query results: {feedback}'
    custom = recipes.group(1).split('/')
    assert len(custom) == 7, feedback
    assert custom[0] == 'true', f'registered recipe was not queryable by id: {feedback}'
    assert int(custom[1]) > 0, f'custom recipe type query returned no recipes: {feedback}'
    assert int(custom[2]) > 0, f'input item query returned no recipes: {feedback}'
    assert int(custom[3]) > 0, f'output item query returned no recipes: {feedback}'
    assert custom[4] == 'true', f'registered recipe replacement was not visible: {feedback}'
    assert int(custom[6]) > 0, f'replaced recipe output query returned no recipes: {feedback}'


def parse_entity_status(feedback):
    m = re.search(
        r'count=(\d+) handle=(\d+) age=(\d+) hp=([0-9.]+) pos=([-0-9.]+),([-0-9.]+),([-0-9.]+)',
        feedback,
    )
    if not m:
        raise CheckFailed(f'unexpected entity_status output: {feedback}')
    return {
        'count': int(m.group(1)),
        'handle': int(m.group(2)),
        'age': int(m.group(3)),
        'hp': float(m.group(4)),
        'pos': (float(m.group(5)), float(m.group(6)), float(m.group(7))),
    }


def verify_custom_entity(ctl_bin, socket, cwd, env):
    command(ctl_bin, socket, cwd, env, 'fill -38 69 -12 -35 69 -8 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'fill -38 70 -12 -35 72 -8 minecraft:air')
    command(ctl_bin, socket, cwd, env, 'fill -38 70 -12 -38 71 -8 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'fill -35 70 -12 -35 71 -8 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'fill -38 70 -12 -35 71 -12 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'fill -38 70 -8 -35 71 -8 minecraft:stone')

    spawn_out = command(ctl_bin, socket, cwd, env, 'machine_api spawn_automaton')
    assert 'near=1' in spawn_out and 'despawned=true' in spawn_out, spawn_out

    ctl(ctl_bin, socket, cwd, env, 'wait', 8)
    status = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert status['count'] == 1, status
    assert status['age'] >= 2, status
    assert status['hp'] == 40.0, status

    ctrl_out = command(ctl_bin, socket, cwd, env, 'machine_api control_automaton')
    assert 'controlled=' in ctrl_out, ctrl_out
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    controlled = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert abs(controlled['pos'][0] - (-36.5)) < 0.25 and abs(controlled['pos'][2] - (-10.5)) < 0.25, controlled

    # Custom entity on_entity_hurt cancels cactus damage, while generic damage
    # passes on_entity_hurt and is halved by the EntityDamage vanilla hook.
    command(ctl_bin, socket, cwd, env,
            'damage @e[type=test_machine:automaton,limit=1] 10 minecraft:cactus')
    after_cactus = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert after_cactus['hp'] == 40.0, after_cactus

    command(ctl_bin, socket, cwd, env,
            'damage @e[type=test_machine:automaton,limit=1] 10 minecraft:generic')
    after_generic = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert after_generic['hp'] == 35.0, after_generic

    # Player right-click interaction with iron_ingot dispatches on_entity_interact
    # and sets the custom entity's health to 20.0.
    command(ctl_bin, socket, cwd, env, 'clear @s')
    command(ctl_bin, socket, cwd, env, 'item replace entity @s weapon.mainhand with minecraft:iron_ingot')
    command(ctl_bin, socket, cwd, env, 'fill -37 69 -11 -36 69 -9 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'tp @s -36.5 70 -9.2')
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    ex, ey, ez = after_generic['pos']
    ctl(ctl_bin, socket, cwd, env, 'look-at', ex, ey + 0.9, ez)
    ctl(ctl_bin, socket, cwd, env, 'mouse', 'right', 'click')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    after_interact = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert after_interact['hp'] == 20.0, after_interact
    command(ctl_bin, socket, cwd, env, 'clear @s')

    nbt = command(ctl_bin, socket, cwd, env, 'data get entity @e[type=test_machine:automaton,limit=1]')
    assert 'entity.test_machine.automaton' in nbt and 'horizon_mod_data:' in nbt, nbt
    return after_interact['age']


def verify_custom_entity_after_reopen(ctl_bin, socket, cwd, env, age_before_save):
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    restored = parse_entity_status(command(ctl_bin, socket, cwd, env, 'machine_api entity_status'))
    assert restored['count'] == 1, restored
    assert restored['age'] >= age_before_save, (restored, age_before_save)
    assert restored['hp'] == 20.0, restored
    nbt = command(ctl_bin, socket, cwd, env, 'data get entity @e[type=test_machine:automaton,limit=1]')
    assert 'entity.test_machine.automaton' in nbt and 'horizon_mod_data:' in nbt, nbt


def wait_for_dimension(ctl_bin, socket, cwd, env, expected, *, attempts=40):
    last = None
    for _ in range(attempts):
        last = ctl(ctl_bin, socket, cwd, env, 'state')
        if last.get('dimension') == expected and last.get('player') is not None:
            return last
        ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    raise AssertionError(f'expected dimension {expected} with spawned player, got {last}')


def verify_custom_dimension(ctl_bin, socket, cwd, env):
    dims_out = command(ctl_bin, socket, cwd, env, 'machine_api dims')
    assert 'test_machine:twilight' in dims_out, dims_out

    # Custom portal activation: right-clicking the registered frame_block
    # (`minecraft:mossy_cobblestone`) with `minecraft:diamond` teleports the
    # player into `test_machine:twilight` with coordinate_scale=2.0 and generates
    # the custom flat surface (`minecraft:moss_block` at y=63).
    command(ctl_bin, socket, cwd, env, 'fill -32 69 -12 -30 69 -10 minecraft:stone')
    command(ctl_bin, socket, cwd, env, 'setblock -31 70 -12 minecraft:mossy_cobblestone')
    command(ctl_bin, socket, cwd, env, 'tp @s -30.5 70.0 -10.5')
    command(ctl_bin, socket, cwd, env, 'clear @s')
    command(ctl_bin, socket, cwd, env, 'item replace entity @s weapon.mainhand with minecraft:diamond')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)
    ctl(ctl_bin, socket, cwd, env, 'look-at-block', -31, 70, -12, 'south')
    ctl(ctl_bin, socket, cwd, env, 'mouse', 'right', 'click')

    twilight_state = wait_for_dimension(ctl_bin, socket, cwd, env, 'test_machine:twilight')
    player_pos = (twilight_state.get('player') or {}).get('position') or [0.0, 0.0, 0.0]
    assert abs(player_pos[0] - (-15.25)) < 1.5 and abs(player_pos[2] - (-5.25)) < 1.5, twilight_state
    wait_for_block(ctl_bin, socket, cwd, env, (-16, 63, -6), 'minecraft:moss_block', attempts=30)
    wait_for_block(ctl_bin, socket, cwd, env, (-16, -64, -6), 'minecraft:bedrock', attempts=10)

    # Return to Overworld by activating the portal inside the custom dimension.
    command(ctl_bin, socket, cwd, env, 'setblock -16 64 -7 minecraft:mossy_cobblestone')
    command(ctl_bin, socket, cwd, env, 'tp @s -15.5 64.0 -5.5')
    ctl(ctl_bin, socket, cwd, env, 'wait', 2)
    ctl(ctl_bin, socket, cwd, env, 'look-at-block', -16, 64, -7, 'south')
    ctl(ctl_bin, socket, cwd, env, 'mouse', 'right', 'click')
    wait_for_dimension(ctl_bin, socket, cwd, env, 'minecraft:overworld')

    # Direct mod teleport (`dimensions::teleport`) to and from the custom dimension.
    tp_out = command(ctl_bin, socket, cwd, env, 'machine_api tp_twilight')
    assert 'tp_twilight=ok' in tp_out, tp_out
    wait_for_dimension(ctl_bin, socket, cwd, env, 'test_machine:twilight')
    wait_for_block(ctl_bin, socket, cwd, env, (8, 63, 8), 'minecraft:moss_block', attempts=30)

    back_out = command(ctl_bin, socket, cwd, env, 'machine_api tp_overworld')
    assert 'tp_overworld=ok' in back_out, back_out
    wait_for_dimension(ctl_bin, socket, cwd, env, 'minecraft:overworld')
    command(ctl_bin, socket, cwd, env, 'setblock -31 70 -12 minecraft:air')
    command(ctl_bin, socket, cwd, env, 'clear @s')
    ctl(ctl_bin, socket, cwd, env, 'wait', 3)


def check_mode(client, ctl_bin, mods, *, in_frame):
    mode = 'in-frame' if in_frame else 'integrated'
    work = Path(tempfile.mkdtemp(prefix=f'horizon-mod-api-{mode}-'))
    game_dir = work / 'game'
    runtime = work / 'runtime'
    mods_dir = work / 'mods'
    for path in (game_dir, runtime, mods_dir, work / 'home', work / 'launcher',
                 work / 'xdg-data', work / 'xdg-config'):
        path.mkdir(parents=True, exist_ok=True)
    local_mods = []
    for mod in mods:
        dest = mods_dir / mod.name
        shutil.copy2(mod, dest)
        local_mods.append(dest)
    mods = local_mods
    runtime.chmod(0o700)
    env = game_environment(mode)
    env.update({
        'HORIZON_HOME': str(work / 'home'),
        'HORIZON_LAUNCHER_HOME': str(work / 'launcher'),
        'HORIZON_LAUNCHER_NO_SELF_UPDATE': '1',
        'HORIZON_NO_AUDIO': '1',
        'HORIZON_TELEMETRY_URL': 'off',
        'XDG_DATA_HOME': str(work / 'xdg-data'),
        'XDG_CONFIG_HOME': str(work / 'xdg-config'),
    })
    process = log = socket = None
    try:
        process, log, socket = start_game(client, ctl_bin, mods, work, env, in_frame=in_frame)
        verify_explosion_hook(ctl_bin, socket, game_dir, env)
        command(ctl_bin, socket, game_dir, env, 'experience add @s 50 points')
        xp_levels = command(ctl_bin, socket, game_dir, env, 'experience query @s levels')
        assert re.search(r'\b7\b', xp_levels), f'xp-gain hook did not double command XP: {xp_levels}'
        cfg_before = command(ctl_bin, socket, game_dir, env, 'machine_api read_config')
        assert 'speed=2 bool=true float=1 mode=safe' in cfg_before, cfg_before
        cfg_set = command(ctl_bin, socket, game_dir, env, 'machine_api config')
        assert 'speed=2->5/reject=true/kept=5' in cfg_set and 'mode=safe->fast/reject=true' in cfg_set, cfg_set
        verify_recipe_api(ctl_bin, socket, game_dir, env)
        command(ctl_bin, socket, game_dir, env, 'machine_api set_machine_block')
        command(ctl_bin, socket, game_dir, env, 'peer_api set_peer_block')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 3)
        verify_mod_screen(ctl_bin, socket, game_dir, env)
        for pos, state in ((OUTPUT, 'minecraft:hopper[facing=down]'),
                           (INPUT, 'minecraft:hopper[facing=south]')):
            x, y, z = pos
            assert 'Changed the block' in command(
                ctl_bin, socket, game_dir, env, f'setblock {x} {y} {z} {state}')

        mx, my, mz = MACHINE
        ox, oy, oz = OUTPUT
        nbt = command(ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz}')
        assert 'id: "horizon:mod_block_entity"' in nbt and 'mod_slots: 2' in nbt, nbt
        energy_initial, water_initial = machine_capability_state(nbt)
        assert energy_initial > 0 and water_initial > 0, nbt
        assert_machine_owned_capacities(nbt)
        px, py, pz = (MACHINE[0] + 1, MACHINE[1], MACHINE[2])
        peer = command(ctl_bin, socket, game_dir, env, f'data get block {px} {py} {pz}')
        assert 'mod_block: "test_peer:cell"' in peer, peer
        peer_energy_initial = energy_state(peer)
        assert peer_energy_initial > 0, f'peer received no energy from test_machine: {peer}'
        peer_water_initial, peer_water_capacity = fluid_state(peer, 'minecraft:water')
        assert 0 < peer_water_initial <= peer_water_capacity, peer
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.0 with minecraft:raw_iron')
        seeded = command(ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' in seeded, seeded
        ctl(ctl_bin, socket, game_dir, env, 'wait', 10)
        bottom = command(ctl_bin, socket, game_dir, env, f'data get block {ox} {oy} {oz} Items')
        still_in_machine = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' not in bottom, bottom
        assert 'minecraft:raw_iron' in still_in_machine, still_in_machine
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.0 with minecraft:air')
        cleared = command(ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' not in cleared, cleared
        command(ctl_bin, socket, game_dir, env,
                f'setblock {ox} {oy} {oz} minecraft:air')

        above = (mx, my + 1, mz)
        ax, ay, az = above
        assert 'Changed the block' in command(
            ctl_bin, socket, game_dir, env,
            f'setblock {ax} {ay} {az} minecraft:hopper[facing=down]')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {ax} {ay} {az} container.0 with minecraft:raw_iron')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 10)
        top = command(ctl_bin, socket, game_dir, env, f'data get block {ax} {ay} {az} Items')
        machine_input = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' in top, top
        assert 'minecraft:raw_iron' not in machine_input, machine_input
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {ax} {ay} {az} container.0 with minecraft:air')
        command(ctl_bin, socket, game_dir, env, f'setblock {ax} {ay} {az} minecraft:air')
        assert 'Changed the block' in command(
            ctl_bin, socket, game_dir, env,
            f'setblock {ox} {oy} {oz} minecraft:hopper[facing=down]')

        # A full output stack must backpressure machine processing. Releasing
        # one item of capacity lets exactly one recipe complete.
        command(ctl_bin, socket, game_dir, env,
                f'setblock {ox} {oy} {oz} minecraft:air')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:glass_bottle 64')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.0 with minecraft:honey_bottle 2')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 2)
        blocked_remainders = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:honey_bottle' in blocked_remainders and 'count: 2,' in blocked_remainders, blocked_remainders
        assert 'minecraft:glass_bottle' in blocked_remainders and 'count: 64,' in blocked_remainders, blocked_remainders
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:glass_bottle 63')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 2)
        one_remainder = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:honey_bottle' in one_remainder and 'count: 1,' in one_remainder, one_remainder
        assert 'minecraft:glass_bottle' in one_remainder and 'count: 64,' in one_remainder, one_remainder
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:glass_bottle 63')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 2)
        remainder_output = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:honey_bottle' not in remainder_output, remainder_output
        assert 'minecraft:glass_bottle' in remainder_output and 'count: 64,' in remainder_output, remainder_output
        assert 'Changed the block' in command(
            ctl_bin, socket, game_dir, env,
            f'setblock {ox} {oy} {oz} minecraft:hopper[facing=down]')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 8)
        remainder_hopper = command(
            ctl_bin, socket, game_dir, env, f'data get block {ox} {oy} {oz} Items')
        remainder_left = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:glass_bottle' in remainder_hopper, remainder_hopper
        moved = stack_count(remainder_hopper, 'minecraft:glass_bottle')
        remaining = stack_count(remainder_left, 'minecraft:glass_bottle')
        assert moved > 0 and remaining > 0, (remainder_hopper, remainder_left)
        command(ctl_bin, socket, game_dir, env,
                f'setblock {ox} {oy} {oz} minecraft:air')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:air')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.0 with minecraft:raw_iron')
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:iron_ingot 64')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 220)
        blocked_input = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' in blocked_input, blocked_input
        assert 'minecraft:iron_ingot' in blocked_input and 'count: 64,' in blocked_input, blocked_input
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:iron_ingot 63')
        ctl(ctl_bin, socket, game_dir, env, 'wait', 220)
        completed_input = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'minecraft:raw_iron' not in completed_input, completed_input
        assert 'minecraft:iron_ingot' in completed_input and 'count: 64,' in completed_input, completed_input
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.1 with minecraft:air')
        assert 'Changed the block' in command(
            ctl_bin, socket, game_dir, env,
            f'setblock {ox} {oy} {oz} minecraft:hopper[facing=down]')

        ix, iy, iz = INPUT
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {ix} {iy} {iz} container.0 with minecraft:raw_iron')

        ctl(ctl_bin, socket, game_dir, env, 'wait', 240)
        output = command(ctl_bin, socket, game_dir, env, f'data get block {ox} {oy} {oz} Items')
        source = command(ctl_bin, socket, game_dir, env, f'data get block {ix} {iy} {iz} Items')
        assert 'minecraft:iron_ingot' in output, output
        assert 'minecraft:raw_iron' not in source, source
        machine_before_save = command(
            ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz}')
        energy_saved, water_saved = machine_capability_state(machine_before_save)
        assert_machine_owned_capacities(machine_before_save)
        assert energy_saved > energy_initial and water_saved == 1000, machine_before_save
        peer_before_save = command(
            ctl_bin, socket, game_dir, env, f'data get block {px} {py} {pz}')
        peer_energy_saved = energy_state(peer_before_save)
        assert peer_energy_saved >= peer_energy_initial, peer_before_save
        assert peer_energy_saved < 1_000, f'per-tick receive limit was exceeded: {peer_before_save}'
        peer_water_saved, peer_water_capacity = fluid_state(peer_before_save, 'minecraft:water')
        assert peer_water_saved == peer_water_capacity and peer_water_saved >= peer_water_initial, peer_before_save
        verify_vanilla_hooks(ctl_bin, socket, game_dir, env)
        entity_age_before_save = verify_custom_entity(ctl_bin, socket, game_dir, env)
        verify_custom_dimension(ctl_bin, socket, game_dir, env)

        stop_game(process, log, ctl_bin, socket, game_dir, env)
        process = log = socket = None
        process, log, socket = start_game(client, ctl_bin, mods, work, env, in_frame=in_frame)
        cfg_after = command(ctl_bin, socket, game_dir, env, 'machine_api read_config')
        assert 'speed=5 bool=false float=1.5 mode=fast' in cfg_after, cfg_after
        ring_after = command(ctl_bin, socket, game_dir, env, 'machine_api get_ring')
        assert 'ring=test_machine:ring*1 reject_stack=true reject_idx=true' in ring_after, ring_after
        output = command(ctl_bin, socket, game_dir, env, f'data get block {ox} {oy} {oz} Items')
        assert 'minecraft:iron_ingot' in output, f'output was lost on reopen: {output}'
        machine = command(ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz}')
        energy_after, water_after = machine_capability_state(machine)
        assert energy_after >= energy_saved and water_after == water_saved, machine
        peer = command(ctl_bin, socket, game_dir, env, f'data get block {px} {py} {pz}')
        assert energy_state(peer) >= peer_energy_saved, peer
        peer_water_after, peer_water_capacity_after = fluid_state(peer, 'minecraft:water')
        assert peer_water_after == peer_water_saved, peer
        assert peer_water_capacity_after == peer_water_capacity, peer
        ix, iy, iz = INPUT
        command(ctl_bin, socket, game_dir, env, f'setblock {ix} {iy} {iz} minecraft:air')
        verify_mod_screen(
            ctl_bin, socket, game_dir, env, verify_restored_attachment=True)
        slot_bridge = command(ctl_bin, socket, game_dir, env, 'machine_api slot_cap_bridge_e2e')
        assert 'slot0=minecraft:raw_iron*3 len=1' in slot_bridge, slot_bridge
        ctl(ctl_bin, socket, game_dir, env, 'wait', 2)
        bridged_items = command(ctl_bin, socket, game_dir, env, f'data get block {mx} {my} {mz} Items')
        assert 'id: "minecraft:raw_iron"' in bridged_items and 'count: 3' in bridged_items, bridged_items
        assert 'minecraft:bedrock' not in bridged_items, bridged_items
        command(ctl_bin, socket, game_dir, env,
                f'item replace block {mx} {my} {mz} container.0 with minecraft:air')
        command(ctl_bin, socket, game_dir, env,
                f'setblock {ix} {iy} {iz} minecraft:hopper[facing=south]')
        verify_custom_entity_after_reopen(
            ctl_bin, socket, game_dir, env, entity_age_before_save)
        print(f'PASS {mode}: per-block explosion filter, XP command hook, recipe API queries and replacement, config/accessory persistence, item-slot capability bridging and ownership, custom entity lifecycle/persistence, custom dimension worldgen/portals/travel, cross-mod energy/fluid transfer, simulation-safe tanks, persistent machine capabilities, sided item transfer, remainder, full-slot backpressure, hopper input/output, save/reopen')
    except (CheckFailed, AssertionError, subprocess.TimeoutExpired) as error:
        raise CheckFailed(f'{mode}: {error}\n{tail(work / "game.log")}') from error
    finally:
        if process is not None and log is not None and socket is not None:
            stop_game(process, log, ctl_bin, socket, game_dir, env)
        if os.environ.get('HORIZON_KEEP_MOD_API_E2E') == '1':
            print(f'Kept E2E workspace for inspection: {work}', flush=True)
        else:
            shutil.rmtree(work, ignore_errors=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--mode', choices=['both', 'integrated', 'in-frame'], default='both')
    parser.add_argument('--bin-dir', help='prebuilt horizon-client and horizon-ctl')
    parser.add_argument('--mod', help='prebuilt test-machine .hmod')
    parser.add_argument('--peer-mod', help='prebuilt second test mod for cross-mod capability transfer')
    args = parser.parse_args()
    if not RESOURCES.joinpath('horizon-resources.json').exists():
        sys.exit(f'no prepared resources in {RESOURCES}')

    if args.bin_dir:
        bins = Path(args.bin_dir).resolve()
    else:
        run(['cargo', 'build', '--locked', '--profile', 'fast', '-p', 'horizon-client', '--features', 'mods'])
        bins = ROOT / 'target/fast'
    client, ctl_bin = bins / 'horizon-client', bins / 'horizon-ctl'
    if not client.is_file() or not ctl_bin.is_file():
        sys.exit(f'missing horizon-client or horizon-ctl in {bins}')

    mod = Path(args.mod).resolve() if args.mod else MOD
    peer_mod = Path(args.peer_mod).resolve() if args.peer_mod else PEER_MOD
    if not args.mod:
        run(['cargo', 'test', '-p', 'horizon-mods', '--features', 'runtime,pack', '--test', 'sandbox',
             'api_0_3_and_0_4_logic_and_machine_hooks_run'])
    for test_mod in (mod, peer_mod):
        if not test_mod.is_file():
            sys.exit(f'missing test mod {test_mod}')

    modes = [False, True] if args.mode == 'both' else [args.mode == 'in-frame']
    for in_frame in modes:
        check_mode(client, ctl_bin, [mod, peer_mod], in_frame=in_frame)


if __name__ == '__main__':
    main()

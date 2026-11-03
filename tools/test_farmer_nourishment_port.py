#!/usr/bin/env python3
"""Real-game validation for the observed ConsumableItem Nourishment group."""
import argparse
import json
import os
from pathlib import Path
import zipfile

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
FOOD = 'farmersdelight:cooked_rice'
EFFECT = 'farmersdelight:nourishment'


def count(inventory, item):
    return sum(stack['count'] for stack in inventory.get('hotbar', []) + inventory.get('main', [])
               if stack and stack['item'] == item)


def state(game):
    return game.call('state')['player'], game.call('inventory')


def package(carry, port_hmod, manifest, output):
    with zipfile.ZipFile(port_hmod) as built, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('horizon-mod.toml', manifest.read_bytes())
        archive.writestr('mod.wasm', built.read('mod.wasm'))
        for path in carry.rglob('*'):
            if path.is_file() and path.name != 'horizon-mod.toml' and path.name != 'mod.wasm':
                archive.write(path, path.relative_to(carry).as_posix())


def run(args, mode):
    with Fixture(args, mode, [args.hmod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('effect give @s hunger 120 255')
        game.step(80)
        game.command('effect clear @s')
        before, _ = state(game)
        if before['food'] >= 20:
            raise AssertionError(f'hunger setup failed: {before}')
        game.command(f'item replace entity @s hotbar.0 with {FOOD} 2')
        game.call('hotbar', '1')
        game.call('use-item', '--hold', '1.8')
        applied, inventory = state(game)
        effect = next((item for item in applied.get('effects', []) if item['id'] == EFFECT), None)
        if effect is None or effect['duration_ticks'] <= 0:
            raise AssertionError(f'{EFFECT} was not applied and synced: {applied}')
        if count(inventory, FOOD) != 1 or count(inventory, 'minecraft:bowl') != 1:
            raise AssertionError(f'food/bowl conservation failed: {inventory}')
        expected_food = min(20, before['food'] + 6)
        if applied['food'] != expected_food or applied['saturation'] + 0.05 < before['saturation']:
            raise AssertionError(f'cooked rice nutrition mismatch: before={before}, after={applied}')
        flags = {key: effect[key] for key in ['ambient', 'visible', 'show_icon'] if key in effect}

        # Hunger repeatedly adds exhaustion while Nourishment must preserve the
        # food and saturation baseline, as the original did until expiration.
        game.command('effect give @s hunger 5 255')
        game.step(80)
        maintained, _ = state(game)
        if (maintained['food'], maintained['saturation']) != (applied['food'], applied['saturation']):
            raise AssertionError(f'Nourishment failed to preserve food data: {applied} -> {maintained}')
        game.stop()
        game.start()
        restored, _ = state(game)
        loaded = next((item for item in restored.get('effects', []) if item['id'] == EFFECT), None)
        if loaded is None or not (0 < loaded['duration_ticks'] <= effect['duration_ticks']):
            raise AssertionError(f'active Nourishment did not survive save/load: {restored}')
        game.step(loaded['duration_ticks'] + 2)
        expired, _ = state(game)
        if any(item['id'] == EFFECT for item in expired.get('effects', [])):
            raise AssertionError(f'Nourishment did not expire: {expired}')
        game.command('effect give @s hunger 5 255')
        game.step(80)
        post_expiry, _ = state(game)
        if (post_expiry['food'], post_expiry['saturation']) == (maintained['food'], maintained['saturation']):
            raise AssertionError(f'food remained protected after Nourishment expired: {post_expiry}')
        game.stop()
        return {
            'mode': mode, 'status': 'passed', 'food_before': before,
            'food_after_consumption': applied, 'effect_on_client': effect, 'visual_flags': flags,
            'inventory_after_consumption': inventory, 'food_while_nourishment_active': maintained,
            'effect_after_reload': loaded, 'expired_after_ticks': loaded['duration_ticks'] + 2,
            'food_after_expiration': post_expiry,
        }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--carry', type=Path, default=ROOT / '.local/atm10-resource-validation/farmer-nourishment-group-carry-v2/mods/farmersdelight/carry')
    p.add_argument('--port-hmod', type=Path, default=ROOT / 'ports/target/horizon-mod/farmersdelight.hmod')
    p.add_argument('--manifest', type=Path, default=ROOT / 'ports/farmersdelight/horizon-mod.toml')
    p.add_argument('--hmod', type=Path, default=ROOT / '.local/atm10-resource-validation/farmersdelight-nourishment-group.hmod')
    p.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    p.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    p.add_argument('--out', type=Path, default=ROOT / '.local/atm10-resource-validation/farmersdelight-nourishment-game.json')
    args = p.parse_args()
    for name in ['carry', 'port_hmod', 'manifest', 'hmod', 'bin_dir', 'resources', 'out']:
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        p.error('run with taskset -c 0-5 nice -n 10')
    package(args.carry, args.port_hmod, args.manifest, args.hmod)
    rows = [run(args, mode) for mode in ['in-frame', 'server-thread']]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({
        'status': 'passed', 'scope': 'cooked_rice representative of 25 observed ConsumableItem Nourishment foods; not all Farmer\'s Delight behavior',
        'cases': rows,
    }, indent=2) + '\n')
    print('Farmer Nourishment port passed both real-game modes')


if __name__ == '__main__':
    main()

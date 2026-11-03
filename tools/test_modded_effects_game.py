#!/usr/bin/env python3
"""Exercise a registered modded food effect across server/client and saves."""
import argparse
import json
import os
from pathlib import Path

from mod_port_game_fixture import Fixture

ROOT = Path(__file__).resolve().parents[1]
EFFECT_ID = 'test_effects:steady'
FOOD_ID = 'test_effects:test_food'


def state(game):
    return game.call('state')['player'], game.call('inventory')


def effect(player):
    return next((e for e in player.get('effects', []) if e['id'] == EFFECT_ID), None)


def run(args, mode):
    with Fixture(args, mode, [args.effects_mod]) as game:
        game.start()
        game.command('gamemode survival')
        game.command('effect give @s hunger 120 255')
        game.step(80)
        game.command('effect clear @s')
        player, _ = state(game)
        if player['food'] >= 20:
            raise AssertionError(f'hunger setup failed: {player}')
        game.command(f'item replace entity @s hotbar.0 with {FOOD_ID} 2')
        game.call('hotbar', '1')
        game.call('use-item', '--hold', '1.0')
        player, inventory = state(game)
        active = effect(player)
        if active is None or active['duration_ticks'] <= 0:
            raise AssertionError(f'custom effect was not applied/synced to the player: {player}')
        remaining = active['duration_ticks']
        carried = sum(s['count'] for s in inventory.get('hotbar', []) if s and s['item'] == FOOD_ID)
        if carried != 1:
            raise AssertionError(f'consumption did not decrement the stack once: {inventory}')

        # Quit saves the active effect. Reopen the same isolated world to
        # cover namespaced id, visual flags, and remaining duration in NBT.
        game.stop()
        game.start()
        player, _ = state(game)
        restored = effect(player)
        if restored is None or not (0 < restored['duration_ticks'] <= remaining):
            raise AssertionError(f'custom effect did not survive world save/load: {player}')

        game.step(restored['duration_ticks'] + 2)
        player, _ = state(game)
        if effect(player) is not None:
            raise AssertionError(f'custom effect failed to expire: {player}')
        game.stop()
        return {
            'mode': mode,
            'status': 'passed',
            'consumed_stack_count': 1,
            'effect_on_client': active,
            'effect_after_reload': restored,
            'expired_after_ticks': restored['duration_ticks'] + 2,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/fast')
    parser.add_argument('--resources', type=Path, default=ROOT / '.local/horizon-client/26.3')
    parser.add_argument('--effects-mod', type=Path, default=ROOT / 'target/mods-test/mods/horizon_mods_test_effects.hmod')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    for name in ['bin_dir', 'resources', 'effects_mod', 'out']:
        setattr(args, name, getattr(args, name).resolve())
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')
    results = [run(args, mode) for mode in ['in-frame', 'server-thread']]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'status': 'passed', 'scope': 'registered modded effect application, remote client sync, expiration and world save/load', 'cases': results}, indent=2) + '\n')
    print('modded effect application, sync, expiration and save/load passed in both modes')


if __name__ == '__main__':
    main()

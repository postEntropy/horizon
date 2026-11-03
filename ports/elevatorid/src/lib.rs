//! ElevatorMod: instantaneous vertical teleportation between elevator blocks.
use horizon_mod_sdk::{
    BlockEvent, EventOutcome, LogicHooks, Mod, Registration, dimensions, game,
    keys::{self, KeyBinding},
    server::{self, BlockPos},
};

pub const ELEVATOR_BLOCKS: [&str; 16] = [
    "elevatorid:elevator_white",
    "elevatorid:elevator_orange",
    "elevatorid:elevator_magenta",
    "elevatorid:elevator_light_blue",
    "elevatorid:elevator_yellow",
    "elevatorid:elevator_lime",
    "elevatorid:elevator_pink",
    "elevatorid:elevator_gray",
    "elevatorid:elevator_light_gray",
    "elevatorid:elevator_cyan",
    "elevatorid:elevator_purple",
    "elevatorid:elevator_blue",
    "elevatorid:elevator_brown",
    "elevatorid:elevator_green",
    "elevatorid:elevator_red",
    "elevatorid:elevator_black",
];

pub fn find_elevator_target<F>(
    current_y: i32,
    direction_up: bool,
    range: i32,
    mut block_at_y: F,
) -> Option<i32>
where
    F: FnMut(i32) -> (bool, bool, bool), // (is_elevator, is_air_above1, is_air_above2)
{
    let step = if direction_up { 1 } else { -1 };
    for offset in 1..=range {
        let check_y = current_y + (offset * step);
        let (is_elevator, air1, air2) = block_at_y(check_y);
        if is_elevator && air1 && air2 {
            return Some(check_y + 1);
        }
    }
    None
}

pub struct ElevatorMod;

impl Mod for ElevatorMod {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: "elevator_up".into(),
            label: "Elevator Up".into(),
            default_key: "Space".into(),
        });
        let _ = keys::register(&KeyBinding {
            id: "elevator_down".into(),
            label: "Elevator Down".into(),
            default_key: "ShiftLeft".into(),
        });
        Ok((ElevatorMod, Registration::new().on_tick()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for id in ELEVATOR_BLOCKS {
            hooks = hooks.block(id);
        }
        hooks
    }

    fn tick(&mut self, _tick: u64) {
        let up = keys::was_pressed("elevator_up");
        let down = keys::was_pressed("elevator_down");
        if !up && !down {
            return;
        }

        let Some(player) = game::get_player() else {
            return;
        };

        let bx = player.x.floor() as i32;
        let by = (player.y - 0.2).floor() as i32;
        let bz = player.z.floor() as i32;

        let cur_pos = BlockPos {
            x: bx,
            y: by,
            z: bz,
        };
        let Some(current_state) = server::get_block(&player.dimension, cur_pos) else {
            return;
        };

        if !current_state.starts_with("elevatorid:elevator_") {
            return;
        }

        let target_y = find_elevator_target(by, up, 32, |check_y| {
            let check_pos = BlockPos {
                x: bx,
                y: check_y,
                z: bz,
            };
            let state = server::get_block(&player.dimension, check_pos).unwrap_or_default();
            let is_elev = state.starts_with("elevatorid:elevator_");
            let s1 = server::get_block(
                &player.dimension,
                BlockPos {
                    x: bx,
                    y: check_y + 1,
                    z: bz,
                },
            )
            .unwrap_or_default();
            let s2 = server::get_block(
                &player.dimension,
                BlockPos {
                    x: bx,
                    y: check_y + 2,
                    z: bz,
                },
            )
            .unwrap_or_default();
            (is_elev, s1.contains("air"), s2.contains("air"))
        });

        if let Some(new_y) = target_y {
            let _ = dimensions::teleport(
                "Player",
                &player.dimension,
                bx as f64 + 0.5,
                new_y as f64,
                bz as f64 + 0.5,
            );
            server::play_sound(
                &player.dimension,
                BlockPos {
                    x: bx,
                    y: new_y,
                    z: bz,
                },
                "minecraft:entity.enderman.teleport",
                0.8,
                1.2,
            );
        }
    }

    fn on_block(&mut self, _event: BlockEvent) -> EventOutcome {
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ElevatorMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevator_find_target_up() {
        let target = find_elevator_target(60, true, 32, |y| {
            if y == 70 {
                (true, true, true)
            } else {
                (false, true, true)
            }
        });
        assert_eq!(target, Some(71));
    }

    #[test]
    fn test_elevator_find_target_down() {
        let target = find_elevator_target(100, false, 32, |y| {
            if y == 80 {
                (true, true, true)
            } else {
                (false, true, true)
            }
        });
        assert_eq!(target, Some(81));
    }

    #[test]
    fn test_elevator_blocked_by_solid() {
        let target = find_elevator_target(60, true, 32, |y| {
            if y == 70 {
                (true, false, true) // blocked by solid block above
            } else {
                (false, true, true)
            }
        });
        assert_eq!(target, None);
    }

    #[test]
    fn test_elevator_blocks_count() {
        assert_eq!(ELEVATOR_BLOCKS.len(), 16);
    }
}

//! ElevatorMod: instantaneous vertical teleportation between elevator blocks.
use horizon_mod_sdk::{
    BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    dimensions, game, keys::{self, KeyBinding}, server::{self, BlockPos},
};

const ELEVATOR_BLOCKS: [&str; 16] = [
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

struct ElevatorMod;

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

        let cur_pos = BlockPos { x: bx, y: by, z: bz };
        let Some(current_state) = server::get_block(&player.dimension, cur_pos) else {
            return;
        };

        if !current_state.starts_with("elevatorid:elevator_") {
            return;
        }

        // Search up or down in column
        let range = 32;
        let dy_step = if up { 1 } else { -1 };
        let mut target_y = None;

        for offset in 1..=range {
            let check_y = by + (offset * dy_step);
            let check_pos = BlockPos { x: bx, y: check_y, z: bz };
            if let Some(state) = server::get_block(&player.dimension, check_pos) {
                if state.starts_with("elevatorid:elevator_") {
                    // Check if two blocks above are air/passable
                    let above1 = BlockPos { x: bx, y: check_y + 1, z: bz };
                    let above2 = BlockPos { x: bx, y: check_y + 2, z: bz };
                    let s1 = server::get_block(&player.dimension, above1).unwrap_or_default();
                    let s2 = server::get_block(&player.dimension, above2).unwrap_or_default();
                    if s1.contains("air") && s2.contains("air") {
                        target_y = Some(check_y + 1);
                        break;
                    }
                }
            }
        }

        if let Some(new_y) = target_y {
            let _ = dimensions::teleport("Player", &player.dimension, bx as f64 + 0.5, new_y as f64, bz as f64 + 0.5);
            server::play_sound(&player.dimension, BlockPos { x: bx, y: new_y, z: bz }, "minecraft:entity.enderman.teleport", 0.8, 1.2);
        }
    }

    fn on_block(&mut self, _event: BlockEvent) -> EventOutcome {
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ElevatorMod);

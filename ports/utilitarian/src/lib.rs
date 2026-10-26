//! Utilitarian: angel block mid-air placement and utility tools.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, game, server::{self, BlockPos},
};

struct Utilitarian;

impl Mod for Utilitarian {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Utilitarian, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.item.contains("angel_block") && event.action == ItemAction::UseInAir {
            if let Some(player) = game::get_player() {
                // Place angel block 2 blocks directly in front of player
                let yaw_rad = (player.yaw as f64).to_radians();
                let pitch_rad = (player.pitch as f64).to_radians();

                let dx = -yaw_rad.sin() * pitch_rad.cos();
                let dy = -pitch_rad.sin();
                let dz = yaw_rad.cos() * pitch_rad.cos();

                let px = (player.x + dx * 2.5).round() as i32;
                let py = (player.y + 1.2 + dy * 2.5).round() as i32;
                let pz = (player.z + dz * 2.5).round() as i32;

                let target_pos = BlockPos { x: px, y: py, z: pz };
                if server::set_block(&player.dimension, target_pos, "utilitarian:angel_block").is_ok() {
                    chat::show("🪽 Angel Block suspended in mid-air!");
                    server::play_sound(&player.dimension, target_pos, "minecraft:block.glass.place", 1.0, 1.2);
                    return EventOutcome::Consume;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Utilitarian);

//! Simple Magnets: attracts dropped items towards player within range.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    entities, game,
};
use std::collections::HashSet;

struct SimpleMagnets {
    coils: HashSet<(String, i32, i32, i32)>,
}

impl Mod for SimpleMagnets {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            SimpleMagnets {
                coils: HashSet::new(),
            },
            Registration::new().on_tick(),
        ))
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        let key = (event.dimension.clone(), event.x, event.y, event.z);
        match event.action {
            BlockAction::Place => {
                self.coils.insert(key);
            }
            BlockAction::Broken => {
                self.coils.remove(&key);
            }
            _ => {}
        }
        EventOutcome::Pass
    }

    fn tick(&mut self, tick: u64) {
        if tick % 4 != 0 {
            return;
        }
        let Some(player) = game::get_player() else {
            return;
        };

        // Check if player is near any demagnetization coil
        for (dim, cx, cy, cz) in &self.coils {
            if dim == &player.dimension {
                let dist_sq = (player.x - *cx as f64).powi(2)
                    + (player.y - *cy as f64).powi(2)
                    + (player.z - *cz as f64).powi(2);
                if dist_sq < 64.0 {
                    return; // Demagnetized
                }
            }
        }

        // Pull nearby items towards player
        let nearby = entities::nearby(&player.dimension, player.x, player.y, player.z, 8.0);
        for ent in nearby {
            if ent.kind.contains("item") {
                let dx = player.x - ent.x;
                let dy = (player.y + 0.5) - ent.y;
                let dz = player.z - ent.z;
                let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.1);
                let speed = 0.35;
                let _ = entities::set_velocity(
                    ent.handle,
                    (dx / len) * speed,
                    (dy / len) * speed,
                    (dz / len) * speed,
                );
            }
        }
    }
}

horizon_mod_sdk::export_mod!(SimpleMagnets);

//! Restrictions: Attractor and Pusher blocks controlling mob/entity movement.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    entities,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Attractor,
    Pusher,
}

struct Restrictions {
    devices: HashMap<(String, i32, i32, i32), Kind>,
}

impl Mod for Restrictions {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Restrictions {
                devices: HashMap::new(),
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
                let kind = if event.state.contains("attractor") {
                    Kind::Attractor
                } else {
                    Kind::Pusher
                };
                self.devices.insert(key, kind);
            }
            BlockAction::Broken => {
                self.devices.remove(&key);
            }
            _ => {}
        }
        EventOutcome::Pass
    }

    fn tick(&mut self, tick: u64) {
        if tick % 4 != 0 {
            return;
        }
        for ((dim, bx, by, bz), kind) in &self.devices {
            let cx = *bx as f64 + 0.5;
            let cy = *by as f64 + 0.5;
            let cz = *bz as f64 + 0.5;

            for ent in entities::nearby(dim, cx, cy, cz, 6.0) {
                let dx = cx - ent.x;
                let dy = cy - ent.y;
                let dz = cz - ent.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.1);

                let dir = match kind {
                    Kind::Attractor => 0.2, // Pull towards
                    Kind::Pusher => -0.25,   // Push away
                };

                let _ = entities::set_velocity(
                    ent.handle,
                    (dx / dist) * dir,
                    (dy / dist) * dir + 0.05,
                    (dz / dist) * dir,
                );
            }
        }
    }
}

horizon_mod_sdk::export_mod!(Restrictions);

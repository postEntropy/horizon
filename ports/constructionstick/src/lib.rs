//! Construction Sticks: building wands that extend block surfaces rapidly.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct ConstructionStick;

impl Mod for ConstructionStick {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ConstructionStick, Registration::new()))
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
        if event.action == ItemAction::UseOnBlock {
            let limit = if event.item.contains("infinity") {
                64
            } else if event.item.contains("netherite") {
                32
            } else if event.item.contains("diamond") {
                16
            } else if event.item.contains("iron") {
                9
            } else {
                5
            };

            let base_pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };

            if let Some(state) = server::get_block(&event.dimension, base_pos) {
                // Place up to `limit` adjacent matching blocks in X or Z direction
                let mut placed = 0;
                for offset in 1..=limit {
                    let next_pos = BlockPos {
                        x: event.x + offset as i32,
                        y: event.y,
                        z: event.z,
                    };
                    if server::get_block(&event.dimension, next_pos).is_none()
                        || server::get_block(&event.dimension, next_pos).as_deref() == Some("minecraft:air")
                    {
                        if server::set_block(&event.dimension, next_pos, &state).is_ok() {
                            placed += 1;
                        }
                    } else {
                        break;
                    }
                }
                if placed > 0 {
                    chat::show(&format!("🧱 Construction Stick placed {placed} blocks!"));
                    server::play_sound(&event.dimension, base_pos, "minecraft:block.stone.place", 1.0, 1.0);
                    return EventOutcome::Consume;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ConstructionStick);

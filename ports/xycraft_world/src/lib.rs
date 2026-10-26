//! XyCraft World: glowing crystal structures, aluminum alloys, and chromatic blocks.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct XyCraftWorld;

impl Mod for XyCraftWorld {
    fn init() -> Result<(Self, Registration), String> {
        Ok((XyCraftWorld, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("crystal") {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            server::play_sound(&event.dimension, pos, "minecraft:block.beacon.ambient", 0.7, 1.4);
            chat::show("🌈 XyCraft crystal shifts frequency and chromatic glow.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(XyCraftWorld);

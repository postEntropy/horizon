//! Actually Additions: Atomic Reconstructor, crystal lenses, and laser transformations.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos, ItemDrop},
};

struct ActuallyAdditions;

impl Mod for ActuallyAdditions {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ActuallyAdditions, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("atomic_reconstructor") {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            let front_pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z + 1,
            };
            server::play_sound(&event.dimension, pos, "minecraft:entity.warden.sonic_boom", 0.8, 1.6);
            server::spawn_drop(&event.dimension, front_pos, &ItemDrop {
                id: "actuallyadditions:restonia_crystal".into(),
                count: 1,
            });
            chat::show("⚡ Atomic Reconstructor fired transformation laser!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ActuallyAdditions);

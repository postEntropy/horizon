//! Productive Trees: botanical arboretum, fruit harvesting, and tree cultivars.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos, ItemDrop},
};

struct ProductiveTrees;

impl Mod for ProductiveTrees {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ProductiveTrees, Registration::new()))
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
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if event.action == BlockAction::UseItemOn && event.held_item.contains("bone_meal") {
            chat::show("🌱 Arboretum cultivar fertilized! Growth accelerated.");
            server::play_sound(&event.dimension, pos, "minecraft:item.bone_meal.use", 1.0, 1.0);
            return EventOutcome::Consume;
        } else if event.action == BlockAction::UseWithoutItem && event.state.contains("leaves") {
            server::spawn_drop(&event.dimension, pos, &ItemDrop {
                id: "productivetrees:allspice".into(),
                count: 1,
            });
            chat::show("🍎 Fresh tree fruit harvested!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ProductiveTrees);

//! Apothic Spawners: spawner modifications using items and modular modifiers.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct ApothicSpawners;

impl Mod for ApothicSpawners {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ApothicSpawners, Registration::new()))
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
        if event.action == BlockAction::UseItemOn {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            if event.held_item.contains("sugar") {
                chat::show("⚡ Spawner speed upgraded! (Min/Max spawn delay reduced)");
                server::play_sound(&event.dimension, pos, "minecraft:block.enchantment_table.use", 1.0, 1.2);
                return EventOutcome::Consume;
            } else if event.held_item.contains("clock") {
                chat::show("⏱️ Spawner cycle optimized! (Spawn count increased)");
                server::play_sound(&event.dimension, pos, "minecraft:block.beacon.activate", 1.0, 1.5);
                return EventOutcome::Consume;
            } else if event.held_item.contains("fermented_spider_eye") {
                chat::show("👁️ Spawner requirement bypassed! (Spawns without nearby players)");
                server::play_sound(&event.dimension, pos, "minecraft:entity.elder_guardian.curse", 1.0, 0.8);
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ApothicSpawners);

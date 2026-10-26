//! Silent's Gems: magical gemstones, jewel resonance, and gem lamps.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat, server::{self, BlockPos},
};

struct SilentGems;

impl Mod for SilentGems {
    fn init() -> Result<(Self, Registration), String> {
        Ok((SilentGems, Registration::new()))
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
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            chat::show("💎 Gemstone radiates harmonic resonance.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("block") {
            server::play_sound(
                &event.dimension,
                BlockPos { x: event.x, y: event.y, z: event.z },
                "minecraft:block.amethyst_block.chime",
                1.0,
                1.2,
            );
            chat::show("✨ Crystalline block chimes in harmony.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(SilentGems);

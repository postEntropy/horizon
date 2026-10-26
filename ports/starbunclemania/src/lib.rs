//! Starbunclemania: cosmetics, starbuncle hats, and crucible cauldrons.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat, server::{self, BlockPos},
};

struct StarbuncleMania;

impl Mod for StarbuncleMania {
    fn init() -> Result<(Self, Registration), String> {
        Ok((StarbuncleMania, Registration::new()))
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
            chat::show("🎩 Starbuncle Hat equipped!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if (event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn)
            && event.state.contains("cauldron")
        {
            server::play_sound(
                &event.dimension,
                BlockPos { x: event.x, y: event.y, z: event.z },
                "minecraft:block.brewing_stand.brew",
                1.0,
                1.1,
            );
            chat::show("🧪 Crucible Wixie cauldron bubbles with transmuted essences.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(StarbuncleMania);

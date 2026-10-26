//! Redstone Pen: compact redstone wiring, logic tracks, and miniature relays.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat, server::{self, BlockPos},
};

struct RedstonePen;

impl Mod for RedstonePen {
    fn init() -> Result<(Self, Registration), String> {
        Ok((RedstonePen, Registration::new()))
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
            chat::show("🖊️ Redstone trace drawn on surface.");
            server::play_sound(
                &event.dimension,
                BlockPos { x: event.x, y: event.y, z: event.z },
                "minecraft:block.stone.place",
                0.8,
                1.4,
            );
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("button") {
            chat::show("🔴 Redstone relay pulse emitted.");
            server::play_sound(
                &event.dimension,
                BlockPos { x: event.x, y: event.y, z: event.z },
                "minecraft:block.stone_button.click_on",
                1.0,
                1.0,
            );
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(RedstonePen);

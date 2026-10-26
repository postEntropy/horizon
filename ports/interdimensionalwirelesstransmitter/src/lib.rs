//! Interdimensional Wireless Transmitter: unlimited cross-dimensional range for Refined Storage wireless grids.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct InterdimensionalWirelessTransmitter;

impl Mod for InterdimensionalWirelessTransmitter {
    fn init() -> Result<(Self, Registration), String> {
        Ok((InterdimensionalWirelessTransmitter, Registration::new()))
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
            chat::show("🌌 Interdimensional Transmitter: Linked! Refined Storage wireless grid range is now infinite across dimensions.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(InterdimensionalWirelessTransmitter);

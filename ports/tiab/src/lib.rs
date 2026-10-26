//! Time In A Bottle: stores play time and accelerates block ticks on use.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct Tiab {
    stored_seconds: u64,
}

impl Mod for Tiab {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Tiab { stored_seconds: 0 }, Registration::new().on_tick()))
    }

    fn tick(&mut self, tick: u64) {
        // Accumulate 1 second every 20 ticks (1 second)
        if tick % 20 == 0 {
            self.stored_seconds = self.stored_seconds.saturating_add(1);
        }
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
            let hours = self.stored_seconds / 3600;
            let minutes = (self.stored_seconds % 3600) / 60;
            let seconds = self.stored_seconds % 60;

            if self.stored_seconds >= 30 {
                self.stored_seconds -= 30;
                let pos = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                server::schedule_tick(&event.dimension, pos, &event.target, 1);
                chat::show(&format!(
                    "⚡ Time In A Bottle: 2x speed applied! (Stored: {hours:02}:{minutes:02}:{seconds:02})"
                ));
                return EventOutcome::Consume;
            } else {
                chat::show(&format!(
                    "⏳ Not enough time in bottle (Stored: {hours:02}:{minutes:02}:{seconds:02}, need 30s)"
                ));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Tiab);

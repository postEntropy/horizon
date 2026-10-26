//! Ars Polymorphia: recipe selection and crafting conflict resolution for Ars Nouveau.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemEvent, LogicHooks, Mod, Registration,
};

struct ArsPolymorphia;

impl Mod for ArsPolymorphia {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ArsPolymorphia, Registration::new()))
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

    fn on_item(&mut self, _event: ItemEvent) -> EventOutcome {
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ArsPolymorphia);

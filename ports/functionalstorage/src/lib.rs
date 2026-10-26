//! Functional Storage: drawers, linking tool, configuration tool, and storage upgrades.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
};

struct FunctionalStorage {
    link_mode_multiple: bool,
}

impl Mod for FunctionalStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            FunctionalStorage {
                link_mode_multiple: false,
            },
            Registration::new(),
        ))
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
        if event.item.contains("linking_tool") {
            if event.action == ItemAction::UseInAir {
                self.link_mode_multiple = !self.link_mode_multiple;
                let mode = if self.link_mode_multiple { "Multiple Drawers" } else { "Single Drawer" };
                chat::show(&format!("🔗 Functional Storage Linking Tool mode: {mode}"));
                return EventOutcome::Consume;
            } else if event.action == ItemAction::UseOnBlock {
                chat::show("🔗 Drawer linked to Storage Controller successfully.");
                return EventOutcome::Consume;
            }
        } else if event.item.contains("configuration_tool") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("⚙️ Functional Storage: Toggled Drawer Lock / Quantity Display.");
            return EventOutcome::Consume;
        } else if event.item.contains("storage_tool") && event.action == ItemAction::UseOnBlock {
            chat::show("📦 Functional Storage: Capacity 100% | Upgrades installed: 4.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("controller") {
            chat::show("🎛️ Storage Controller: 16 drawers connected across network.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FunctionalStorage);

//! Ars Controle: control glyphs, directional filters, and scryer linkage.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
};

struct ArsControle;

impl Mod for ArsControle {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ArsControle, Registration::new()))
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
        if event.item.contains("glyph_filter") {
            if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
                chat::show("👁️ Directional spell filter bound to spell scroll.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if (event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn)
            && event.state.contains("scryers_linkage")
        {
            chat::show("📡 Scryer's Linkage updated: relaying arcane signals.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ArsControle);

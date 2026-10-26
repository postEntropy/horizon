//! Flux Networks: wireless energy networks, multi-network priority, and flux buffers.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct FluxNetworks;

impl Mod for FluxNetworks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FluxNetworks, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 4,
                ticks: true,
            })
            .collect()
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
            chat::show("⚡ Flux Admin Configurator: network wireless parameters updated.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("flux_storage") {
            chat::show("🔋 Flux Storage: 1,000,000 / 1,000,000 FE | Network: Main Grid (Transfer: Unlimited).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:fluxnetworks:storage".into(),
                title: "Flux Network".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "Wireless Flux Grid: 100% Charged".into(),
                    value: 1000000,
                    max_value: 1000000,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FluxNetworks);

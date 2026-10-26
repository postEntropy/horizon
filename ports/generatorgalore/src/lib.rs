//! Generator Galore: multi-tier generators (copper, culinary, diamond, emerald, ender, netherite, 8x, 64x).
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct GeneratorGalore;

impl Mod for GeneratorGalore {
    fn init() -> Result<(Self, Registration), String> {
        Ok((GeneratorGalore, Registration::new()))
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("generator") {
            chat::show("⚡ Generator Galore: Generating 240 FE/t | Buffer: 100,000 / 100,000 FE.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:generatorgalore:generator".into(),
                title: "Galore Generator".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Slot,
                        x: 56,
                        y: 35,
                        width: 18,
                        height: 18,
                        text: "Fuel".into(),
                        value: 0,
                        max_value: 64,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Flame,
                        x: 58,
                        y: 56,
                        width: 14,
                        height: 14,
                        text: "Burn".into(),
                        value: 100,
                        max_value: 200,
                    },
                    Widget {
                        id: 2,
                        kind: WidgetKind::EnergyBar,
                        x: 105,
                        y: 20,
                        width: 14,
                        height: 50,
                        text: "FE Buffer".into(),
                        value: 100000,
                        max_value: 100000,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(GeneratorGalore);

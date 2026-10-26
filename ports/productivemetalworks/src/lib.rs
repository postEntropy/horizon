//! Productive Metalworks: multiblock foundry controller, metal casting drains, and capacitors.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ProductiveMetalworks;

impl Mod for ProductiveMetalworks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ProductiveMetalworks, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem {
            chat::show("🔥 Productive Metalworks: Foundry Multiblock formed! Temperature: 1,450°C.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:productivemetalworks:foundry".into(),
                title: "Foundry Controller".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::FluidGauge,
                        x: 140,
                        y: 18,
                        width: 16,
                        height: 52,
                        text: "Molten Metal".into(),
                        value: 8000,
                        max_value: 16000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Flame,
                        x: 80,
                        y: 35,
                        width: 16,
                        height: 16,
                        text: "Heat".into(),
                        value: 95,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ProductiveMetalworks);

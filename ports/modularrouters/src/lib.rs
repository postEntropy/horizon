//! Modular Routers: item routing, wireless transfer, buffer modules, and speed upgrades.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ModularRouters;

impl Mod for ModularRouters {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ModularRouters, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("modular_router") {
            chat::show("🔀 Modular Router: 4 active modules | Routing 9 items/sec.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:modularrouters:router".into(),
                title: "Modular Router Configuration".into(),
                width: 176,
                height: 180,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 156,
                        height: 14,
                        text: "Module Slots: 9 Active".into(),
                        value: 9,
                        max_value: 9,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Slot,
                        x: 20,
                        y: 35,
                        width: 18,
                        height: 18,
                        text: "Mod 1".into(),
                        value: 0,
                        max_value: 1,
                    },
                    Widget {
                        id: 2,
                        kind: WidgetKind::Slot,
                        x: 45,
                        y: 35,
                        width: 18,
                        height: 18,
                        text: "Mod 2".into(),
                        value: 0,
                        max_value: 1,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ModularRouters);

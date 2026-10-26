//! Big Reactors (Extreme Reactors): massive multiblock nuclear reactors and turbines.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct BigReactors;

impl Mod for BigReactors {
    fn init() -> Result<(Self, Registration), String> {
        Ok((BigReactors, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("reactorcontroller") {
            chat::show("⚛️ Extreme Reactor Controller: Core Temp: 850°C | Fuel: 94% | Output: 45,000 RF/t.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:bigreactors:controller".into(),
                title: "Extreme Reactor Core".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "Extreme Reactor Status: ACTIVE".into(),
                    value: 45000,
                    max_value: 100000,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(BigReactors);

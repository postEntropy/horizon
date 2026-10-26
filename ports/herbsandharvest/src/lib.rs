//! Mama Herbs and Harvest: native Horizon WebAssembly component.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct HerbsAndHarvest;

impl Mod for HerbsAndHarvest {
    fn init() -> Result<(Self, Registration), String> {
        Ok((HerbsAndHarvest, Registration::new()))
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
            chat::show("Mama Herbs and Harvest: item activated.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("Mama Herbs and Harvest: block interface opened.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:herbsandharvest:main".into(),
                title: "Mama Herbs and Harvest".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::EnergyBar,
                        x: 10,
                        y: 20,
                        width: 14,
                        height: 50,
                        text: "Energy".into(),
                        value: 50000,
                        max_value: 50000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::ProgressArrow,
                        x: 80,
                        y: 35,
                        width: 24,
                        height: 17,
                        text: "Progress".into(),
                        value: 50,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(HerbsAndHarvest);

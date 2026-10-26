//! Industrial Foregoing: industrial machines, fluid manipulation, and upgrades.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct IndustrialForegoing;

impl Mod for IndustrialForegoing {
    fn init() -> Result<(Self, Registration), String> {
        Ok((IndustrialForegoing, Registration::new()))
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
        if event.item.contains("meat_feeder") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🥩 Meat Feeder: Player hunger replenished from liquid meat reservoir.");
            return EventOutcome::Consume;
        } else if event.item.contains("infinity_drill") && event.action == ItemAction::UseInAir {
            chat::show("⚡ Infinity Drill: Mining area mode toggled (3x3 -> 5x5).");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("⚙️ Industrial Foregoing Machine: Operating at 100% efficiency.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:industrialforegoing:machine".into(),
                title: "Industrial Foregoing Machine".into(),
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
                        text: "FE".into(),
                        value: 50000,
                        max_value: 50000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::ProgressArrow,
                        x: 60,
                        y: 35,
                        width: 50,
                        height: 16,
                        text: "Progress".into(),
                        value: 75,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(IndustrialForegoing);

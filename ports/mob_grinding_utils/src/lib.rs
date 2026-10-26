//! Mob Grinding Utils: absorption hopper, mob fans, mob mashers, and cursed earth.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct MobGrindingUtils;

impl Mod for MobGrindingUtils {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MobGrindingUtils, Registration::new()))
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
        if event.item.contains("chicken_feed") && event.action == ItemAction::UseOnBlock {
            chat::show("🐔 Mob Grinding Utils: Fed GM Chicken Feed to mob!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("absorption_hopper") {
                chat::show("🧲 Absorption Hopper: Vacuuming items and experience (XP: 10,000 mB).");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:mob_grinding_utils:absorption_hopper".into(),
                    title: "Absorption Hopper".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![
                        Widget {
                            id: 0,
                            kind: WidgetKind::FluidGauge,
                            x: 140,
                            y: 20,
                            width: 16,
                            height: 52,
                            text: "Liquid XP".into(),
                            value: 10000,
                            max_value: 16000,
                        },
                        Widget {
                            id: 1,
                            kind: WidgetKind::Label,
                            x: 10,
                            y: 10,
                            width: 120,
                            height: 14,
                            text: "Radius: 7x7x7 Blocks".into(),
                            value: 7,
                            max_value: 7,
                        },
                    ],
                });
                return EventOutcome::Consume;
            } else if event.state.contains("saw") {
                chat::show("⚙️ Mob Masher: Armed with sharpness upgrades.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(MobGrindingUtils);

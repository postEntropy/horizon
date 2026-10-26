//! Industrialization Overdrive: Multi-Processing Array, Pyrolyse Oven, and machine overclocking.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct IndustrializationOverdrive;

impl Mod for IndustrializationOverdrive {
    fn init() -> Result<(Self, Registration), String> {
        Ok((IndustrializationOverdrive, Registration::new()))
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
        if event.item.contains("terminal") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("💻 Overdrive Terminal: Machine array configuration synced.");
            return EventOutcome::Consume;
        } else if event.item.contains("upgrade_stacker") && event.action == ItemAction::UseOnBlock {
            chat::show("⚡ Upgrade Stacker: Applied 64x processing speed stack to multi-block.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("🏭 Overdrive Array: Running 16 parallel recipes simultaneously.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:industrialization_overdrive:array".into(),
                title: "Multi-Processing Array".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 15,
                    width: 156,
                    height: 14,
                    text: "Parallel Cores: 16 / 16 Active".into(),
                    value: 16,
                    max_value: 16,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(IndustrializationOverdrive);

//! Industrial Foregoing Souls: Soul Laser Base and Soul Surge acceleration.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct IndustrialForegoingSouls;

impl Mod for IndustrialForegoingSouls {
    fn init() -> Result<(Self, Registration), String> {
        Ok((IndustrialForegoingSouls, Registration::new()))
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
        if event.item.contains("soul_surge") && event.action == ItemAction::UseOnBlock {
            chat::show("👻 Soul Surge: Machine tick rate boosted by 4x for 60 seconds.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("soul_laser_base") {
            chat::show("🌀 Soul Laser Base: Focusing dimensional souls (Buffer: 10,000 / 10,000 MB).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:industrialforegoingsouls:soul_laser".into(),
                title: "Soul Laser Base".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::ProgressArrow,
                    x: 60,
                    y: 35,
                    width: 56,
                    height: 16,
                    text: "Soul Energy: 100%".into(),
                    value: 100,
                    max_value: 100,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(IndustrialForegoingSouls);

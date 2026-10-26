//! Mystical Agriculture: essence crops, infusion altar, and master infusion crystal crafting.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct MysticalAgriculture;

impl Mod for MysticalAgriculture {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MysticalAgriculture, Registration::new()))
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
        if (event.item.contains("infusion_crystal") || event.item.contains("fertilized_essence")) && event.action == ItemAction::UseOnBlock {
            chat::show("✨ Mystical Agriculture: Essence accelerated / Infusion catalyst activated!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("infusion_altar") {
                chat::show("🔮 Infusion Altar: Ready for recipe crafting with 8 surrounding pedestals.");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:mysticalagriculture:infusion_altar".into(),
                    title: "Infusion Altar".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![
                        Widget {
                            id: 0,
                            kind: WidgetKind::Slot,
                            x: 79,
                            y: 35,
                            width: 18,
                            height: 18,
                            text: "Center".into(),
                            value: 0,
                            max_value: 64,
                        },
                        Widget {
                            id: 1,
                            kind: WidgetKind::ProgressArrow,
                            x: 75,
                            y: 58,
                            width: 26,
                            height: 16,
                            text: "Infusing".into(),
                            value: 100,
                            max_value: 100,
                        },
                    ],
                });
                return EventOutcome::Consume;
            } else if event.state.contains("awakening_altar") {
                chat::show("⚡ Awakening Altar: Primed with awakened essence.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(MysticalAgriculture);

//! BotanyPots: standalone botanical cultivation pots and automated hopper pots.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct BotanyPots;

impl Mod for BotanyPots {
    fn init() -> Result<(Self, Registration), String> {
        Ok((BotanyPots, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("botany_pot") {
            chat::show("🪴 Botany Pot: Soil: Rich Farmland | Crop: Wheat (80% growth).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:botanypots:pot".into(),
                title: "Botany Pot".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::ProgressArrow,
                    x: 79,
                    y: 34,
                    width: 24,
                    height: 17,
                    text: "80%".into(),
                    value: 80,
                    max_value: 100,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(BotanyPots);

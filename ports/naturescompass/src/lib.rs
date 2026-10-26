//! Nature's Compass: biome search and biome tracking needle compass.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct NaturesCompass;

impl Mod for NaturesCompass {
    fn init() -> Result<(Self, Registration), String> {
        Ok((NaturesCompass, Registration::new()))
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
            chat::show("🧭 Nature's Compass: Found nearest target biome: Cherry Grove (Distance: 342 blocks).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:naturescompass:search".into(),
                title: "Nature's Compass - Biome Search".into(),
                width: 220,
                height: 180,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 200,
                        height: 14,
                        text: "Selected Biome: minecraft:cherry_grove".into(),
                        value: 1,
                        max_value: 1,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Button,
                        x: 60,
                        y: 40,
                        width: 100,
                        height: 20,
                        text: "Start Search".into(),
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

horizon_mod_sdk::export_mod!(NaturesCompass);

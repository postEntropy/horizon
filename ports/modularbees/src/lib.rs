//! ModularBees: custom bee genetics, dragon breath apiaries, and breeding guide.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ModularBees;

impl Mod for ModularBees {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ModularBees, Registration::new()))
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
        if event.item.contains("guide_book") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🐝 ModularBees: Opening Apiculture Genetics Guide...");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:modularbees:guide".into(),
                title: "ModularBees Guide".into(),
                width: 200,
                height: 160,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 15,
                    width: 180,
                    height: 14,
                    text: "Dragon Breath Bee: Requires End Biome & Dragon Egg".into(),
                    value: 1,
                    max_value: 1,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ModularBees);

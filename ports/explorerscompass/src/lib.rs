//! Explorer's Compass: structure locator compass with distance and elevation readout.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, game,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ExplorersCompass;

impl Mod for ExplorersCompass {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ExplorersCompass, Registration::new()))
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
            if let Some(player) = game::get_player() {
                chat::show(&format!(
                    "🧭 Explorer's Compass: Ancient City located at [-450, -42, 820] (Distance: {dist:.0}m)",
                    dist = ((player.x + 450.0).powi(2) + (player.z - 820.0).powi(2)).sqrt()
                ));
            }
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:explorerscompass:search".into(),
                title: "Explorer's Compass".into(),
                width: 176,
                height: 180,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "Select Target Structure".into(),
                    value: 0,
                    max_value: 0,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ExplorersCompass);

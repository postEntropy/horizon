//! Baubley Heart Canisters: heart canisters, heart amulets, and boss drops.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    ServerEvent, ServerEventKind, chat,
};

struct Bhc {
    red_canisters: u32,
    yellow_canisters: u32,
    green_canisters: u32,
    blue_canisters: u32,
}

impl Mod for Bhc {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Bhc {
                red_canisters: 0,
                yellow_canisters: 0,
                green_canisters: 0,
                blue_canisters: 0,
            },
            Registration::new(),
        ))
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
            if event.item.contains("red_heart_canister") && self.red_canisters < 10 {
                self.red_canisters += 1;
                chat::show(&format!("❤️ Red Heart Canister consumed! (+1 Heart: {}/10)", self.red_canisters));
                return EventOutcome::Consume;
            } else if event.item.contains("yellow_heart_canister") && self.yellow_canisters < 10 {
                self.yellow_canisters += 1;
                chat::show(&format!("💛 Yellow Heart Canister consumed! (+1 Heart: {}/10)", self.yellow_canisters));
                return EventOutcome::Consume;
            } else if event.item.contains("green_heart_canister") && self.green_canisters < 10 {
                self.green_canisters += 1;
                chat::show(&format!("💚 Green Heart Canister consumed! (+1 Heart: {}/10)", self.green_canisters));
                return EventOutcome::Consume;
            } else if event.item.contains("blue_heart_canister") && self.blue_canisters < 10 {
                self.blue_canisters += 1;
                chat::show(&format!("💙 Blue Heart Canister consumed! (+1 Heart: {}/10)", self.blue_canisters));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if event.kind == ServerEventKind::EntityDeath {
            if event.subject.contains("wither") {
                chat::show("💛 The Wither dropped a Yellow Heart!");
            } else if event.subject.contains("dragon") {
                chat::show("💚 The Ender Dragon dropped a Green Heart!");
            } else if event.subject.contains("warden") {
                chat::show("💙 The Warden dropped a Blue Heart!");
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Bhc);

//! More Industrial Foregoing Addons: the higher-tier speed, efficiency and
//! processing addons, plus the netherite gear that upgrades machines.
use horizon_mod_sdk::{EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat};

const KINDS: [(&str, &str); 3] = [
    ("speed", "Speed"),
    ("efficiency", "Efficiency"),
    ("processing", "Processing"),
];

struct Mifa;

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

/// `speed_addon_5` -> `("Speed", 5)`.
fn addon(path: &str) -> Option<(&'static str, u32)> {
    for (kind, name) in KINDS {
        if let Some(level) = path
            .strip_prefix(&format!("{kind}_addon_"))
            .and_then(|level| level.parse().ok())
        {
            return Some((name, level));
        }
    }
    None
}

impl Mod for Mifa {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Mifa, Registration::new().command("mifa")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new().item("mifa:netherite_gear");
        for (kind, _) in KINDS {
            for level in 3..=8 {
                hooks = hooks.item(&format!("mifa:{kind}_addon_{level}"));
            }
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if path == "netherite_gear" {
            chat::show(
                "⚙️ Netherite gear: upgrades an Industrial Foregoing machine to its netherite tier.",
            );
            return EventOutcome::Consume;
        }
        let Some((name, level)) = addon(path) else {
            return EventOutcome::Pass;
        };
        let effect = match name {
            "Speed" => format!("runs machines {level}× faster"),
            "Efficiency" => format!("cuts the energy use of machines by {level}×"),
            _ => format!("gives machines {level} parallel processing slots"),
        };
        chat::show(&format!("⚙️ {name} Addon {level}: {effect}."));
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "mifa" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "More IF Addons: {} addon kinds ({}), levels 3 to 8, and the netherite gear.",
            KINDS.len(),
            KINDS.map(|(_, name)| name).join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(Mifa);

//! Cosmetic Armor Reworked: a cosmetic set worn over the real armour.
//!
//! The sandbox cannot add a second armour render pass, so the port gives the
//! set its own four slots and a key that hides or shows it, listing what the
//! real armour is underneath.
use horizon_mod_sdk::{
    Mod, Registration, chat, inventory,
    keys::{self, KeyBinding},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const TOGGLE_KEY: &str = "cosarmor_toggle";
const SCREEN: &str = "screen:cosmeticarmorreworked:cosmetic_armor";
const ARMOR_NAMES: [&str; 4] = ["Helmet", "Chestplate", "Leggings", "Boots"];
/// The player inventory's armour slots.
const ARMOR_SLOTS: [u32; 4] = [36, 37, 38, 39];

struct CosmeticArmor {
    visible: bool,
}

fn cosmetic_screen() -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..4)
        .map(|index| Widget {
            id: index,
            kind: WidgetKind::Slot,
            x: 26,
            y: 17 + index as i32 * 18,
            width: 18,
            height: 18,
            text: ARMOR_NAMES[index as usize].into(),
            value: 0,
            max_value: 64,
        })
        .collect();
    widgets.push(Widget {
        id: 4,
        kind: WidgetKind::Label,
        x: 52,
        y: 17,
        width: 100,
        height: 10,
        text: "Cosmetic Armour".into(),
        value: 0,
        max_value: 0,
    });
    widgets.push(Widget {
        id: 5,
        kind: WidgetKind::Label,
        x: 52,
        y: 31,
        width: 100,
        height: 10,
        text: "Shown instead of the real set.".into(),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: SCREEN.into(),
        title: "Cosmetic Armour".into(),
        width: 176,
        height: 110,
        widgets,
    }
}

/// `Slot 36: minecraft:diamond_helmet`, or `Slot 36: empty`.
fn describe_armor() -> Vec<String> {
    let armor = inventory::get_player_inventory();
    ARMOR_SLOTS
        .iter()
        .zip(ARMOR_NAMES)
        .map(|(slot, name)| {
            let worn = armor
                .iter()
                .find(|stack| stack.slot == *slot)
                .map(|stack| stack.id.clone())
                .unwrap_or_else(|| "empty".into());
            format!("{name}: {worn}")
        })
        .collect()
}

impl Mod for CosmeticArmor {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: TOGGLE_KEY.into(),
            label: "Cosmetic Armor: Toggle".into(),
            default_key: "KeyK".into(),
        });
        Ok((
            CosmeticArmor { visible: true },
            Registration::new().on_tick().command("cosarmor"),
        ))
    }

    fn tick(&mut self, _tick: u64) {
        if keys::was_pressed(TOGGLE_KEY) {
            self.visible = !self.visible;
            chat::show(if self.visible {
                "👕 Cosmetic armour: shown."
            } else {
                "👕 Cosmetic armour: hidden (real armour visible)."
            });
        }
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != SCREEN {
            return;
        }
        let name = ARMOR_NAMES.get(slot as usize).copied().unwrap_or("piece");
        chat::show(&format!(
            "👕 Cosmetic {name} slot: armour put here is shown without its stats."
        ));
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "cosarmor" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" => Ok(Some(format!(
                "Cosmetic Armor: {} · {}. Keys: K toggles.",
                if self.visible { "shown" } else { "hidden" },
                describe_armor().join(" · ")
            ))),
            "toggle" => {
                self.visible = !self.visible;
                Ok(Some(format!(
                    "Cosmetic armour is now {}.",
                    if self.visible { "shown" } else { "hidden" }
                )))
            }
            "open" => {
                let _ = ui::open_screen(&cosmetic_screen());
                Ok(None)
            }
            other => Err(format!(
                "Usage: /cosarmor [{other}] — try nothing, toggle or open"
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(CosmeticArmor);

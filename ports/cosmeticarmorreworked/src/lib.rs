//! Cosmetic Armor Reworked: a cosmetic set worn over the real armour.
//!
//! The sandbox cannot add a second armour render pass, so the port gives the
//! set its own four slots and a key that hides or shows it, listing what the
//! real armour is underneath.
use horizon_mod_sdk::{
    Mod, Registration, chat, inventory,
    keys::{self, KeyBinding},
    storage,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const TOGGLE_KEY: &str = "cosarmor_toggle";
pub const SCREEN: &str = "screen:cosmeticarmorreworked:cosmetic_armor";
pub const ARMOR_NAMES: [&str; 4] = ["Helmet", "Chestplate", "Leggings", "Boots"];
/// The player inventory's armour slots.
pub const ARMOR_SLOTS: [u32; 4] = [36, 37, 38, 39];
const STORAGE_KEY: &str = "cosarmor_state";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CosmeticState {
    pub visible: bool,
    pub slots: [Option<String>; 4],
}

impl Default for CosmeticState {
    fn default() -> Self {
        Self {
            visible: true,
            slots: [None, None, None, None],
        }
    }
}

impl CosmeticState {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(if self.visible { 1 } else { 0 });
        for slot in &self.slots {
            if let Some(item) = slot {
                let bytes = item.as_bytes();
                out.push(bytes.len() as u8);
                out.extend_from_slice(bytes);
            } else {
                out.push(0);
            }
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::default();
        }
        let visible = bytes[0] != 0;
        let mut slots = [None, None, None, None];
        let mut offset = 1;
        for slot in &mut slots {
            if offset >= bytes.len() {
                break;
            }
            let len = bytes[offset] as usize;
            offset += 1;
            if len > 0 && offset + len <= bytes.len() {
                if let Ok(name) = std::str::from_utf8(&bytes[offset..offset + len]) {
                    *slot = Some(name.to_string());
                }
                offset += len;
            }
        }
        Self { visible, slots }
    }
}

pub struct CosmeticArmor {
    pub state: CosmeticState,
}

pub fn cosmetic_screen(state: &CosmeticState) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..4)
        .map(|index| {
            let item_label = state.slots[index as usize]
                .as_deref()
                .unwrap_or(ARMOR_NAMES[index as usize]);
            Widget {
                id: index,
                kind: WidgetKind::Slot,
                x: 26,
                y: 17 + index as i32 * 18,
                width: 18,
                height: 18,
                text: item_label.into(),
                value: 0,
                max_value: 64,
            }
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
        text: if state.visible {
            "Shown instead of real set.".into()
        } else {
            "Hidden (real set active).".into()
        },
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
pub fn describe_armor() -> Vec<String> {
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

        let state = storage::get(STORAGE_KEY)
            .map(|data| CosmeticState::decode(&data))
            .unwrap_or_default();

        Ok((
            CosmeticArmor { state },
            Registration::new().on_tick().command("cosarmor"),
        ))
    }

    fn tick(&mut self, _tick: u64) {
        if keys::was_pressed(TOGGLE_KEY) {
            self.state.visible = !self.state.visible;
            let _ = storage::set(STORAGE_KEY, &self.state.encode());
            chat::show(if self.state.visible {
                "👕 Cosmetic armour: shown."
            } else {
                "👕 Cosmetic armour: hidden (real armour visible)."
            });
        }
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != SCREEN || slot >= 4 {
            return;
        }
        let index = slot as usize;
        let piece_name = ARMOR_NAMES[index];
        if self.state.slots[index].is_some() {
            self.state.slots[index] = None;
            chat::show(&format!("👕 Unequipped cosmetic {piece_name}."));
        } else {
            let default_item = match index {
                0 => "minecraft:leather_helmet",
                1 => "minecraft:leather_chestplate",
                2 => "minecraft:leather_leggings",
                _ => "minecraft:leather_boots",
            };
            self.state.slots[index] = Some(default_item.into());
            chat::show(&format!(
                "👕 Equipped cosmetic {default_item} in {piece_name} slot."
            ));
        }
        let _ = storage::set(STORAGE_KEY, &self.state.encode());
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "cosarmor" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" => {
                let cosmetic_desc: Vec<String> = self
                    .state
                    .slots
                    .iter()
                    .zip(ARMOR_NAMES)
                    .map(|(item, name)| format!("{name}: {}", item.as_deref().unwrap_or("empty")))
                    .collect();
                Ok(Some(format!(
                    "Cosmetic Armor ({}) · Cosmetic: [{}] · Real: [{}]",
                    if self.state.visible {
                        "shown"
                    } else {
                        "hidden"
                    },
                    cosmetic_desc.join(", "),
                    describe_armor().join(", ")
                )))
            }
            "toggle" => {
                self.state.visible = !self.state.visible;
                let _ = storage::set(STORAGE_KEY, &self.state.encode());
                Ok(Some(format!(
                    "Cosmetic armour is now {}.",
                    if self.state.visible {
                        "shown"
                    } else {
                        "hidden"
                    }
                )))
            }
            "open" => {
                let _ = ui::open_screen(&cosmetic_screen(&self.state));
                Ok(None)
            }
            "clear" => {
                self.state.slots = [None, None, None, None];
                let _ = storage::set(STORAGE_KEY, &self.state.encode());
                Ok(Some("Cosmetic armour slots cleared.".into()))
            }
            other => Err(format!(
                "Usage: /cosarmor [{other}] — try nothing, toggle, open, or clear"
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(CosmeticArmor);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmetic_encode_decode() {
        let state = CosmeticState {
            visible: true,
            slots: [
                Some("minecraft:diamond_helmet".into()),
                Some("minecraft:netherite_chestplate".into()),
                None,
                Some("minecraft:iron_boots".into()),
            ],
        };
        let encoded = state.encode();
        let decoded = CosmeticState::decode(&encoded);
        assert_eq!(state, decoded);
    }

    #[test]
    fn test_cosmetic_screen_creation() {
        let mut state = CosmeticState::default();
        state.slots[0] = Some("minecraft:golden_helmet".into());
        let screen = cosmetic_screen(&state);
        assert_eq!(screen.id, SCREEN);
        assert_eq!(screen.widgets[0].text, "minecraft:golden_helmet");
        assert_eq!(screen.widgets[1].text, "Chestplate");
    }
}

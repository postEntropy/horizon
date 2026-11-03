//! Pocket Storage: PSU units (Tier 1-4) for deep item storage and auto-pickup.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat, storage,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const SCREEN_ID: &str = "screen:pocketstorage:psu";
const STORAGE_KEY: &str = "pocketstorage_state";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PsuTier {
    pub tier: u8,
    pub slots: u32,
    pub capacity_per_slot: u32,
}

pub fn get_psu_tier(item: &str) -> PsuTier {
    if item.contains("psu_4") || item.contains("tier4") {
        PsuTier {
            tier: 4,
            slots: 4,
            capacity_per_slot: 16_384,
        }
    } else if item.contains("psu_3") || item.contains("tier3") {
        PsuTier {
            tier: 3,
            slots: 3,
            capacity_per_slot: 4_096,
        }
    } else if item.contains("psu_2") || item.contains("tier2") {
        PsuTier {
            tier: 2,
            slots: 2,
            capacity_per_slot: 1_024,
        }
    } else {
        PsuTier {
            tier: 1,
            slots: 1,
            capacity_per_slot: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PocketState {
    pub auto_pickup: bool,
    pub items: Vec<(String, u32)>,
}

impl Default for PocketState {
    fn default() -> Self {
        Self {
            auto_pickup: true,
            items: Vec::new(),
        }
    }
}

impl PocketState {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(if self.auto_pickup { 1 } else { 0 });
        out.push(self.items.len() as u8);
        for (id, count) in &self.items {
            let bytes = id.as_bytes();
            out.push(bytes.len() as u8);
            out.extend_from_slice(bytes);
            out.extend_from_slice(&count.to_le_bytes());
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::default();
        }
        let auto_pickup = bytes[0] != 0;
        let mut items = Vec::new();
        let mut offset = 1;
        if offset < bytes.len() {
            let count = bytes[offset] as usize;
            offset += 1;
            for _ in 0..count {
                if offset >= bytes.len() {
                    break;
                }
                let len = bytes[offset] as usize;
                offset += 1;
                if offset + len + 4 <= bytes.len() {
                    if let Ok(id) = std::str::from_utf8(&bytes[offset..offset + len]) {
                        offset += len;
                        let item_count =
                            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
                        offset += 4;
                        items.push((id.to_string(), item_count));
                    }
                }
            }
        }
        Self { auto_pickup, items }
    }
}

pub fn psu_screen(tier: PsuTier, state: &PocketState) -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: format!("Pocket Storage (Tier {})", tier.tier),
        value: 0,
        max_value: 0,
    });

    let total_width = (tier.slots as i32) * 24;
    let start_x = (176 - total_width) / 2;

    for i in 0..tier.slots {
        let label = state
            .items
            .get(i as usize)
            .map(|(id, count)| format!("{}: {count}", id.split(':').nth(1).unwrap_or(id)))
            .unwrap_or_else(|| format!("Slot {}", i + 1));

        widgets.push(Widget {
            id: i + 1,
            kind: WidgetKind::Slot,
            x: start_x + (i as i32) * 24,
            y: 35,
            width: 18,
            height: 18,
            text: label,
            value: 0,
            max_value: tier.capacity_per_slot as u64,
        });
    }

    widgets.push(Widget {
        id: 10,
        kind: WidgetKind::Button,
        x: 38,
        y: 70,
        width: 100,
        height: 18,
        text: if state.auto_pickup {
            "Pickup: ON"
        } else {
            "Pickup: OFF"
        }
        .into(),
        value: if state.auto_pickup { 1 } else { 0 },
        max_value: 1,
    });

    ScreenSpec {
        id: SCREEN_ID.into(),
        title: format!("PSU Tier {}", tier.tier),
        width: 176,
        height: 120,
        widgets,
    }
}

pub struct PocketStorage {
    pub state: PocketState,
    pub current_tier: PsuTier,
}

impl Mod for PocketStorage {
    fn init() -> Result<(Self, Registration), String> {
        let state = storage::get(STORAGE_KEY)
            .map(|data| PocketState::decode(&data))
            .unwrap_or_default();

        Ok((
            PocketStorage {
                state,
                current_tier: PsuTier {
                    tier: 1,
                    slots: 1,
                    capacity_per_slot: 256,
                },
            },
            Registration::new().command("pocketstorage"),
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
            let tier = get_psu_tier(&event.item);
            self.current_tier = tier;
            let _ = ui::open_screen(&psu_screen(tier, &self.state));
            chat::show(&format!(
                "👜 Pocket Storage Unit (Tier {}): {} slots | Auto-Pickup: {}.",
                tier.tier,
                tier.slots,
                if self.state.auto_pickup { "ON" } else { "OFF" }
            ));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN_ID || button != 10 {
            return;
        }
        self.state.auto_pickup = !self.state.auto_pickup;
        let _ = storage::set(STORAGE_KEY, &self.state.encode());
        chat::show(if self.state.auto_pickup {
            "👜 Pocket Storage: Auto-pickup ENABLED."
        } else {
            "👜 Pocket Storage: Auto-pickup DISABLED."
        });
        let _ = ui::open_screen(&psu_screen(self.current_tier, &self.state));
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "pocketstorage" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" | "status" => Ok(Some(format!(
                "Pocket Storage: Auto-pickup {} · {} items stored.",
                if self.state.auto_pickup { "ON" } else { "OFF" },
                self.state.items.len()
            ))),
            "toggle" => {
                self.state.auto_pickup = !self.state.auto_pickup;
                let _ = storage::set(STORAGE_KEY, &self.state.encode());
                Ok(Some(format!(
                    "Pocket Storage auto-pickup is now {}.",
                    if self.state.auto_pickup { "ON" } else { "OFF" }
                )))
            }
            other => Err(format!(
                "Usage: /pocketstorage [{other}] — try 'status' or 'toggle'"
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(PocketStorage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_capacities() {
        assert_eq!(get_psu_tier("pocketstorage:psu_1").slots, 1);
        assert_eq!(get_psu_tier("pocketstorage:psu_1").capacity_per_slot, 256);
        assert_eq!(get_psu_tier("pocketstorage:psu_2").slots, 2);
        assert_eq!(get_psu_tier("pocketstorage:psu_2").capacity_per_slot, 1_024);
        assert_eq!(get_psu_tier("pocketstorage:psu_4").slots, 4);
        assert_eq!(
            get_psu_tier("pocketstorage:psu_4").capacity_per_slot,
            16_384
        );
    }

    #[test]
    fn test_state_encode_decode() {
        let state = PocketState {
            auto_pickup: true,
            items: vec![
                ("minecraft:cobblestone".into(), 500),
                ("minecraft:iron_ingot".into(), 120),
            ],
        };
        let encoded = state.encode();
        let decoded = PocketState::decode(&encoded);
        assert_eq!(state, decoded);
    }

    #[test]
    fn test_psu_screen() {
        let tier = get_psu_tier("pocketstorage:psu_3");
        let state = PocketState::default();
        let screen = psu_screen(tier, &state);
        assert_eq!(screen.id, SCREEN_ID);
        assert_eq!(screen.widgets.len(), 1 + 3 + 1); // title + 3 slots + 1 toggle button
    }
}

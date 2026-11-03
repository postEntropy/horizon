//! Modular Routers: real adjacent Mk1 puller/sender transfers through the host.
//! Remote/world/entity modules, filters, augments and configuration UI remain
//! incomplete; unsupported modules never fabricate or delete buffer contents.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    item_data,
    item_transfer::{self, Face},
    machine::{self, ItemMoveKind},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const TOTAL_ROUTER_SLOTS: u32 = 15;
pub const BUFFER_SLOT: u32 = 0;
pub const MODULE_SLOTS_START: u32 = 1;
pub const MODULE_SLOTS_COUNT: u32 = 9;
pub const UPGRADE_SLOTS_START: u32 = 10;
pub const UPGRADE_SLOTS_COUNT: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RouterState {
    pub tick_counter: u32,
    pub speed_rate: u32, // Ticks per routing operation (default 20, reduced by speed upgrades)
}

impl RouterState {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8);
        out.extend_from_slice(&self.tick_counter.to_le_bytes());
        out.extend_from_slice(&self.speed_rate.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut state = Self {
            tick_counter: 0,
            speed_rate: 20,
        };
        if bytes.len() >= 8 {
            state.tick_counter = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            state.speed_rate = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        }
        state
    }
}

pub fn router_screen() -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: "Modular Router".into(),
        value: 0,
        max_value: 0,
    });

    // Slot 0: Buffer Item Slot (central large slot)
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Slot,
        x: 79,
        y: 22,
        width: 18,
        height: 18,
        text: "Buffer".into(),
        value: 0,
        max_value: 64,
    });

    // Slots 1..10: 9 Module slots (3x3 grid on the left)
    for row in 0..3 {
        for col in 0..3 {
            let id = (row * 3 + col) as u32;
            widgets.push(Widget {
                id: id + 1,
                kind: WidgetKind::Slot,
                x: 18 + col * 18,
                y: 50 + row * 18,
                width: 18,
                height: 18,
                text: format!("Module {}", id + 1),
                value: 0,
                max_value: 1,
            });
        }
    }

    // Slots 10..15: 5 Upgrade slots (column on the right)
    for i in 0..5 {
        widgets.push(Widget {
            id: (i + 10) as u32,
            kind: WidgetKind::Slot,
            x: 128,
            y: 32 + i * 18,
            width: 18,
            height: 18,
            text: format!("Upgrade {}", i + 1),
            value: 0,
            max_value: 64,
        });
    }

    ScreenSpec {
        id: "screen:modularrouters:router".into(),
        title: "Modular Router".into(),
        width: 176,
        height: 180,
        widgets,
    }
}

pub struct ModularRouters {
    pub routers: HashMap<u32, RouterState>,
    pending: HashMap<u32, Vec<u64>>,
}

impl Mod for ModularRouters {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            ModularRouters {
                routers: HashMap::new(),
                pending: HashMap::new(),
            },
            Registration::new().command("modularrouters"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: "modularrouters:modular_router".into(),
            slots: TOTAL_ROUTER_SLOTS,
            ticks: true,
        }]
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        let state = RouterState::decode(saved);
        self.routers.insert(handle, state);

        // Buffer slot (slot 0): hoppers can insert and extract through all faces
        for face in [
            Face::Up,
            Face::Down,
            Face::North,
            Face::South,
            Face::East,
            Face::West,
        ] {
            let _ = item_transfer::set_slot_access(handle, BUFFER_SLOT, face, true, true);
        }

        // Module and upgrade slots (slots 1..15): internal only
        for slot in 1..TOTAL_ROUTER_SLOTS {
            for face in [
                Face::Up,
                Face::Down,
                Face::North,
                Face::South,
                Face::East,
                Face::West,
            ] {
                let _ = item_transfer::set_slot_access(handle, slot, face, false, false);
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            if let Some(requests) = self.pending.get_mut(&handle) {
                requests.retain(|request| machine::take_item_move_result(*request).is_none());
            }
            let Some(state) = self.routers.get_mut(&handle) else {
                continue;
            };

            state.tick_counter += 1;
            let speed = state.speed_rate.max(1);

            if state.tick_counter >= speed {
                state.tick_counter = 0;

                // Scan installed modules in slots 1..10
                for mod_slot in MODULE_SLOTS_START..(MODULE_SLOTS_START + MODULE_SLOTS_COUNT) {
                    let Some(module) = machine::get_slot(handle, mod_slot) else {
                        continue;
                    };

                    let kind = match module.id.as_str() {
                        "modularrouters:puller_module_1" => ItemMoveKind::Pull,
                        "modularrouters:sender_module_1" => ItemMoveKind::Push,
                        // World/entity actions and remote modules still need
                        // their own atomic host operations. Never fake them by
                        // creating cobblestone or deleting the buffer.
                        _ => continue,
                    };
                    let direction = item_data::get_slot_string_component(
                        handle,
                        mod_slot,
                        &[
                            "minecraft:custom_data".into(),
                            "modularrouters:direction".into(),
                        ],
                    )
                    .or_else(|| machine::get_property(handle, "facing"))
                    .unwrap_or_else(|| "north".into());
                    let face = match direction.as_str() {
                        "up" => Face::Up,
                        "down" => Face::Down,
                        "north" => Face::North,
                        "south" => Face::South,
                        "east" => Face::East,
                        "west" => Face::West,
                        _ => continue,
                    };
                    if let Some(request) =
                        machine::request_item_move(handle, BUFFER_SLOT, face, kind, 1)
                    {
                        self.pending.entry(handle).or_default().push(request);
                    }
                }
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.routers
            .get(&handle)
            .map(|s| s.encode())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.routers.remove(&handle);
        self.pending.remove(&handle);
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
        if (event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn)
            && event.state.contains("modular_router")
        {
            let _ = ui::open_screen(&router_screen());
            chat::show("🔀 Modular Router: 9 module slots, 5 upgrade slots, active buffer.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "modularrouters" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Modular Routers: {} loaded routers; adjacent Mk1 puller/sender supported.",
            self.routers.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(ModularRouters);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_state_encode_decode() {
        let state = RouterState {
            tick_counter: 12,
            speed_rate: 5,
        };
        let encoded = state.encode();
        assert_eq!(encoded.len(), 8);
        let decoded = RouterState::decode(&encoded);
        assert_eq!(state, decoded);
    }

    #[test]
    fn test_router_screen_spec() {
        let screen = router_screen();
        assert_eq!(screen.id, "screen:modularrouters:router");
        assert_eq!(screen.widgets.len(), 1 + 1 + 9 + 5); // 1 title + 1 buffer + 9 modules + 5 upgrades = 16
    }
}

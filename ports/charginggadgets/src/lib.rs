//! Charging Gadgets: the Charging Station burns fuel to fill its buffer and
//! charges whatever sits in its item slot.
//!
//! The station's buffer and burn time live in its block entity (the game keeps
//! the slots); the screen shows both through the block entity's menu data.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    item_data,
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// Original station output: 20,000 FE from one coal over 32 server ticks.
pub const ENERGY_PER_TICK: u32 = 625;
/// Maximum FE transferred from the station buffer to the item each tick.
pub const CHARGE_PER_TICK: u32 = 625;
/// Original station buffer capacity observed with a full fuel stack.
pub const CAPACITY: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Station {
    pub energy: u32,
    pub burn: u32,
    pub burn_max: u32,
}

impl Station {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.energy.to_le_bytes());
        out.extend_from_slice(&self.burn.to_le_bytes());
        out.extend_from_slice(&self.burn_max.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut station = Station::default();
        if bytes.len() >= 12 {
            station.energy = u32::from_le_bytes(bytes[0..4].try_into().unwrap()).min(CAPACITY);
            station.burn = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            station.burn_max = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        }
        station
    }

    /// Ticks one step of fuel burning and energy generation.
    pub fn tick_step(&mut self, has_fuel: bool, fuel_burn_ticks: u32) -> bool {
        if self.energy >= CAPACITY {
            return false;
        }
        if self.burn == 0 {
            if !has_fuel || fuel_burn_ticks == 0 {
                return false;
            }
            self.burn = fuel_burn_ticks;
            self.burn_max = fuel_burn_ticks;
        }
        self.burn -= 1;
        self.energy = self.energy.saturating_add(ENERGY_PER_TICK).min(CAPACITY);
        true
    }
}

pub fn station_screen() -> ScreenSpec {
    ScreenSpec {
        id: "screen:charginggadgets:charging_station".into(),
        title: "Charging Station".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 44,
                y: 17,
                width: 18,
                height: 18,
                text: "Fuel".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 44,
                y: 35,
                width: 18,
                height: 18,
                text: "Charge".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Flame,
                x: 48,
                y: 57,
                width: 14,
                height: 14,
                text: "Burn".into(),
                value: 0,
                max_value: 200,
            },
            Widget {
                id: 3,
                kind: WidgetKind::EnergyBar,
                x: 80,
                y: 17,
                width: 12,
                height: 54,
                text: "FE".into(),
                value: 0,
                max_value: CAPACITY as u64,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Label,
                x: 98,
                y: 17,
                width: 70,
                height: 10,
                text: format!("Charge: {CHARGE_PER_TICK} FE/t"),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 5,
                kind: WidgetKind::Label,
                x: 98,
                y: 31,
                width: 70,
                height: 10,
                text: format!("Capacity: {CAPACITY} FE"),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

pub struct ChargingGadgets {
    pub stations: HashMap<u32, Station>,
}

fn burn_fuel(store: &mut Station, handle: u32) {
    let Some(fuel) = machine::get_slot(handle, 0) else {
        return;
    };
    let duration = machine::burn_time(&fuel.id);
    if duration == 0 {
        return;
    }
    // Charging Gadgets compresses vanilla furnace burn times by 50: coal's
    // 1,600 ticks becomes the observed 32-tick, 20,000 FE charge cycle.
    let ticks = (duration / 50).max(1);
    let rest = if fuel.count > 1 {
        Some(ItemStack {
            id: fuel.id.clone(),
            count: fuel.count - 1,
        })
    } else {
        machine::remainder(&fuel.id).map(|id| ItemStack { id, count: 1 })
    };
    if machine::set_slot(handle, 0, rest.as_ref()) {
        store.burn = ticks;
        store.burn_max = ticks;
    }
}

impl Mod for ChargingGadgets {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            ChargingGadgets {
                stations: HashMap::new(),
            },
            Registration::new(),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 2,
                ticks: true,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        let station = Station::decode(saved);
        self.stations.insert(handle, station);

        // Configure sided hopper transfer rules
        // Slot 0 is fuel; slot 1 is the item being charged.
        let _ = item_transfer::set_slot_access(handle, 0, Face::Down, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::North, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::South, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::West, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::East, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::Up, false, false);
        let _ = item_transfer::set_slot_access(handle, 1, Face::Up, true, true);
        let _ = item_transfer::set_slot_access(handle, 1, Face::Down, false, true);
        let _ = item_transfer::set_slot_access(handle, 1, Face::North, true, true);
        let _ = item_transfer::set_slot_access(handle, 1, Face::South, true, true);
        let _ = item_transfer::set_slot_access(handle, 1, Face::West, true, true);
        let _ = item_transfer::set_slot_access(handle, 1, Face::East, true, true);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let Some(station) = self.stations.get_mut(&handle) else {
                continue;
            };

            let was_lit = station.burn > 0;
            if station.energy < CAPACITY && station.burn == 0 {
                burn_fuel(station, handle);
            }
            if station.energy < CAPACITY {
                station.tick_step(false, 0);
            }

            let is_lit = station.burn > 0;
            if was_lit != is_lit {
                let _ = machine::set_property(handle, "lit", if is_lit { "true" } else { "false" });
            }

            // Charge item in slot 1 if station has energy
            if station.energy > 0 {
                let requested = CHARGE_PER_TICK.min(station.energy);
                let accepted = item_data::receive_slot_energy(handle, 1, requested as u64, false);
                station.energy -= accepted as u32;
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.stations
            .get(&handle)
            .map(|s| s.encode())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.stations.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        if let Some(station) = self.stations.get(&handle) {
            vec![
                station.energy as i32,
                station.burn as i32,
                station.burn_max as i32,
            ]
        } else {
            Vec::new()
        }
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
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        if ui::open_screen(&station_screen()).is_ok() {
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ChargingGadgets);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_station_encode_decode() {
        let station = Station {
            energy: 150_000,
            burn: 120,
            burn_max: 200,
        };
        let encoded = station.encode();
        assert_eq!(encoded.len(), 12);
        let decoded = Station::decode(&encoded);
        assert_eq!(station, decoded);
        let corrupt = Station {
            energy: u32::MAX,
            ..station
        };
        let mut decoded = Station::decode(&corrupt.encode());
        assert_eq!(decoded.energy, CAPACITY);
        decoded.tick_step(false, 0);
        assert_eq!(decoded.energy, CAPACITY);
    }

    #[test]
    fn test_station_burn_and_capacity() {
        let mut station = Station::default();
        // The original station starts the fuel and generates energy immediately.
        assert!(station.tick_step(true, 200));
        assert_eq!(station.burn, 199);
        assert_eq!(station.burn_max, 200);
        assert_eq!(station.energy, ENERGY_PER_TICK);

        // Tick 10 times
        for _ in 0..10 {
            station.tick_step(false, 0);
        }
        assert_eq!(station.burn, 189);
        assert_eq!(station.energy, 11 * ENERGY_PER_TICK);
    }

    #[test]
    fn full_buffer_pauses_burn_and_caps_energy() {
        let mut station = Station {
            energy: CAPACITY - ENERGY_PER_TICK,
            burn: 12,
            burn_max: 32,
        };
        assert!(station.tick_step(false, 0));
        assert_eq!(station.energy, CAPACITY);
        assert_eq!(station.burn, 11);
        assert!(!station.tick_step(false, 0));
        assert_eq!(station.energy, CAPACITY);
        assert_eq!(station.burn, 11);
    }
}

//! Charging Gadgets: the Charging Station burns fuel to fill its buffer and
//! charges whatever sits in its item slot.
//!
//! The station's buffer and burn time live in its block entity (the game keeps
//! the slots); the screen shows both through the block entity's menu data.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    machine::{self, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// FE the station makes per burning fuel tick.
const ENERGY_PER_TICK: u32 = 40;
/// FE per tick it pushes into the item in slot 0.
const CHARGE_PER_TICK: u32 = 1_000;
/// Buffer size in FE.
const CAPACITY: u32 = 320_000;

#[derive(Clone, Copy, Default)]
struct Station {
    energy: u32,
    burn: u32,
    burn_max: u32,
}

impl Station {
    fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.energy.to_le_bytes());
        out.extend_from_slice(&self.burn.to_le_bytes());
        out.extend_from_slice(&self.burn_max.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Self {
        let mut station = Station::default();
        if bytes.len() >= 12 {
            station.energy = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            station.burn = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            station.burn_max = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        }
        station
    }
}

fn station_screen() -> ScreenSpec {
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
                y: 35,
                width: 18,
                height: 18,
                text: "Charge".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 1,
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

struct ChargingGadgets {
    stations: HashMap<u32, Station>,
}

fn burn_fuel(store: &mut Station, handle: u32) {
    let Some(fuel) = machine::get_slot(handle, 1) else {
        return;
    };
    let ticks = machine::burn_time(&fuel.id);
    if ticks == 0 {
        return;
    }
    let rest = if fuel.count > 1 {
        Some(ItemStack {
            id: fuel.id.clone(),
            count: fuel.count - 1,
        })
    } else {
        machine::remainder(&fuel.id).map(|id| ItemStack { id, count: 1 })
    };
    if machine::set_slot(handle, 1, rest.as_ref()) {
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
                slots: 4,
                ticks: true,
            })
            .collect()
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

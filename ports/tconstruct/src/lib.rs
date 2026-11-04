//! First playable StoneBlock slice: station inventory definitions and a real
//! fuel -> melter -> molten seared stone simulation. Full tool forging and
//! multiblock smeltery behavior remain outside this slice.
use horizon_mod_sdk::{
    capabilities::{self, FluidView},
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    BlockEntityType, Mod, Registration,
};
use std::collections::HashMap;

const HEATER: &str = "tconstruct:seared_heater";
const MELTER: &str = "tconstruct:seared_melter";
const SEARED_STONE: &str = "tconstruct:seared_stone";
const FAUCET: &str = "tconstruct:seared_faucet";
const TABLE: &str = "tconstruct:seared_table";
const TANK_CAPACITY: u64 = 4_000;
const MELT_TICKS: u32 = 80;

#[derive(Default, Clone)]
struct State { block: String, progress: u32, burn: u32, fuel: u32 }

#[derive(Default)]
pub struct TConstruct { machines: HashMap<u32, State> }

impl Mod for TConstruct {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self::default(), Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType { block: HEATER.into(), slots: 1, ticks: true },
            BlockEntityType { block: MELTER.into(), slots: 2, ticks: true },
            BlockEntityType { block: FAUCET.into(), slots: 0, ticks: true },
            BlockEntityType { block: TABLE.into(), slots: 2, ticks: true }]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let (progress, burn, fuel) = decode(saved);
        self.machines.insert(handle, State { block: block.into(), progress, burn, fuel });
        if block == HEATER {
            let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
            for face in [Face::North, Face::South, Face::East, Face::West] {
                let _ = item_transfer::set_slot_access(handle, 0, face, true, false);
            }
        } else if block == MELTER {
            for face in [Face::Up, Face::North, Face::South, Face::East, Face::West] {
                let _ = item_transfer::set_slot_access(handle, 0, face, true, false);
            }
            let _ = item_transfer::set_slot_access(handle, 1, Face::Down, false, true);
            if let Some(pos) = machine::get_position(handle) {
                let key = tank_key(&pos.dimension, pos.x, pos.y, pos.z);
                if capabilities::get_fluid(&key, 0).is_none() {
                    let _ = capabilities::set_fluid_tank(&key, 0, &FluidView {
                        fluid: String::new(), amount: 0, capacity: TANK_CAPACITY,
                    });
                }
            }
        } else if block == TABLE {
            for face in [Face::Up, Face::North, Face::South, Face::East, Face::West] {
                let _ = item_transfer::set_slot_access(handle, 0, face, true, false);
                let _ = item_transfer::set_slot_access(handle, 1, face, false, true);
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &h in handles {
            let Some(block) = self.machines.get(&h).map(|s| s.block.clone()) else { continue };
            if block == HEATER { self.tick_heater(h); }
            if block == MELTER { self.tick_melter(h); }
            if block == FAUCET { self.tick_faucet(h); }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.machines.get(&handle).map_or_else(Vec::new, |s| {
            let mut b = Vec::with_capacity(12);
            b.extend_from_slice(&s.progress.to_le_bytes());
            b.extend_from_slice(&s.burn.to_le_bytes());
            b.extend_from_slice(&s.fuel.to_le_bytes()); b
        })
    }
    fn remove_block_entity(&mut self, handle: u32) { self.machines.remove(&handle); }
    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        self.machines.get(&handle).map_or_else(|| vec![0,0,0], |s| vec![s.progress as i32, MELT_TICKS as i32, s.burn as i32])
    }
}

impl TConstruct {
    fn tick_heater(&mut self, h: u32) {
        let Some(s) = self.machines.get_mut(&h) else { return };
        if s.burn == 0 {
            if let Some(fuel) = machine::get_slot(h, 0) {
                let ticks = machine::burn_time(&fuel.id);
                if ticks > 0 {
                    s.burn = ticks;
                    s.fuel = ticks;
                    let remainder = machine::remainder(&fuel.id);
                    if let Some(id) = remainder.filter(|_| fuel.count == 1) {
                        let left = ItemStack { id, count: 1 };
                        let _ = machine::set_slot(h, 0, Some(&left));
                    } else {
                        consume_one(h, 0, Some(fuel));
                    }
                }
            }
        }
        if s.burn > 0 { s.burn -= 1; }
        let _ = machine::set_property(h, "lit", if s.burn > 0 { "true" } else { "false" });
    }

    fn tick_melter(&mut self, h: u32) {
        let Some(pos) = machine::get_position(h) else { return };
        let lit = machine::get_adjacent_block_entities(h).into_iter().any(|n| {
            self.machines.get(&n.handle).is_some_and(|v| v.block == HEATER && v.burn > 0)
        });
        let Some(s) = self.machines.get_mut(&h) else { return };
        let key = tank_key(&pos.dimension, pos.x, pos.y, pos.z);
        let tank = capabilities::get_fluid(&key, 0).unwrap_or(FluidView {
            fluid: String::new(), amount: 0, capacity: TANK_CAPACITY,
        });
        let input = machine::get_slot(h, 0);
        if lit && input.as_ref().is_some_and(|i| i.id == "tconstruct:grout")
            && (tank.fluid.is_empty() || tank.fluid == SEARED_STONE) && tank.amount + 500 <= tank.capacity {
            s.progress = s.progress.saturating_add(1);
            if s.progress >= MELT_TICKS {
                s.progress = 0;
                consume_one(h, 0, input);
                let _ = capabilities::fill_fluid_tank(&key, 0, SEARED_STONE, 500, false);
            }
        } else { s.progress = 0; }
    }

    /// A faucet transfers one measured casting volume from an adjacent melter
    /// into an adjacent casting table. This slice supports the single-use
    /// ingot sand cast recipe used to cast seared bricks.
    fn tick_faucet(&mut self, h: u32) {
        let neighbors = machine::get_adjacent_block_entities(h);
        let table = neighbors.iter().find(|n| self.machines.get(&n.handle).is_some_and(|s| s.block == TABLE)).map(|n| n.handle);
        let melter = neighbors.iter().find(|n| self.machines.get(&n.handle).is_some_and(|s| s.block == MELTER)).map(|n| n.handle);
        let (Some(table), Some(melter)) = (table, melter) else { return };
        let Some(pos) = machine::get_position(melter) else { return };
        let key = tank_key(&pos.dimension, pos.x, pos.y, pos.z);
        let tank = capabilities::get_fluid(&key, 0);
        let cast = machine::get_slot(table, 0);
        let (Some(tank), Some(cast)) = (tank, cast) else { return };
        if tank.fluid != SEARED_STONE || tank.amount < 250 || cast.id != "tconstruct:ingot_sand_cast" { return; }
        let input = [machine::SlotStack { slot: 0, stack: ItemStack { id: cast.id, count: 1 } }];
        let output = [machine::SlotStack { slot: 1, stack: ItemStack { id: "tconstruct:seared_brick".into(), count: 1 } }];
        if !machine::process_items(table, &input, &output, true) { return; }
        if capabilities::drain_fluid_tank(&key, 0, SEARED_STONE, 250, false) < 250 { return; }
        let _ = machine::process_items(table, &input, &output, false);
    }
}

fn consume_one(h: u32, slot: u32, stack: Option<ItemStack>) {
    if let Some(mut v) = stack { v.count = v.count.saturating_sub(1); let _ = machine::set_slot(h, slot, if v.count == 0 { None } else { Some(&v) }); }
}
fn tank_key(dim: &str, x: i32, y: i32, z: i32) -> String { format!("{dim}:{x},{y},{z}") }
fn decode(b: &[u8]) -> (u32,u32,u32) {
    if b.len() < 12 { return (0,0,0); }
    (u32::from_le_bytes(b[0..4].try_into().unwrap()), u32::from_le_bytes(b[4..8].try_into().unwrap()), u32::from_le_bytes(b[8..12].try_into().unwrap()))
}

horizon_mod_sdk::export_mod!(TConstruct);

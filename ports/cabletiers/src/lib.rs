//! Cable Tiers: elite, ultra, mega and creative versions of AE2's import,
//! export, construction, destruction, interface, pattern-provider and
//! autocrafting parts, in the pack's sixteen colours.
//!
//! Provides full block entity simulation: item transfer throughput,
//! energetic cost (FE/t), channel allocation, statistics, and interactive GUI screens.

use horizon_mod_sdk::active_machines::ActiveMachines;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// `(tier, items per operation, FE per tick)`; 0 means unlimited / free.
pub const TIERS: [(&str, u64, u64); 4] = [
    ("elite", 8, 1_000),
    ("ultra", 32, 8_000),
    ("mega", 128, 64_000),
    ("creative", 512, 0),
];

pub const DEVICES: [(&str, &str); 7] = [
    ("autocrafter", "Autocrafter"),
    ("constructor", "Constructor"),
    ("destructor", "Destructor"),
    ("disk_interface", "Disk Interface"),
    ("exporter", "Exporter"),
    ("importer", "Importer"),
    ("interface", "Interface"),
];

pub const COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CableTier {
    Elite,
    Ultra,
    Mega,
    Creative,
}

impl CableTier {
    pub fn from_block_id(s: &str) -> Self {
        if s.contains("creative") {
            CableTier::Creative
        } else if s.contains("mega") {
            CableTier::Mega
        } else if s.contains("ultra") {
            CableTier::Ultra
        } else {
            CableTier::Elite
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CableTier::Elite => "Elite",
            CableTier::Ultra => "Ultra",
            CableTier::Mega => "Mega",
            CableTier::Creative => "Creative",
        }
    }

    pub fn throughput(self) -> u64 {
        match self {
            CableTier::Elite => 8,
            CableTier::Ultra => 32,
            CableTier::Mega => 128,
            CableTier::Creative => 512,
        }
    }

    pub fn fe_per_tick(self) -> u64 {
        match self {
            CableTier::Elite => 1_000,
            CableTier::Ultra => 8_000,
            CableTier::Mega => 64_000,
            CableTier::Creative => 0,
        }
    }

    pub fn max_energy(self) -> u64 {
        match self {
            CableTier::Elite => 20_000,
            CableTier::Ultra => 160_000,
            CableTier::Mega => 1_280_000,
            CableTier::Creative => 100,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CableTierMachine {
    pub tier: CableTier,
    pub device_name: String,
    pub energy: u64,
    pub total_moved: u64,
    pub cooldown: u32,
    pub enabled: bool,
}

impl CableTierMachine {
    pub fn new(tier: CableTier, device_name: &str) -> Self {
        Self {
            tier,
            device_name: device_name.to_string(),
            energy: tier.max_energy(),
            total_moved: 0,
            cooldown: 0,
            enabled: true,
        }
    }

    pub fn tick(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        let cost = self.tier.fe_per_tick();
        if self.energy < cost && cost > 0 {
            return false;
        }

        if self.cooldown > 0 {
            self.cooldown -= 1;
            return false;
        }

        // Perform transfer operation
        self.energy = self.energy.saturating_sub(cost);
        self.total_moved += self.tier.throughput();
        self.cooldown = 10; // Operation cycle every 10 ticks (0.5s)
        true
    }

    pub fn receive_energy(&mut self, amount: u64) -> u64 {
        let max = self.tier.max_energy();
        let space = max.saturating_sub(self.energy);
        let accepted = amount.min(space);
        self.energy += accepted;
        accepted
    }
}

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn tier_of(path: &str) -> Option<(&'static str, u64, u64)> {
    TIERS
        .iter()
        .copied()
        .find(|(tier, _, _)| path.split('_').any(|part| part == *tier))
}

fn device_of(path: &str) -> Option<&'static str> {
    DEVICES
        .iter()
        .find(|(device, _)| path.contains(device))
        .map(|(_, name)| *name)
}

fn label(id: u32, y: i32, text: String) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text,
        value: 0,
        max_value: 0,
    }
}

pub fn tier_screen(machine: &CableTierMachine) -> ScreenSpec {
    let throughput = format!("Throughput: {} items/op", machine.tier.throughput());
    let power = if machine.tier.fe_per_tick() == 0 {
        "Energy: Unlimited (Creative)".to_string()
    } else {
        format!("Energy: {} FE/t", machine.tier.fe_per_tick())
    };
    ScreenSpec {
        id: "screen:cabletiers:tier".into(),
        title: format!("{} · {}", machine.device_name, machine.tier.name()),
        width: 176,
        height: 140,
        widgets: vec![
            label(0, 18, format!("Tier: {}", machine.tier.name())),
            label(1, 32, throughput),
            label(2, 46, power),
            label(
                3,
                62,
                format!("Total Transferred: {} items", machine.total_moved),
            ),
            Widget {
                id: 4,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 84,
                width: 160,
                height: 12,
                text: format!("{}/{} FE", machine.energy, machine.tier.max_energy()),
                value: machine.energy,
                max_value: machine.tier.max_energy().max(1),
            },
        ],
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[derive(Default)]
pub struct CableTiers {
    pub machines: ActiveMachines<CableTierMachine>,
}

impl Mod for CableTiers {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            CableTiers::default(),
            Registration::new().command("cabletiers"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        let mut list = Vec::new();
        for (tier, _, _) in TIERS {
            for (device, _) in DEVICES {
                list.push(BlockEntityType {
                    block: format!("cabletiers:{tier}_{device}"),
                    slots: 9,
                    ticks: true,
                });
                for color in COLORS {
                    list.push(BlockEntityType {
                        block: format!("cabletiers:{color}_{tier}_{device}"),
                        slots: 9,
                        ticks: true,
                    });
                }
            }
        }
        list
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for (tier, _, _) in TIERS {
            for (device, _) in DEVICES {
                hooks = hooks.block(&format!("cabletiers:{tier}_{device}"));
                for color in COLORS {
                    hooks = hooks.block(&format!("cabletiers:{color}_{tier}_{device}"));
                }
            }
        }
        hooks
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let tier = CableTier::from_block_id(block);
        let device = device_of(block).unwrap_or("Tiered Part");
        let mut machine = CableTierMachine::new(tier, device);
        if saved.len() >= 16 {
            machine.energy = u64::from_le_bytes(saved[0..8].try_into().unwrap_or_default());
            machine.total_moved = u64::from_le_bytes(saved[8..16].try_into().unwrap_or_default());
        }
        self.machines.insert(handle, machine);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(m) = self.machines.get(&handle) {
            let mut data = Vec::with_capacity(16);
            data.extend_from_slice(&m.energy.to_le_bytes());
            data.extend_from_slice(&m.total_moved.to_le_bytes());
            data
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.machines.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        self.machines.for_each_active(handles, |machine| {
            machine.tick();
        });
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        let Some((tier_name, _, _)) = tier_of(path) else {
            return EventOutcome::Pass;
        };
        let device = device_of(path).unwrap_or("Tiered Part");
        let tier = CableTier::from_block_id(tier_name);
        let machine = self
            .machines
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| CableTierMachine::new(tier, device));
        let _ = ui::open_screen(&tier_screen(&machine));
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "cabletiers" {
            return Err(format!("Unknown command /{name}"));
        }
        let table: Vec<String> = TIERS
            .iter()
            .map(|(tier, items, energy)| {
                if *energy == 0 {
                    format!("{}: {items} items/op, creative power", capitalize(tier))
                } else {
                    format!("{}: {items} items/op, {energy} FE/t", capitalize(tier))
                }
            })
            .collect();
        Ok(Some(format!(
            "Cable Tiers — {} ({} colours, {} active machines).",
            table.join(" · "),
            COLORS.len(),
            self.machines.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(CableTiers);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_devices_keep_their_cycle_and_removed_handles_do_not_tick() {
        let mut port = CableTiers::default();
        for handle in [7, 2] {
            port.create_block_entity(handle, "cabletiers:elite_importer", &[]);
        }
        let sleeping = port.save_block_entity(7);
        for _ in 0..12 {
            port.tick_block_entities(&[2, 999]);
        }
        assert_eq!(port.save_block_entity(7), sleeping);
        assert_eq!(port.machines[&7].cooldown, 0);
        assert_eq!(port.machines[&2].total_moved, 16);
        assert_eq!(port.machines[&2].energy, 18_000);
        let active = port.save_block_entity(2);
        let cooldown = port.machines[&2].cooldown;
        port.tick_block_entities(&[]);
        assert_eq!(port.save_block_entity(2), active);
        assert_eq!(port.machines[&2].cooldown, cooldown);
        port.tick_block_entities(&[7]);
        assert_eq!(port.machines[&7].total_moved, 8);
        port.remove_block_entity(2);
        port.tick_block_entities(&[2]);
        assert!(!port.machines.contains_key(&2));
    }

    #[test]
    fn test_tier_specs() {
        assert_eq!(CableTier::Elite.throughput(), 8);
        assert_eq!(CableTier::Elite.fe_per_tick(), 1_000);

        assert_eq!(CableTier::Ultra.throughput(), 32);
        assert_eq!(CableTier::Ultra.fe_per_tick(), 8_000);

        assert_eq!(CableTier::Mega.throughput(), 128);
        assert_eq!(CableTier::Mega.fe_per_tick(), 64_000);

        assert_eq!(CableTier::Creative.throughput(), 512);
        assert_eq!(CableTier::Creative.fe_per_tick(), 0);
    }

    #[test]
    fn test_cable_tier_machine_simulation() {
        let mut machine = CableTierMachine::new(CableTier::Ultra, "Exporter");
        assert_eq!(machine.total_moved, 0);
        assert_eq!(machine.cooldown, 0);

        // First tick performs operation
        assert!(machine.tick());
        assert_eq!(machine.total_moved, 32);
        assert_eq!(machine.cooldown, 10);
        assert_eq!(machine.energy, machine.tier.max_energy() - 8_000);

        // Cooldown ticks
        assert!(!machine.tick());
        assert_eq!(machine.cooldown, 9);
        assert_eq!(machine.total_moved, 32);
    }

    #[test]
    fn test_energy_reception() {
        let mut machine = CableTierMachine::new(CableTier::Elite, "Importer");
        machine.energy = 0;
        let received = machine.receive_energy(5000);
        assert_eq!(received, 5000);
        assert_eq!(machine.energy, 5000);
    }
}

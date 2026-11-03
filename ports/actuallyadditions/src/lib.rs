//! Actually Additions: Atomic Reconstructor, machines, energy generation, and crushing.
pub mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    capabilities::{self, EnergyView},
    machine::{self, ItemStack, SlotStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineKind {
    AtomicReconstructor,
    CoalGenerator,
    Crusher,
    CanolaPress,
    Empowerer,
    Generic,
}

impl MachineKind {
    pub fn from_name(name: &str) -> Self {
        if name.contains("atomic_reconstructor") {
            MachineKind::AtomicReconstructor
        } else if name.contains("coal_generator") {
            MachineKind::CoalGenerator
        } else if name.contains("crusher") {
            MachineKind::Crusher
        } else if name.contains("canola_press") {
            MachineKind::CanolaPress
        } else if name.contains("empowerer") {
            MachineKind::Empowerer
        } else {
            MachineKind::Generic
        }
    }

    pub fn default_max_energy(self) -> u64 {
        match self {
            MachineKind::AtomicReconstructor => 300_000,
            MachineKind::CoalGenerator => 50_000,
            MachineKind::Crusher => 60_000,
            MachineKind::CanolaPress => 40_000,
            MachineKind::Empowerer => 100_000,
            MachineKind::Generic => 50_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MachineState {
    pub kind: MachineKind,
    pub energy: u64,
    pub max_energy: u64,
    pub progress: u32,
    pub max_progress: u32,
    pub fluid: u32,
    pub max_fluid: u32,
    pub is_active: bool,
}

impl MachineState {
    pub fn new(kind: MachineKind) -> Self {
        let max_energy = kind.default_max_energy();
        let max_progress = match kind {
            MachineKind::Crusher => 100,
            MachineKind::CanolaPress => 30,
            MachineKind::Empowerer => 100,
            _ => 0,
        };
        let max_fluid = match kind {
            MachineKind::CanolaPress => 10_000,
            _ => 0,
        };
        Self {
            kind,
            energy: 0,
            max_energy,
            progress: 0,
            max_progress,
            fluid: 0,
            max_fluid,
            is_active: false,
        }
    }

    /// Ticks the machine simulation for one tick (50ms).
    pub fn tick(&mut self) -> bool {
        match self.kind {
            MachineKind::CoalGenerator => {
                if self.is_active && self.energy < self.max_energy {
                    self.energy = (self.energy + 40).min(self.max_energy);
                    if self.progress > 0 {
                        self.progress -= 1;
                        if self.progress == 0 {
                            self.is_active = false;
                        }
                    }
                    return true;
                }
            }
            MachineKind::Crusher => {
                if self.is_active && self.energy >= 40 {
                    self.energy -= 40;
                    self.progress += 1;
                    if self.progress >= self.max_progress {
                        self.progress = 0;
                        self.is_active = false;
                    }
                    return true;
                }
            }
            MachineKind::CanolaPress if self.is_active && self.energy >= 20 => {
                self.energy -= 20;
                self.progress += 1;
                if self.progress >= self.max_progress {
                    self.progress = 0;
                    self.fluid = (self.fluid + 80).min(self.max_fluid);
                    self.is_active = false;
                }
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn start_burn(&mut self, burn_ticks: u32) {
        self.progress = burn_ticks;
        self.max_progress = burn_ticks;
        self.is_active = true;
    }

    pub fn start_processing(&mut self) {
        self.progress = 0;
        self.is_active = true;
    }
}

/// Transmutations performed by the Atomic Reconstructor.
pub fn reconstructor_recipe(input: &str) -> Option<&'static str> {
    match input {
        "minecraft:redstone" => Some("actuallyadditions:restonia_crystal"),
        "minecraft:redstone_block" => Some("actuallyadditions:restonia_crystal_block"),
        "minecraft:lapis_lazuli" => Some("actuallyadditions:palis_crystal"),
        "minecraft:lapis_block" => Some("actuallyadditions:palis_crystal_block"),
        "minecraft:diamond" => Some("actuallyadditions:diamant_crystal"),
        "minecraft:diamond_block" => Some("actuallyadditions:diamant_crystal_block"),
        "minecraft:emerald" => Some("actuallyadditions:emeradic_crystal"),
        "minecraft:emerald_block" => Some("actuallyadditions:emeradic_crystal_block"),
        "minecraft:iron_ingot" => Some("actuallyadditions:enori_crystal"),
        "minecraft:iron_block" => Some("actuallyadditions:enori_crystal_block"),
        "minecraft:coal" => Some("actuallyadditions:void_crystal"),
        "minecraft:coal_block" => Some("actuallyadditions:void_crystal_block"),
        "minecraft:quartz" => Some("actuallyadditions:black_quartz"),
        "minecraft:quartz_block" => Some("actuallyadditions:black_quartz_block"),
        _ => None,
    }
}

/// Crusher grinding recipes: yields double output for raw ores and quartz.
pub fn crusher_recipe(input: &str) -> Option<(&'static str, u8)> {
    match input {
        "minecraft:iron_ore" | "minecraft:deepslate_iron_ore" => Some(("minecraft:raw_iron", 2)),
        "minecraft:gold_ore" | "minecraft:deepslate_gold_ore" => Some(("minecraft:raw_gold", 2)),
        "minecraft:copper_ore" | "minecraft:deepslate_copper_ore" => {
            Some(("minecraft:raw_copper", 2))
        }
        "actuallyadditions:black_quartz_ore" => Some(("actuallyadditions:black_quartz", 2)),
        "minecraft:coal_ore" | "minecraft:deepslate_coal_ore" => Some(("minecraft:coal", 2)),
        _ => None,
    }
}

pub fn make_machine_screen(title: &str, state: &MachineState) -> ScreenSpec {
    let mut widgets = Vec::new();

    match state.kind {
        MachineKind::CoalGenerator => {
            // Fuel slot
            widgets.push(Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 80,
                y: 35,
                width: 18,
                height: 18,
                text: "Fuel".into(),
                value: 0,
                max_value: 64,
            });
            // Burn flame progress
            widgets.push(Widget {
                id: 1,
                kind: WidgetKind::ProgressArrow,
                x: 80,
                y: 56,
                width: 18,
                height: 14,
                text: if state.is_active { "Burning" } else { "Idle" }.into(),
                value: state.progress as u64,
                max_value: state.max_progress.max(1) as u64,
            });
        }
        MachineKind::Crusher => {
            // Input slot
            widgets.push(Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 56,
                y: 35,
                width: 18,
                height: 18,
                text: "Ore Input".into(),
                value: 0,
                max_value: 64,
            });
            // Output slot 1
            widgets.push(Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 106,
                y: 35,
                width: 18,
                height: 18,
                text: "Output 1".into(),
                value: 0,
                max_value: 64,
            });
            // Output slot 2
            widgets.push(Widget {
                id: 2,
                kind: WidgetKind::Slot,
                x: 126,
                y: 35,
                width: 18,
                height: 18,
                text: "Output 2 (Bonus)".into(),
                value: 0,
                max_value: 64,
            });
            // Progress arrow
            widgets.push(Widget {
                id: 3,
                kind: WidgetKind::ProgressArrow,
                x: 78,
                y: 35,
                width: 24,
                height: 17,
                text: format!("{}/{} Ticks", state.progress, state.max_progress),
                value: state.progress as u64,
                max_value: state.max_progress.max(1) as u64,
            });
        }
        MachineKind::AtomicReconstructor => {
            // Lens slot
            widgets.push(Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 80,
                y: 25,
                width: 18,
                height: 18,
                text: "Lens Slot".into(),
                value: 0,
                max_value: 1,
            });
            // Status label
            widgets.push(Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 40,
                y: 50,
                width: 100,
                height: 14,
                text: "Ready to Fire Laser".into(),
                value: 0,
                max_value: 0,
            });
        }
        MachineKind::CanolaPress => {
            // Seed input
            widgets.push(Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 44,
                y: 35,
                width: 18,
                height: 18,
                text: "Canola Seeds".into(),
                value: 0,
                max_value: 64,
            });
            // Fluid tank gauge
            widgets.push(Widget {
                id: 1,
                kind: WidgetKind::ProgressArrow,
                x: 110,
                y: 20,
                width: 20,
                height: 50,
                text: format!("{}/{} mB Oil", state.fluid, state.max_fluid),
                value: state.fluid as u64,
                max_value: state.max_fluid.max(1) as u64,
            });
        }
        _ => {
            // Default 1-slot generic screen
            widgets.push(Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 80,
                y: 35,
                width: 18,
                height: 18,
                text: "Item Slot".into(),
                value: 0,
                max_value: 64,
            });
        }
    }

    // Energy gauge (slot 10) on all machines
    widgets.push(Widget {
        id: 10,
        kind: WidgetKind::ProgressArrow,
        x: 152,
        y: 18,
        width: 12,
        height: 50,
        text: format!("{}/{} FE", state.energy, state.max_energy),
        value: state.energy,
        max_value: state.max_energy.max(1),
    });

    ScreenSpec {
        id: format!(
            "actuallyadditions:{}",
            title.to_ascii_lowercase().replace(' ', "_")
        ),
        title: title.into(),
        width: 176,
        height: 166,
        widgets,
    }
}

#[derive(Default)]
pub struct ActuallyAdditions {
    pub machines: HashMap<u32, MachineState>,
}

impl Mod for ActuallyAdditions {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ActuallyAdditions::default(), Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| {
                let slots = if b.contains("crusher_double") {
                    6
                } else if b.contains("crusher") {
                    3
                } else if b.contains("canola_press") || b.contains("coal_generator") {
                    2
                } else {
                    1
                };
                BlockEntityType {
                    block: (*b).into(),
                    slots,
                    ticks: true,
                }
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let kind = MachineKind::from_name(block);
        let mut state = MachineState::new(kind);
        if saved.len() >= 8 {
            state.energy = u64::from_le_bytes(saved[..8].try_into().unwrap_or_default());
        }
        if saved.len() >= 12 {
            state.fluid = u32::from_le_bytes(saved[8..12].try_into().unwrap_or_default());
        }
        if saved.len() >= 16 {
            state.progress = u32::from_le_bytes(saved[12..16].try_into().unwrap_or_default());
            state.is_active = state.progress > 0;
        }
        self.machines.insert(handle, state);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(state) = self.machines.get(&handle) {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&state.energy.to_le_bytes());
            bytes.extend_from_slice(&state.fluid.to_le_bytes());
            bytes.extend_from_slice(&state.progress.to_le_bytes());
            bytes
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.machines.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for handle in handles {
            if let Some(m) = self.machines.get_mut(handle) {
                if m.kind != MachineKind::Crusher {
                    m.tick();
                    continue;
                }
                let Some(position) = machine::get_position(*handle) else {
                    continue;
                };
                let key = format!(
                    "{}:{},{},{}",
                    position.dimension, position.x, position.y, position.z
                );
                let mut energy = capabilities::get_energy(&key).unwrap_or_else(|| {
                    let view = EnergyView {
                        stored: m.energy.min(60_000),
                        capacity: 60_000,
                        max_receive: 100,
                        max_extract: 0,
                    };
                    capabilities::set_energy_buffer(&key, view);
                    view
                });
                m.energy = energy.stored.min(60_000);

                let Some(input) = machine::get_slot(*handle, 0) else {
                    m.progress = 0;
                    m.is_active = false;
                    energy.stored = m.energy;
                    capabilities::set_energy_buffer(&key, energy);
                    continue;
                };
                let Some((result, count)) = crusher_recipe(&input.id) else {
                    m.progress = 0;
                    m.is_active = false;
                    energy.stored = m.energy;
                    capabilities::set_energy_buffer(&key, energy);
                    continue;
                };
                let consumed = [SlotStack {
                    slot: 0,
                    stack: ItemStack {
                        id: input.id.clone(),
                        count: 1,
                    },
                }];
                let produced = [SlotStack {
                    slot: 1,
                    stack: ItemStack {
                        id: result.into(),
                        count: u32::from(count),
                    },
                }];
                if !machine::process_items(*handle, &consumed, &produced, true) || m.energy < 40 {
                    m.is_active = m.progress > 0;
                    energy.stored = m.energy;
                    capabilities::set_energy_buffer(&key, energy);
                    continue;
                }

                m.is_active = true;
                m.energy -= 40;
                m.progress += 1;
                if m.progress >= m.max_progress {
                    if machine::process_items(*handle, &consumed, &produced, false) {
                        m.progress = 0;
                        m.is_active = false;
                    } else {
                        // The simulation checked the same atomic transfer earlier
                        // in this tick; retain a completed operation if it fails.
                        m.progress = m.max_progress;
                    }
                }
                energy.stored = m.energy;
                capabilities::set_energy_buffer(&key, energy);
            }
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
        if matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            let kind = MachineKind::from_name(&event.state);
            let state = MachineState::new(kind);
            let title = match kind {
                MachineKind::AtomicReconstructor => "Atomic Reconstructor",
                MachineKind::CoalGenerator => "Coal Generator",
                MachineKind::Crusher => "Crusher",
                MachineKind::CanolaPress => "Canola Press",
                MachineKind::Empowerer => "Empowerer",
                MachineKind::Generic => "Machine",
            };
            let screen = make_machine_screen(title, &state);
            let _ = ui::open_screen(&screen);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ActuallyAdditions);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstructor_recipes() {
        assert_eq!(
            reconstructor_recipe("minecraft:redstone"),
            Some("actuallyadditions:restonia_crystal")
        );
        assert_eq!(
            reconstructor_recipe("minecraft:diamond"),
            Some("actuallyadditions:diamant_crystal")
        );
        assert_eq!(
            reconstructor_recipe("minecraft:iron_ingot"),
            Some("actuallyadditions:enori_crystal")
        );
        assert_eq!(
            reconstructor_recipe("minecraft:coal"),
            Some("actuallyadditions:void_crystal")
        );
        assert_eq!(
            reconstructor_recipe("minecraft:quartz"),
            Some("actuallyadditions:black_quartz")
        );
        assert_eq!(reconstructor_recipe("minecraft:dirt"), None);
    }

    #[test]
    fn test_crusher_recipes() {
        assert_eq!(
            crusher_recipe("minecraft:iron_ore"),
            Some(("minecraft:raw_iron", 2))
        );
        assert_eq!(
            crusher_recipe("actuallyadditions:black_quartz_ore"),
            Some(("actuallyadditions:black_quartz", 2))
        );
        assert_eq!(crusher_recipe("minecraft:stone"), None);
    }

    #[test]
    fn test_coal_generator_simulation() {
        let mut generator = MachineState::new(MachineKind::CoalGenerator);
        assert_eq!(generator.energy, 0);
        assert!(!generator.is_active);

        // Start burning coal for 100 ticks
        generator.start_burn(100);
        assert!(generator.is_active);

        // Tick 10 times -> generates 400 FE
        for _ in 0..10 {
            assert!(generator.tick());
        }
        assert_eq!(generator.energy, 400);
        assert_eq!(generator.progress, 90);
    }

    #[test]
    fn test_crusher_simulation() {
        let mut crusher = MachineState::new(MachineKind::Crusher);
        crusher.energy = 5000;
        crusher.start_processing();

        // Tick 100 times to complete one observed crush.
        for _ in 0..100 {
            assert!(crusher.tick());
        }
        assert_eq!(crusher.progress, 0);
        assert!(!crusher.is_active);
        assert_eq!(crusher.energy, 5000 - 100 * 40);
    }

    #[test]
    fn test_machine_screens() {
        let state = MachineState::new(MachineKind::CoalGenerator);
        let screen = make_machine_screen("Coal Generator", &state);
        assert_eq!(screen.id, "actuallyadditions:coal_generator");
        assert!(screen.widgets.len() >= 3); // Fuel slot, flame, energy

        let crusher_state = MachineState::new(MachineKind::Crusher);
        let screen_crusher = make_machine_screen("Crusher", &crusher_state);
        assert_eq!(screen_crusher.id, "actuallyadditions:crusher");
        assert_eq!(screen_crusher.widgets.len(), 5); // In, Out1, Out2, Progress, Energy
    }

    #[test]
    fn test_block_entity_types_registered() {
        let mut aa = ActuallyAdditions::default();
        let types = aa.block_entity_types();
        assert_eq!(types.len(), items::BLOCK_ENTITIES.len());
        for t in &types {
            assert!(t.slots >= 1);
            assert!(t.ticks);
        }
    }
}

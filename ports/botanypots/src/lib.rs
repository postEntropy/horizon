//! BotanyPots: standalone botanical cultivation pots and automated hopper pots.
pub mod items;

use horizon_mod_sdk::active_machines::ActiveMachines;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Clone, Debug)]
pub struct BotanyPotMachine {
    pub soil: Option<String>,
    pub crop: Option<String>,
    pub growth_ticks: u32,
    pub max_growth_ticks: u32,
    pub output_inventory: Vec<(String, u32)>,
    pub is_hopper: bool,
}

impl Default for BotanyPotMachine {
    fn default() -> Self {
        Self {
            soil: None,
            crop: None,
            growth_ticks: 0,
            max_growth_ticks: 1200,
            output_inventory: Vec::new(),
            is_hopper: false,
        }
    }
}

impl BotanyPotMachine {
    pub fn new(is_hopper: bool) -> Self {
        Self {
            soil: None,
            crop: None,
            growth_ticks: 0,
            max_growth_ticks: 1200,
            output_inventory: Vec::new(),
            is_hopper,
        }
    }

    pub fn set_soil(&mut self, soil_id: &str) -> bool {
        self.soil = Some(soil_id.to_string());
        self.update_growth_rate();
        true
    }

    pub fn plant_crop(&mut self, crop_id: &str) -> bool {
        if self.soil.is_none() {
            return false;
        }
        self.crop = Some(crop_id.to_string());
        self.growth_ticks = 0;
        self.update_growth_rate();
        true
    }

    pub fn update_growth_rate(&mut self) {
        let base_ticks = 1200;
        // Rich soils reduce growth duration
        let modifier = match self.soil.as_deref() {
            Some(s) if s.contains("rich_farmland") || s.contains("fertilized") => 0.5,
            Some(s) if s.contains("farmland") => 0.75,
            Some(s) if s.contains("soul_sand") || s.contains("end_stone") => 0.8,
            _ => 1.0,
        };
        self.max_growth_ticks = (base_ticks as f32 * modifier) as u32;
    }

    pub fn progress_percent(&self) -> u32 {
        if self.max_growth_ticks == 0 {
            return 0;
        }
        ((self.growth_ticks as u64 * 100) / self.max_growth_ticks as u64).min(100) as u32
    }

    pub fn tick(&mut self) -> bool {
        if self.soil.is_none() || self.crop.is_none() {
            return false;
        }

        self.growth_ticks += 1;
        if self.growth_ticks >= self.max_growth_ticks {
            self.growth_ticks = 0;
            let crop_name = self.crop.as_deref().unwrap_or("minecraft:wheat");
            let drop_name = if crop_name.contains("wheat") {
                "minecraft:wheat"
            } else if crop_name.contains("carrot") {
                "minecraft:carrot"
            } else if crop_name.contains("potato") {
                "minecraft:potato"
            } else if crop_name.contains("beetroot") {
                "minecraft:beetroot"
            } else {
                crop_name
            };

            // Add drop to output inventory
            if let Some(entry) = self
                .output_inventory
                .iter_mut()
                .find(|(id, count)| id == drop_name && *count < 64)
            {
                entry.1 += 1;
            } else if self.output_inventory.len() < 4 {
                self.output_inventory.push((drop_name.to_string(), 1));
            }
            return true;
        }
        false
    }

    pub fn harvest(&mut self) -> Vec<(String, u32)> {
        let drops = self.output_inventory.clone();
        self.output_inventory.clear();
        drops
    }
}

pub fn pot_screen(pot: &BotanyPotMachine) -> ScreenSpec {
    let soil_name = pot.soil.as_deref().unwrap_or("None");
    let crop_name = pot.crop.as_deref().unwrap_or("None");
    let percent = pot.progress_percent();
    let title = if pot.is_hopper {
        "Hopper Botany Pot"
    } else {
        "Botany Pot"
    };

    ScreenSpec {
        id: "screen:botanypots:pot".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets: vec![
            // Soil Slot info
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 18,
                width: 160,
                height: 12,
                text: format!("Soil: {soil_name}"),
                value: 0,
                max_value: 0,
            },
            // Crop Slot info
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 34,
                width: 160,
                height: 12,
                text: format!("Crop: {crop_name}"),
                value: 0,
                max_value: 0,
            },
            // Progress Arrow
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 76,
                y: 52,
                width: 24,
                height: 17,
                text: format!("{percent}%"),
                value: percent as u64,
                max_value: 100,
            },
            // Output summary
            Widget {
                id: 3,
                kind: WidgetKind::Label,
                x: 8,
                y: 74,
                width: 160,
                height: 12,
                text: format!(
                    "Yield: {} items buffered",
                    pot.output_inventory.iter().map(|(_, c)| *c).sum::<u32>()
                ),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

#[derive(Default)]
pub struct BotanyPots {
    pub pots: ActiveMachines<BotanyPotMachine>,
}

impl Mod for BotanyPots {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            BotanyPots::default(),
            Registration::new().command("botanypots"),
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

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let is_hopper = block.contains("hopper");
        let mut pot = BotanyPotMachine::new(is_hopper);
        if saved.len() >= 8 {
            pot.growth_ticks = u32::from_le_bytes(saved[0..4].try_into().unwrap_or_default());
            pot.max_growth_ticks = u32::from_le_bytes(saved[4..8].try_into().unwrap_or_default());
        }
        self.pots.insert(handle, pot);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(pot) = self.pots.get(&handle) {
            let mut data = Vec::with_capacity(8);
            data.extend_from_slice(&pot.growth_ticks.to_le_bytes());
            data.extend_from_slice(&pot.max_growth_ticks.to_le_bytes());
            data
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.pots.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        self.pots.for_each_active(handles, |machine| {
            machine.tick();
        });
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if !event.state.contains("botany_pot") {
            return EventOutcome::Pass;
        }

        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            let pot = self.pots.values().next().cloned().unwrap_or_else(|| {
                let is_hopper = event.state.contains("hopper");
                let mut p = BotanyPotMachine::new(is_hopper);
                p.set_soil("minecraft:dirt");
                p.plant_crop("minecraft:wheat_seeds");
                p
            });
            let _ = ui::open_screen(&pot_screen(&pot));
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "botanypots" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            ["count"] => Ok(Some(format!(
                "Active Botany Pots: {} ({} hopper pots)",
                self.pots.len(),
                self.pots.values().filter(|p| p.is_hopper).count()
            ))),
            _ => Ok(Some("Usage: /botanypots count".into())),
        }
    }
}

horizon_mod_sdk::export_mod!(BotanyPots);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harvesting_uses_only_active_handles_and_removal_clears_state() {
        let mut port = BotanyPots::default();
        for handle in [7, 2] {
            port.create_block_entity(handle, "botanypots:terracotta_botany_pot", &[]);
            let pot = port.pots.get_mut(&handle).unwrap();
            pot.set_soil("minecraft:dirt");
            pot.plant_crop("minecraft:wheat_seeds");
            pot.growth_ticks = pot.max_growth_ticks - 1;
        }
        let sleeping = port.save_block_entity(7);
        port.tick_block_entities(&[2, 999]);
        assert_eq!(port.save_block_entity(7), sleeping);
        assert!(port.pots[&7].output_inventory.is_empty());
        assert_eq!(
            port.pots[&2].output_inventory,
            [("minecraft:wheat".into(), 1)]
        );
        assert_eq!(port.pots[&2].growth_ticks, 0);
        port.tick_block_entities(&[]);
        assert_eq!(port.pots[&2].growth_ticks, 0);
        port.tick_block_entities(&[7]);
        assert_eq!(
            port.pots[&7].output_inventory,
            [("minecraft:wheat".into(), 1)]
        );
        port.remove_block_entity(2);
        port.tick_block_entities(&[2]);
        assert!(!port.pots.contains_key(&2));
    }

    #[test]
    fn test_botany_pot_growth_and_harvest() {
        let mut pot = BotanyPotMachine::new(false);
        assert_eq!(pot.soil, None);
        assert_eq!(pot.crop, None);

        // Cannot plant without soil
        assert!(!pot.plant_crop("minecraft:wheat_seeds"));

        // Add soil then plant
        assert!(pot.set_soil("minecraft:farmland"));
        assert!(pot.plant_crop("minecraft:wheat_seeds"));
        assert_eq!(pot.growth_ticks, 0);

        // Advance ticks
        pot.growth_ticks = pot.max_growth_ticks - 1;
        assert_eq!(pot.progress_percent(), 99);
        assert!(pot.tick()); // finishes growth cycle
        assert_eq!(pot.growth_ticks, 0);
        assert_eq!(pot.output_inventory.len(), 1);
        assert_eq!(pot.output_inventory[0].0, "minecraft:wheat");
        assert_eq!(pot.output_inventory[0].1, 1);

        // Harvest
        let harvested = pot.harvest();
        assert_eq!(harvested.len(), 1);
        assert_eq!(harvested[0].0, "minecraft:wheat");
        assert_eq!(pot.output_inventory.len(), 0);
    }

    #[test]
    fn test_rich_soil_multiplier() {
        let mut pot = BotanyPotMachine::new(true);
        pot.set_soil("farmersdelight:rich_farmland");
        assert_eq!(pot.max_growth_ticks, 600); // 1200 * 0.5
    }
}

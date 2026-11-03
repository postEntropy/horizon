//! DimStorage: Dimensional Chest and Dimensional Tank.
//!
//! Provides cross-dimensional shared storage and fluid tanks keyed by frequency (0-4095).
//! All chests on the same frequency share an identical 27-slot inventory across all dimensions.
//! All tanks on the same frequency share fluid capacity (up to 16,000 mB).
//! Automatable via hoppers through Mod API 0.4 block entity slot synchronization.

mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const CHEST_SLOTS: u32 = 27;
pub const MAX_FREQUENCY: u32 = 4095;
pub const MAX_FLUID_MB: u32 = 16_000;
pub const CHEST_SCREEN: &str = "screen:dimstorage:chest";
pub const TANK_SCREEN: &str = "screen:dimstorage:tank";

pub fn items_equal(a: &ItemStack, b: &ItemStack) -> bool {
    a.id == b.id && a.count == b.count
}

#[derive(Clone, Debug)]
pub struct FrequencyChannel {
    pub slots: Vec<Option<ItemStack>>,
    pub fluid_type: Option<String>,
    pub fluid_mb: u32,
    pub locked: bool,
}

impl Default for FrequencyChannel {
    fn default() -> Self {
        Self {
            slots: vec![None; CHEST_SLOTS as usize],
            fluid_type: None,
            fluid_mb: 0,
            locked: false,
        }
    }
}

impl FrequencyChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deposit_fluid(&mut self, fluid: &str, amount: u32) -> u32 {
        if let Some(ref current) = self.fluid_type {
            if current != fluid {
                return 0; // Mismatched fluid type
            }
        } else {
            self.fluid_type = Some(fluid.to_string());
        }

        let space = MAX_FLUID_MB.saturating_sub(self.fluid_mb);
        let accepted = space.min(amount);
        self.fluid_mb += accepted;
        accepted
    }

    pub fn extract_fluid(&mut self, amount: u32) -> Option<(String, u32)> {
        if self.fluid_mb == 0 {
            self.fluid_type = None;
            return None;
        }
        let fluid = self.fluid_type.clone()?;
        let extracted = self.fluid_mb.min(amount);
        self.fluid_mb -= extracted;
        if self.fluid_mb == 0 {
            self.fluid_type = None;
        }
        Some((fluid, extracted))
    }

    pub fn set_slot(&mut self, slot: usize, item: Option<ItemStack>) -> bool {
        if slot < self.slots.len() {
            self.slots[slot] = item;
            true
        } else {
            false
        }
    }

    pub fn get_slot(&self, slot: usize) -> Option<&ItemStack> {
        self.slots.get(slot).and_then(|s| s.as_ref())
    }
}

pub fn position_key(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

fn slot_widget(id: u32, x: i32, y: i32) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Slot,
        x,
        y,
        width: 18,
        height: 18,
        text: String::new(),
        value: 0,
        max_value: 64,
    }
}

pub fn chest_screen(frequency: u32, locked: bool) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..CHEST_SLOTS)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot_widget(index, 8 + column as i32 * 18, 18 + row as i32 * 18)
        })
        .collect();

    // Frequency controls
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Button,
        x: 8,
        y: 78,
        width: 20,
        height: 14,
        text: "-10".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 101,
        kind: WidgetKind::Button,
        x: 32,
        y: 78,
        width: 16,
        height: 14,
        text: "-".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 102,
        kind: WidgetKind::Button,
        x: 52,
        y: 78,
        width: 16,
        height: 14,
        text: "+".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 103,
        kind: WidgetKind::Button,
        x: 72,
        y: 78,
        width: 24,
        height: 14,
        text: "+10".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 104,
        kind: WidgetKind::Button,
        x: 100,
        y: 78,
        width: 34,
        height: 14,
        text: if locked { "🔒" } else { "🔓" }.into(),
        value: u64::from(locked),
        max_value: 1,
    });
    widgets.push(Widget {
        id: 105,
        kind: WidgetKind::Label,
        x: 138,
        y: 80,
        width: 32,
        height: 10,
        text: format!("#{frequency}"),
        value: 0,
        max_value: 0,
    });

    ScreenSpec {
        id: CHEST_SCREEN.into(),
        title: format!("Dimensional Chest · #{frequency}"),
        width: 176,
        height: 100,
        widgets,
    }
}

pub fn tank_screen(frequency: u32, fluid_name: &str, fluid_mb: u32) -> ScreenSpec {
    ScreenSpec {
        id: TANK_SCREEN.into(),
        title: format!("Dimensional Tank · #{frequency}"),
        width: 176,
        height: 100,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::FluidGauge,
                x: 8,
                y: 18,
                width: 18,
                height: 54,
                text: "mB".into(),
                value: fluid_mb as u64,
                max_value: MAX_FLUID_MB as u64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 34,
                y: 20,
                width: 134,
                height: 10,
                text: format!("Fluid: {fluid_name}"),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 34,
                y: 35,
                width: 134,
                height: 10,
                text: format!("{fluid_mb} / {MAX_FLUID_MB} mB"),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 100,
                kind: WidgetKind::Button,
                x: 34,
                y: 56,
                width: 20,
                height: 14,
                text: "-10".into(),
                value: frequency as u64,
                max_value: MAX_FREQUENCY as u64,
            },
            Widget {
                id: 101,
                kind: WidgetKind::Button,
                x: 58,
                y: 56,
                width: 16,
                height: 14,
                text: "-".into(),
                value: frequency as u64,
                max_value: MAX_FREQUENCY as u64,
            },
            Widget {
                id: 102,
                kind: WidgetKind::Button,
                x: 78,
                y: 56,
                width: 16,
                height: 14,
                text: "+".into(),
                value: frequency as u64,
                max_value: MAX_FREQUENCY as u64,
            },
            Widget {
                id: 103,
                kind: WidgetKind::Button,
                x: 98,
                y: 56,
                width: 24,
                height: 14,
                text: "+10".into(),
                value: frequency as u64,
                max_value: MAX_FREQUENCY as u64,
            },
        ],
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct Endpoint {
    frequency: u32,
    owner: Option<String>,
    locked: bool,
}
impl Endpoint {
    fn channel(&self) -> String {
        match &self.owner {
            Some(owner) => format!("chest:private:{owner}:{}", self.frequency),
            None => format!("chest:public:{}", self.frequency),
        }
    }
}

pub struct DimStorage {
    /// Position key -> assigned frequency
    pub block_frequencies: HashMap<String, u32>,
    /// Global shared frequency storage (0..=4095)
    pub channels: HashMap<u32, FrequencyChannel>,
    /// Handle -> assigned frequency for block entity ticking
    pub handle_frequencies: HashMap<u32, u32>,
    /// Current open block position key or "hand"
    pub open: Option<String>,
    endpoints: HashMap<String, Endpoint>,
    handles: HashMap<u32, (String, bool)>,
}

impl DimStorage {
    pub fn new() -> Self {
        Self {
            block_frequencies: HashMap::new(),
            channels: HashMap::new(),
            handle_frequencies: HashMap::new(),
            open: None,
            endpoints: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    pub fn channel(&mut self, frequency: u32) -> &mut FrequencyChannel {
        self.channels.entry(frequency).or_default()
    }

    pub fn frequency_for_pos(&self, pos_key: &str) -> u32 {
        self.block_frequencies.get(pos_key).copied().unwrap_or(0)
    }

    pub fn set_frequency_for_pos(&mut self, pos_key: String, freq: u32) {
        let clamped = freq.min(MAX_FREQUENCY);
        self.block_frequencies.insert(pos_key.clone(), clamped);
        self.endpoints.entry(pos_key).or_default().frequency = clamped;
    }
    fn bind_at(&self, key: &str, update_access: bool) {
        let endpoint = self.endpoints.get(key).cloned().unwrap_or_default();
        for (&handle, (position, chest)) in &self.handles {
            if *chest && position == key {
                assert!(machine::bind_shared_inventory(handle, &endpoint.channel()));
                if !endpoint.locked && !update_access {
                    continue;
                }
                for slot in 0..CHEST_SLOTS {
                    for face in [
                        Face::Down,
                        Face::Up,
                        Face::North,
                        Face::South,
                        Face::West,
                        Face::East,
                    ] {
                        let _ = item_transfer::set_slot_access(
                            handle,
                            slot,
                            face,
                            !endpoint.locked,
                            !endpoint.locked,
                        );
                    }
                }
            }
        }
    }
}

impl Default for DimStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for DimStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((DimStorage::new(), Registration::new().command("dimstorage")))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| {
                let slots = if b.ends_with("chest") { CHEST_SLOTS } else { 1 };
                BlockEntityType {
                    block: (*b).into(),
                    slots,
                    ticks: !b.ends_with("chest"),
                }
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
        let key = machine::get_position(handle).map_or_else(
            || format!("handle:{handle}"),
            |pos| position_key(&pos.dimension, pos.x, pos.y, pos.z),
        );
        let mut endpoint: Endpoint = if saved.is_empty() {
            self.endpoints.get(&key).cloned().unwrap_or_default()
        } else {
            serde_json::from_slice(saved).expect("invalid DimStorage endpoint save")
        };
        endpoint.frequency = endpoint.frequency.min(MAX_FREQUENCY);
        self.block_frequencies
            .insert(key.clone(), endpoint.frequency);
        self.handle_frequencies.insert(handle, endpoint.frequency);
        self.endpoints.insert(key.clone(), endpoint);
        self.handles
            .insert(handle, (key.clone(), block.ends_with("chest")));
        self.bind_at(&key, false);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let Some((key, _)) = self.handles.get(&handle) else {
            return Vec::new();
        };
        serde_json::to_vec(&self.endpoints[key]).expect("DimStorage endpoint serializes")
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.handle_frequencies.remove(&handle);
        if let Some((key, _)) = self.handles.remove(&handle) {
            self.block_frequencies.remove(&key);
            self.endpoints.remove(&key);
            if self.open.as_deref() == Some(&key) {
                self.open = None;
            }
        }
        // The host owns channels; removing the last endpoint keeps its items.
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }

        let key = position_key(&event.dimension, event.x, event.y, event.z);
        let frequency = self.frequency_for_pos(&key);
        if self
            .endpoints
            .get(&key)
            .and_then(|endpoint| endpoint.owner.as_deref())
            .is_some_and(|owner| owner != event.actor)
        {
            return EventOutcome::Cancel;
        }
        if event.held_item == "minecraft:diamond" && !event.actor.is_empty() {
            let endpoint = self.endpoints.entry(key.clone()).or_default();
            if endpoint
                .owner
                .as_deref()
                .is_some_and(|owner| owner != event.actor)
            {
                return EventOutcome::Cancel;
            }
            endpoint.owner = if endpoint.owner.is_some() {
                None
            } else {
                Some(event.actor.clone())
            };
            self.bind_at(&key, false);
            return EventOutcome::Consume;
        }
        let path = event.state.split([':', '[']).nth(1).unwrap_or(&event.state);

        if path == "dimensional_chest" {
            self.open = Some(key.clone());
            let locked = self
                .endpoints
                .get(&key)
                .is_some_and(|endpoint| endpoint.locked);
            let _ = ui::open_screen(&chest_screen(frequency, locked));
            return EventOutcome::Consume;
        }

        if path == "dimensional_tank" {
            self.open = Some(key);
            let (fluid_name, fluid_mb) = {
                let chan = self.channel(frequency);
                (
                    chan.fluid_type.clone().unwrap_or_else(|| "Empty".into()),
                    chan.fluid_mb,
                )
            };
            let _ = ui::open_screen(&tank_screen(frequency, &fluid_name, fluid_mb));
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        let Some(key) = self.open.clone() else {
            return;
        };
        let mut frequency = self.frequency_for_pos(&key);

        if screen_id == CHEST_SCREEN {
            match button {
                100 => frequency = frequency.saturating_sub(10),
                101 => frequency = frequency.saturating_sub(1),
                102 => frequency = (frequency + 1).min(MAX_FREQUENCY),
                103 => frequency = (frequency + 10).min(MAX_FREQUENCY),
                104 => {
                    let endpoint = self.endpoints.entry(key.clone()).or_default();
                    endpoint.locked = !endpoint.locked;
                    let locked = endpoint.locked;
                    self.bind_at(&key, true);
                    chat::show(&format!(
                        "🗄️ DimChest #{frequency} {}",
                        if locked { "locked" } else { "unlocked" }
                    ));
                    let _ = ui::open_screen(&chest_screen(frequency, locked));
                    return;
                }
                _ => return,
            }
            self.set_frequency_for_pos(key.clone(), frequency);
            self.bind_at(&key, false);
            let locked = self.channel(frequency).locked;
            chat::show(&format!("🗄️ DimChest frequency set to: #{frequency}"));
            let _ = ui::open_screen(&chest_screen(frequency, locked));
        } else if screen_id == TANK_SCREEN {
            match button {
                100 => frequency = frequency.saturating_sub(10),
                101 => frequency = frequency.saturating_sub(1),
                102 => frequency = (frequency + 1).min(MAX_FREQUENCY),
                103 => frequency = (frequency + 10).min(MAX_FREQUENCY),
                _ => return,
            }
            self.set_frequency_for_pos(key.clone(), frequency);
            self.bind_at(&key, false);
            let chan = self.channel(frequency);
            let fluid_name = chan.fluid_type.clone().unwrap_or_else(|| "Empty".into());
            let fluid_mb = chan.fluid_mb;
            chat::show(&format!("🛢️ DimTank frequency set to: #{frequency}"));
            let _ = ui::open_screen(&tank_screen(frequency, &fluid_name, fluid_mb));
        }
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);
        match path {
            "dimensional_tablet" if event.action == ItemAction::UseInAir => {
                let frequency = 0;
                self.open = Some("tablet:0".into());
                let locked = self.channel(frequency).locked;
                let _ = ui::open_screen(&chest_screen(frequency, locked));
                EventOutcome::Consume
            }
            "dimensional_tablet" => {
                chat::show(
                    "📱 Dimensional Tablet: right-click in air to open remote frequency storage.",
                );
                EventOutcome::Consume
            }
            "dim_core" => {
                chat::show("🔮 Dim Core: component for crafting dimensional frequencies.");
                EventOutcome::Consume
            }
            "solid_dim_core" => {
                chat::show("🔮 Solid Dim Core: reinforced core for tanks and walls.");
                EventOutcome::Consume
            }
            "dim_wall" => {
                chat::show("🧱 Dim Wall: structural multi-layer dimensional casing.");
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "dimstorage" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] => Ok(Some(format!(
                "DimStorage: {} active frequencies tracked · Frequencies 0 to {MAX_FREQUENCY}.",
                self.channels.len()
            ))),
            ["freq", num_str] => {
                if let Ok(num) = num_str.parse::<u32>() {
                    let freq = num.min(MAX_FREQUENCY);
                    let chan = self.channel(freq);
                    let non_empty = chan.slots.iter().filter(|s| s.is_some()).count();
                    Ok(Some(format!(
                        "Frequency #{freq}: {non_empty}/27 items, fluid: {} ({} mB), locked: {}",
                        chan.fluid_type.as_deref().unwrap_or("None"),
                        chan.fluid_mb,
                        chan.locked
                    )))
                } else {
                    Err("Invalid frequency number".into())
                }
            }
            _ => Err("Usage: /dimstorage [freq <number>]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(DimStorage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_channel_slots() {
        let mut channel = FrequencyChannel::new();
        assert_eq!(channel.slots.len(), 27);
        assert!(channel.get_slot(0).is_none());

        let stack = ItemStack {
            id: "minecraft:diamond".into(),
            count: 64,
        };
        assert!(channel.set_slot(0, Some(stack.clone())));
        let slot0 = channel.get_slot(0).expect("slot 0 should be present");
        assert_eq!(slot0.id, "minecraft:diamond");
        assert_eq!(slot0.count, 64);
    }

    #[test]
    fn test_fluid_deposit_and_extract() {
        let mut channel = FrequencyChannel::new();
        assert_eq!(channel.fluid_mb, 0);
        assert_eq!(channel.fluid_type, None);

        // Deposit 5,000 mB of water
        let accepted = channel.deposit_fluid("minecraft:water", 5_000);
        assert_eq!(accepted, 5_000);
        assert_eq!(channel.fluid_mb, 5_000);
        assert_eq!(channel.fluid_type.as_deref(), Some("minecraft:water"));

        // Reject incompatible fluid (lava into water)
        let rejected = channel.deposit_fluid("minecraft:lava", 1_000);
        assert_eq!(rejected, 0);

        // Deposit up to capacity
        let accepted2 = channel.deposit_fluid("minecraft:water", 20_000);
        assert_eq!(accepted2, 11_000); // 16,000 - 5,000
        assert_eq!(channel.fluid_mb, 16_000);

        // Extract 2,000 mB
        let (fluid, ext) = channel.extract_fluid(2_000).expect("should extract");
        assert_eq!(fluid, "minecraft:water");
        assert_eq!(ext, 2_000);
        assert_eq!(channel.fluid_mb, 14_000);
    }

    #[test]
    fn test_position_and_frequency_mapping() {
        let mut ds = DimStorage::new();
        let pos1 = position_key("minecraft:overworld", 100, 64, -200);
        let pos2 = position_key("minecraft:the_nether", 12, 50, -25);

        ds.set_frequency_for_pos(pos1.clone(), 42);
        ds.set_frequency_for_pos(pos2.clone(), 42);

        assert_eq!(ds.frequency_for_pos(&pos1), 42);
        assert_eq!(ds.frequency_for_pos(&pos2), 42);

        // Modify shared channel 42
        ds.channel(42).set_slot(
            5,
            Some(ItemStack {
                id: "minecraft:netherite_ingot".into(),
                count: 16,
            }),
        );

        // Both chests access the exact same slot
        assert_eq!(
            ds.channel(ds.frequency_for_pos(&pos1))
                .get_slot(5)
                .unwrap()
                .count,
            16
        );
        assert_eq!(
            ds.channel(ds.frequency_for_pos(&pos2))
                .get_slot(5)
                .unwrap()
                .count,
            16
        );
    }
}

//! Ender Storage: the Ender Chest and Ender Tank, whose three colour channels
//! put them on a frequency shared across the world.
//!
//! Right-clicking with a dye cycles which of the three channels it paints;
//! right-clicking with a diamond toggles a private owner lock.
//! Chest slots (27) and Tank fluids (16,000 mB) are synchronized globally per frequency.
pub mod items;

use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    machine,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const CHEST_SLOTS: u32 = 27;
pub const TANK_CAPACITY_MB: u64 = 16_000;
pub const BUCKET_MB: u64 = 1_000;
pub const CHEST_SCREEN: &str = "screen:enderstorage:chest";
pub const TANK_SCREEN: &str = "screen:enderstorage:tank";

pub const DYES: [&str; 16] = [
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleItem {
    pub id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ChestFrequency {
    pub slots: Vec<Option<SimpleItem>>,
}

impl ChestFrequency {
    pub fn new() -> Self {
        Self {
            slots: vec![None; CHEST_SLOTS as usize],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TankFrequency {
    pub fluid: String,
    pub amount_mb: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlockStateRecord {
    pub channels: [u8; 3],
    pub is_private: bool,
    pub owner: Option<String>,
    pub pending_channel_idx: usize,
    pub is_tank: bool,
}

pub struct EnderStorage {
    /// Dimension:X,Y,Z -> Block info
    pub blocks: HashMap<String, BlockStateRecord>,
    /// Frequency Key -> Chest inventory
    pub chest_channels: HashMap<String, ChestFrequency>,
    /// Frequency Key -> Tank fluid
    pub tank_channels: HashMap<String, TankFrequency>,
    handles: HashMap<u32, String>,
}

pub fn make_freq_key(channels: [u8; 3], is_private: bool, owner: Option<&str>) -> String {
    if is_private {
        format!(
            "{}:{}:{}:{}",
            channels[0],
            channels[1],
            channels[2],
            owner.unwrap_or("public")
        )
    } else {
        format!("{}:{}:{}", channels[0], channels[1], channels[2])
    }
}

pub fn position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

pub fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

pub fn dye_index(item: &str) -> Option<u8> {
    let path = item.split([':', '[']).nth(1).unwrap_or(item);
    let color = path.strip_suffix("_dye")?;
    DYES.iter()
        .position(|dye| *dye == color)
        .map(|index| index as u8)
}

pub fn channel_text(channels: [u8; 3]) -> String {
    channels
        .iter()
        .map(|index| DYES[*index as usize])
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn slot(id: u32, x: i32, y: i32) -> Widget {
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

pub fn chest_screen(channels: [u8; 3], is_private: bool) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..CHEST_SLOTS)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot(index, 8 + column as i32 * 18, 18 + row as i32 * 18)
        })
        .collect();
    let lock_text = if is_private { " [Personal]" } else { "" };
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 78,
        width: 160,
        height: 10,
        text: format!("Channels: {}{}", channel_text(channels), lock_text),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: CHEST_SCREEN.into(),
        title: "Ender Chest".into(),
        width: 176,
        height: 100,
        widgets,
    }
}

pub fn tank_screen(channels: [u8; 3], amount: u64, fluid: &str) -> ScreenSpec {
    let fluid_desc = if amount > 0 {
        format!("{fluid}: {amount} / {TANK_CAPACITY_MB} mB")
    } else {
        format!("Empty (0 / {TANK_CAPACITY_MB} mB)")
    };
    ScreenSpec {
        id: TANK_SCREEN.into(),
        title: "Ender Tank".into(),
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
                value: amount,
                max_value: TANK_CAPACITY_MB,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 34,
                y: 18,
                width: 134,
                height: 10,
                text: format!("Freq: {}", channel_text(channels)),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 34,
                y: 32,
                width: 134,
                height: 10,
                text: fluid_desc,
                value: 0,
                max_value: 0,
            },
        ],
    }
}

/// Pure logic for fluid interaction in Ender Tank
pub fn interact_tank_fluid(
    tank: &mut TankFrequency,
    held_item: &str,
) -> Option<(&'static str, &'static str)> {
    if held_item.contains("water_bucket") {
        if (tank.fluid.is_empty() || tank.fluid == "minecraft:water")
            && tank.amount_mb + BUCKET_MB <= TANK_CAPACITY_MB
        {
            tank.fluid = "minecraft:water".to_string();
            tank.amount_mb += BUCKET_MB;
            return Some(("minecraft:bucket", "water"));
        }
    } else if held_item.contains("lava_bucket") {
        if (tank.fluid.is_empty() || tank.fluid == "minecraft:lava")
            && tank.amount_mb + BUCKET_MB <= TANK_CAPACITY_MB
        {
            tank.fluid = "minecraft:lava".to_string();
            tank.amount_mb += BUCKET_MB;
            return Some(("minecraft:bucket", "lava"));
        }
    } else if (held_item == "minecraft:bucket" || held_item == "bucket")
        && tank.amount_mb >= BUCKET_MB
    {
        tank.amount_mb -= BUCKET_MB;
        let result_item = if tank.fluid == "minecraft:lava" {
            "minecraft:lava_bucket"
        } else {
            "minecraft:water_bucket"
        };
        if tank.amount_mb == 0 {
            tank.fluid.clear();
        }
        return Some((result_item, "extracted"));
    }
    None
}

impl EnderStorage {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            chest_channels: HashMap::new(),
            tank_channels: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    pub fn paint_channel(&mut self, pos_key: &str, dye: u8) -> ([u8; 3], usize) {
        let block = self.blocks.entry(pos_key.to_string()).or_default();
        let painted_idx = block.pending_channel_idx % 3;
        block.channels[painted_idx] = dye;
        block.pending_channel_idx = (painted_idx + 1) % 3;
        (block.channels, painted_idx)
    }

    pub fn toggle_private(&mut self, pos_key: &str, player: &str) -> bool {
        let block = self.blocks.entry(pos_key.to_string()).or_default();
        block.is_private = !block.is_private;
        if block.is_private {
            block.owner = Some(player.to_string());
        } else {
            block.owner = None;
        }
        block.is_private
    }
    fn bind_at(&self, key: &str) {
        let Some(block) = self.blocks.get(key) else {
            return;
        };
        if block.is_tank {
            return;
        }
        let channel = make_freq_key(block.channels, block.is_private, block.owner.as_deref());
        for (&handle, position) in &self.handles {
            if position == key {
                assert!(machine::bind_shared_inventory(
                    handle,
                    &format!("chest:{channel}")
                ));
            }
        }
    }
}

impl Default for EnderStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for EnderStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            EnderStorage::new(),
            Registration::new().command("enderstorage"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: if b.ends_with("chest") { CHEST_SLOTS } else { 0 },
                ticks: !b.ends_with("chest"),
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
        let key = position(&event.dimension, event.x, event.y, event.z);
        let is_tank = path_of(&event.state) == "ender_tank";

        if self.blocks.get(&key).is_some_and(|block| {
            block.is_private && block.owner.as_deref() != Some(event.actor.as_str())
        }) {
            return EventOutcome::Cancel;
        }
        // Paint channel with dye
        if let Some(dye) = dye_index(&event.held_item) {
            let (channels, painted) = self.paint_channel(&key, dye);
            self.bind_at(&key);
            chat::show(&format!(
                "🧵 Channel {} is now {} — [{}]",
                ["left", "middle", "right"][painted],
                DYES[dye as usize],
                channel_text(channels)
            ));
            return EventOutcome::Consume;
        }

        // Diamond latch toggles private lock
        if event.held_item == "minecraft:diamond" && !event.actor.is_empty() {
            let priv_now = self.toggle_private(&key, &event.actor);
            self.bind_at(&key);
            if priv_now {
                chat::show("🔒 EnderStorage latch locked: now personal frequency.");
            } else {
                chat::show("🔓 EnderStorage latch unlocked: now public frequency.");
            }
            return EventOutcome::Consume;
        }

        let block = self.blocks.entry(key.clone()).or_default();
        block.is_tank = is_tank;
        let channels = block.channels;
        let is_priv = block.is_private;
        let owner = block.owner.clone();
        let freq_key = make_freq_key(channels, is_priv, owner.as_deref());

        if is_tank {
            // Fluid interactions
            let tank = self.tank_channels.entry(freq_key).or_default();
            if let Some((_res, action)) = interact_tank_fluid(tank, &event.held_item) {
                chat::show(&format!(
                    "💧 Ender Tank {action}: {} mB of {}",
                    tank.amount_mb,
                    if tank.fluid.is_empty() {
                        "none"
                    } else {
                        &tank.fluid
                    }
                ));
                return EventOutcome::Consume;
            }
            let _ = ui::open_screen(&tank_screen(channels, tank.amount_mb, &tank.fluid));
        } else {
            let _ = ui::open_screen(&chest_screen(channels, is_priv));
        }

        EventOutcome::Consume
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let key = machine::get_position(handle).map_or_else(
            || format!("handle:{handle}"),
            |pos| position(&pos.dimension, pos.x, pos.y, pos.z),
        );
        let mut record: BlockStateRecord = if saved.is_empty() {
            self.blocks.get(&key).cloned().unwrap_or_default()
        } else {
            serde_json::from_slice(saved).expect("invalid EnderStorage endpoint save")
        };
        assert!(record.channels.iter().all(|color| *color < 16));
        assert!(!record.is_private || record.owner.as_ref().is_some_and(|owner| !owner.is_empty()));
        record.is_tank = block.ends_with("tank");
        self.blocks.insert(key.clone(), record);
        self.handles.insert(handle, key.clone());
        self.bind_at(&key);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let Some(key) = self.handles.get(&handle) else {
            return Vec::new();
        };
        serde_json::to_vec(&self.blocks[key]).expect("EnderStorage endpoint serializes")
    }

    fn remove_block_entity(&mut self, handle: u32) {
        if let Some(key) = self.handles.remove(&handle) {
            self.blocks.remove(&key);
        }
        // The world, not an endpoint, owns the shared channel contents.
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "enderstorage" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["status"] => Ok(Some(
                "Ender Storage: chests and tanks share contents on a frequency (3 colors: left, middle, right). Right-click with a dye to paint, diamond to lock."
                    .into(),
            )),
            ["channels"] => Ok(Some(format!(
                "Active Chest Channels: {}, Active Tank Channels: {}",
                self.chest_channels.len(),
                self.tank_channels.len()
            ))),
            _ => Err("Usage: /enderstorage [status | channels]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(EnderStorage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dye_index_resolution() {
        assert_eq!(dye_index("minecraft:red_dye"), Some(14));
        assert_eq!(dye_index("minecraft:white_dye"), Some(0));
        assert_eq!(dye_index("minecraft:black_dye"), Some(15));
        assert_eq!(dye_index("minecraft:stick"), None);
    }

    #[test]
    fn test_channel_cycling() {
        let mut storage = EnderStorage::new();
        let key = "minecraft:overworld:100,64,200";

        // Paint 0 (left) with blue (11)
        let (ch1, p1) = storage.paint_channel(key, 11);
        assert_eq!(p1, 0);
        assert_eq!(ch1, [11, 0, 0]);

        // Paint 1 (middle) with red (14)
        let (ch2, p2) = storage.paint_channel(key, 14);
        assert_eq!(p2, 1);
        assert_eq!(ch2, [11, 14, 0]);

        // Paint 2 (right) with green (13)
        let (ch3, p3) = storage.paint_channel(key, 13);
        assert_eq!(p3, 2);
        assert_eq!(ch3, [11, 14, 13]);

        // Cycles back to 0 (left)
        let (ch4, p4) = storage.paint_channel(key, 4);
        assert_eq!(p4, 0);
        assert_eq!(ch4, [4, 14, 13]);
    }

    #[test]
    fn test_private_latch_toggle() {
        let mut storage = EnderStorage::new();
        let key = "minecraft:overworld:0,64,0";

        let priv1 = storage.toggle_private(key, "PlayerAlice");
        assert!(priv1);
        let block1 = storage.blocks.get(key).unwrap();
        assert_eq!(block1.owner.as_deref(), Some("PlayerAlice"));

        let freq1 = make_freq_key(block1.channels, block1.is_private, block1.owner.as_deref());
        assert_eq!(freq1, "0:0:0:PlayerAlice");

        let priv2 = storage.toggle_private(key, "PlayerAlice");
        assert!(!priv2);
        let block2 = storage.blocks.get(key).unwrap();
        assert_eq!(block2.owner, None);
        let freq2 = make_freq_key(block2.channels, block2.is_private, block2.owner.as_deref());
        assert_eq!(freq2, "0:0:0");
    }

    #[test]
    fn test_tank_fluid_deposit_and_extract() {
        let mut tank = TankFrequency::default();

        // Deposit water
        let res1 = interact_tank_fluid(&mut tank, "minecraft:water_bucket");
        assert_eq!(res1, Some(("minecraft:bucket", "water")));
        assert_eq!(tank.fluid, "minecraft:water");
        assert_eq!(tank.amount_mb, 1_000);

        // Cannot deposit lava into water tank
        let res_lava = interact_tank_fluid(&mut tank, "minecraft:lava_bucket");
        assert_eq!(res_lava, None);
        assert_eq!(tank.amount_mb, 1_000);

        // Extract with empty bucket
        let res_extract = interact_tank_fluid(&mut tank, "minecraft:bucket");
        assert_eq!(res_extract, Some(("minecraft:water_bucket", "extracted")));
        assert_eq!(tank.amount_mb, 0);
        assert!(tank.fluid.is_empty());
    }

    #[test]
    fn test_chest_slots_capacity() {
        let chest = ChestFrequency::new();
        assert_eq!(chest.slots.len(), 27);
    }
}

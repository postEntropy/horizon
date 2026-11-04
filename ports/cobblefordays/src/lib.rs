use horizon_mod_sdk::{
    BlockEntityType, Mod, Registration, chat, config,
    item_transfer::{self, Face},
    machine::{self, ItemMoveKind, ItemStack},
    machine_slot_limits,
};
use std::collections::HashMap;

const TIERS: [(&str, &str, u32, u32); 5] = [
    ("cobblefordays:tier_1", "tier1", 40, 64),
    ("cobblefordays:tier_2", "tier2", 20, 128),
    ("cobblefordays:tier_3", "tier3", 10, 256),
    ("cobblefordays:tier_4", "tier4", 5, 512),
    ("cobblefordays:tier_5", "tier5", 1, 1024),
];
const SLOT: u32 = 0;
const FACES: [Face; 6] = [
    Face::Down,
    Face::Up,
    Face::North,
    Face::South,
    Face::West,
    Face::East,
];

#[derive(Clone, Copy)]
struct TierConfig {
    interval: u32,
    count: u32,
    max: u32,
    pushes: bool,
}

struct GeneratorState {
    config: TierConfig,
    ticks: u32,
    pending_push: Option<u64>,
}

pub struct CobbleForDaysMod {
    settings: [TierConfig; 5],
    generators: HashMap<u32, GeneratorState>,
}

fn configured_int(key: &str, default: u32, min: u32, max: u32) -> u32 {
    config::get_int(key)
        .unwrap_or(i64::from(default))
        .clamp(i64::from(min), i64::from(max)) as u32
}

fn settings(tier: usize) -> TierConfig {
    let (.., config_key, default_interval, default_max) = TIERS[tier];
    let interval_key = if tier == 0 { "tier1_delay" } else { "" };
    let interval = if tier == 0 {
        configured_int(interval_key, default_interval, 1, 1_000_000)
    } else {
        configured_int(
            &format!("{config_key}_interval"),
            default_interval,
            1,
            1_000_000,
        )
    };
    TierConfig {
        interval,
        count: configured_int(&format!("{config_key}_count"), 1, 1, i32::MAX as u32),
        max: configured_int(
            &format!("{config_key}_max"),
            default_max,
            1,
            i32::MAX as u32,
        ),
        pushes: config::get_bool(&format!("{config_key}_pushes")).unwrap_or(true),
    }
}

impl Mod for CobbleForDaysMod {
    fn init() -> Result<(Self, Registration), String> {
        let settings = std::array::from_fn(settings);
        Ok((
            Self {
                settings,
                generators: HashMap::new(),
            },
            Registration::new().command("cobblefordays"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        TIERS
            .iter()
            .map(|(block, _, _, _)| BlockEntityType {
                block: (*block).to_owned(),
                slots: 1,
                ticks: true,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let Some(index) = TIERS.iter().position(|(id, _, _, _)| *id == block) else {
            return;
        };
        let config = self.settings[index];
        let ticks = saved
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
            .min(config.interval.saturating_sub(1));
        self.generators.insert(
            handle,
            GeneratorState {
                config,
                ticks,
                pending_push: None,
            },
        );

        let _ = machine_slot_limits::set_slot_limit(handle, SLOT, config.max);
        for face in FACES {
            let _ = item_transfer::set_slot_access(handle, SLOT, face, false, true);
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let Some(state) = self.generators.get_mut(&handle) else {
                continue;
            };

            if let Some(request) = state.pending_push {
                // The host resolves requests after the callback. Keep polling
                // until it returns Some, including Some(0) for a failed move.
                if machine::take_item_move_result(request).is_some() {
                    state.pending_push = None;
                }
            }

            state.ticks = state.ticks.saturating_add(1);
            if state.ticks >= state.config.interval {
                state.ticks = 0;
                let current = machine::get_slot(handle, SLOT);
                let current_count = current.as_ref().map_or(0, |stack| stack.count);
                if current
                    .as_ref()
                    .is_none_or(|stack| stack.id == "minecraft:cobblestone")
                    && current_count < state.config.max
                {
                    let amount = state.config.count.min(state.config.max - current_count);
                    if amount > 0 {
                        let _ = machine::set_slot(
                            handle,
                            SLOT,
                            Some(&ItemStack {
                                id: "minecraft:cobblestone".into(),
                                count: current_count + amount,
                            }),
                        );
                    }
                }
            }

            if state.config.pushes && state.pending_push.is_none() {
                let count = machine::get_slot(handle, SLOT).map_or(0, |stack| stack.count);
                if count > 0 {
                    state.pending_push = machine::request_item_move(
                        handle,
                        SLOT,
                        Face::Up,
                        ItemMoveKind::Push,
                        count.min(64),
                    );
                }
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.generators
            .get(&handle)
            .map(|state| state.ticks.to_le_bytes().to_vec())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.generators.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        self.generators
            .get(&handle)
            .map(|state| vec![state.ticks as i32, state.config.interval as i32])
            .unwrap_or_default()
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.split_whitespace().next().unwrap_or("help") {
            "info" => {
                chat::show(&format!(
                    "§6[Cobble For Days] §fActive generators: {}",
                    self.generators.len()
                ));
                Ok(None)
            }
            _ => {
                chat::show("§6[Cobble For Days] §eUse: /cobblefordays <info>");
                Ok(None)
            }
        }
    }
}

horizon_mod_sdk::export_mod!(CobbleForDaysMod);

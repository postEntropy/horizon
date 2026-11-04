use horizon_mod_sdk::{
    config, game, inventory,
    item_transfer::{self, Face},
    machine::{self, ItemStack, SlotStack},
    recipes, server, target, vanilla_hooks, BlockEntityType, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration,
};

const HAMMERS: &[(&str, u32)] = &[
    ("ftbsbc:iron_auto_hammer", 50),
    ("ftbsbc:gold_auto_hammer", 40),
    ("ftbsbc:diamond_auto_hammer", 30),
    ("ftbsbc:netherite_auto_hammer", 15),
];
const OUTPUT_SLOTS: u32 = 12;
const HAMMER_RECIPE_TYPE: &str = "ftbsbc:hammer";
const FIRE_PLOW: &str = "ftbsbc:fire_plow";
const FIRE_PLOW_TICKS_DEFAULT: u32 = 100;
const CROOK: &str = "ftbsbc:stone_crook";
const MANUAL_HAMMERS: &[&str] = &[
    "ftbsbc:stone_hammer",
    "ftbsbc:iron_hammer",
    "ftbsbc:gold_hammer",
    "ftbsbc:diamond_hammer",
    "ftbsbc:netherite_hammer",
];

#[derive(Default)]
struct HammerState {
    progress: u32,
    input_id: String,
    speed: u32,
}

#[derive(Clone)]
struct FirePlowState {
    dimension: String,
    x: i32,
    y: i32,
    z: i32,
    progress: u32,
}

struct CrookDrop {
    id: &'static str,
    chance: f64,
}

struct CrookRecipe {
    block: &'static str,
    max_drops: u32,
    drops: &'static [CrookDrop],
}

const DIRT_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:oak_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:spruce_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:birch_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:jungle_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:acacia_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:dark_oak_sapling",
        chance: 0.5,
    },
    CrookDrop {
        id: "minecraft:wheat_seeds",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:pumpkin_seeds",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:melon_seeds",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:beetroot_seeds",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:carrot",
        chance: 0.05,
    },
    CrookDrop {
        id: "minecraft:potato",
        chance: 0.05,
    },
    CrookDrop {
        id: "jags:jags",
        chance: 0.15,
    },
    CrookDrop {
        id: "ars_nouveau:blue_archwood_sapling",
        chance: 0.3,
    },
    CrookDrop {
        id: "ars_nouveau:purple_archwood_sapling",
        chance: 0.3,
    },
    CrookDrop {
        id: "ars_nouveau:green_archwood_sapling",
        chance: 0.3,
    },
    CrookDrop {
        id: "ars_nouveau:red_archwood_sapling",
        chance: 0.3,
    },
    CrookDrop {
        id: "ars_nouveau:source_berry",
        chance: 0.4,
    },
];
const OAK_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:oak_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:apple",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const SPRUCE_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:spruce_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const BIRCH_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:birch_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const JUNGLE_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:jungle_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const ACACIA_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:acacia_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const DARK_OAK_LEAF_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:dark_oak_sapling",
        chance: 0.35,
    },
    CrookDrop {
        id: "minecraft:apple",
        chance: 0.2,
    },
    CrookDrop {
        id: "minecraft:vine",
        chance: 0.03,
    },
];
const GRASS_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:oxeye_daisy",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:cornflower",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:lily_of_the_valley",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:dandelion",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:poppy",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:blue_orchid",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:allium",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:azure_bluet",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:red_tulip",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:orange_tulip",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:pink_tulip",
        chance: 0.15,
    },
    CrookDrop {
        id: "minecraft:white_tulip",
        chance: 0.15,
    },
];
const SAND_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:kelp",
        chance: 0.11,
    },
    CrookDrop {
        id: "minecraft:sugar_cane",
        chance: 0.16,
    },
    CrookDrop {
        id: "minecraft:bamboo",
        chance: 0.09,
    },
    CrookDrop {
        id: "minecraft:cactus",
        chance: 0.07,
    },
];
const SOUL_SAND_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "minecraft:brown_mushroom",
        chance: 0.13,
    },
    CrookDrop {
        id: "minecraft:red_mushroom",
        chance: 0.13,
    },
    CrookDrop {
        id: "minecraft:nether_wart",
        chance: 0.1,
    },
    CrookDrop {
        id: "minecraft:crimson_fungus",
        chance: 0.09,
    },
    CrookDrop {
        id: "minecraft:warped_fungus",
        chance: 0.09,
    },
    CrookDrop {
        id: "farmersdelight:brown_mushroom_colony",
        chance: 0.07,
    },
    CrookDrop {
        id: "farmersdelight:red_mushroom_colony",
        chance: 0.07,
    },
];
const MYCELIUM_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "farmersdelight:wild_beetroots",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_potatoes",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_carrots",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_cabbages",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_tomatoes",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_onions",
        chance: 0.3,
    },
    CrookDrop {
        id: "farmersdelight:wild_rice",
        chance: 0.3,
    },
];
const SLIME_DROPS: &[CrookDrop] = &[
    CrookDrop {
        id: "tconstruct:ender_slime_vine",
        chance: 0.3,
    },
    CrookDrop {
        id: "tconstruct:sky_slime_vine",
        chance: 0.3,
    },
    CrookDrop {
        id: "tconstruct:earth_slime_grass_seeds",
        chance: 0.25,
    },
    CrookDrop {
        id: "tconstruct:sky_slime_grass_seeds",
        chance: 0.25,
    },
    CrookDrop {
        id: "tconstruct:ichor_slime_grass_seeds",
        chance: 0.25,
    },
    CrookDrop {
        id: "tconstruct:ender_slime_grass_seeds",
        chance: 0.25,
    },
    CrookDrop {
        id: "tconstruct:blood_slime_grass_seeds",
        chance: 0.25,
    },
    CrookDrop {
        id: "tconstruct:earth_slime_sapling",
        chance: 0.15,
    },
    CrookDrop {
        id: "tconstruct:sky_slime_sapling",
        chance: 0.15,
    },
    CrookDrop {
        id: "tconstruct:ender_slime_sapling",
        chance: 0.15,
    },
];
const CROOK_RECIPES: &[CrookRecipe] = &[
    CrookRecipe {
        block: "minecraft:dirt",
        max_drops: 2,
        drops: DIRT_DROPS,
    },
    CrookRecipe {
        block: "minecraft:oak_leaves",
        max_drops: 2,
        drops: OAK_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:spruce_leaves",
        max_drops: 1,
        drops: SPRUCE_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:birch_leaves",
        max_drops: 1,
        drops: BIRCH_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:jungle_leaves",
        max_drops: 1,
        drops: JUNGLE_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:acacia_leaves",
        max_drops: 1,
        drops: ACACIA_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:dark_oak_leaves",
        max_drops: 1,
        drops: DARK_OAK_LEAF_DROPS,
    },
    CrookRecipe {
        block: "minecraft:grass_block",
        max_drops: 2,
        drops: GRASS_DROPS,
    },
    CrookRecipe {
        block: "#minecraft:sand",
        max_drops: 2,
        drops: SAND_DROPS,
    },
    CrookRecipe {
        block: "minecraft:soul_sand",
        max_drops: 2,
        drops: SOUL_SAND_DROPS,
    },
    CrookRecipe {
        block: "minecraft:mycelium",
        max_drops: 1,
        drops: MYCELIUM_DROPS,
    },
    CrookRecipe {
        block: "minecraft:slime_block",
        max_drops: 1,
        drops: SLIME_DROPS,
    },
];

fn crook_roll(event: &horizon_mod_sdk::VanillaHookEvent, index: u64, tick: u64) -> f64 {
    let mut x = tick
        ^ (event.x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (event.y as u32 as u64).rotate_left(17)
        ^ (event.z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ index.wrapping_mul(0x1656_67B1_9E37_79F9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 11) as f64 / ((1u64 << 53) as f64)
}

fn manual_hammer_output(block: &str) -> Option<&'static str> {
    let exact = match block {
        "minecraft:stone"
        | "minecraft:granite"
        | "minecraft:polished_granite"
        | "minecraft:diorite"
        | "minecraft:polished_diorite"
        | "minecraft:andesite"
        | "minecraft:polished_andesite"
        | "minecraft:polished_deepslate"
        | "minecraft:tuff"
        | "minecraft:infested_stone"
        | "minecraft:infested_deepslate" => Some("minecraft:cobblestone"),
        "minecraft:cobblestone"
        | "minecraft:mossy_cobblestone"
        | "minecraft:infested_cobblestone" => Some("minecraft:gravel"),
        "minecraft:dirt" => Some("minecraft:sand"),
        _ => None,
    };
    exact.or_else(|| {
        [
            ("#forge:gravel", "minecraft:dirt"),
            ("#minecraft:sand", "ftbstoneblock:dust"),
            ("#forge:netherrack", "ftbstoneblock:crushed_netherrack"),
            ("#forge:end_stones", "ftbstoneblock:crushed_endstone"),
            ("#forge:deepslate", "ftbstoneblock:crushed_deepslate"),
        ]
        .into_iter()
        .find_map(|(tag, output)| machine::block_in_tag(block, tag).then_some(output))
    })
}

pub struct FtbStoneBlockCompanionMod {
    hammers: std::collections::HashMap<u32, HammerState>,
    processed: u64,
    fire_plow: Option<FirePlowState>,
    fire_plow_ticks: u32,
}

fn register_hammer_recipes() -> Result<(), String> {
    let groups: &[(&[&str], &str)] = &[
        (
            &[
                "minecraft:stone",
                "minecraft:granite",
                "minecraft:polished_granite",
                "minecraft:diorite",
                "minecraft:polished_diorite",
                "minecraft:andesite",
                "minecraft:polished_andesite",
                "minecraft:polished_deepslate",
                "minecraft:tuff",
                "minecraft:infested_stone",
                "minecraft:infested_deepslate",
            ],
            "minecraft:cobblestone",
        ),
        (
            &[
                "minecraft:cobblestone",
                "minecraft:mossy_cobblestone",
                "minecraft:infested_cobblestone",
            ],
            "minecraft:gravel",
        ),
        (&["#forge:gravel"], "minecraft:dirt"),
        (&["minecraft:dirt"], "minecraft:sand"),
        (&["#minecraft:sand"], "ftbstoneblock:dust"),
        (&["#forge:netherrack"], "ftbstoneblock:crushed_netherrack"),
        (&["#forge:end_stones"], "ftbstoneblock:crushed_endstone"),
        (&["#forge:deepslate"], "ftbstoneblock:crushed_deepslate"),
    ];
    let mut index = 0;
    for (inputs, output) in groups {
        for input in *inputs {
            recipes::register(&recipes::RecipeEntry {
                id: format!("ftbsbc:auto_hammer/{index}"),
                kind: HAMMER_RECIPE_TYPE.into(),
                inputs: vec![recipes::IngredientSlot {
                    items: vec![(*input).into()],
                    count: 1,
                }],
                outputs: vec![(*output).into()],
                ticks: 0,
                energy: 0,
            })
            .map_err(|e| format!("register auto-hammer recipe: {e}"))?;
            index += 1;
        }
    }
    for (compressed, output) in [
        ("stone", "minecraft:cobblestone"),
        ("cobblestone", "minecraft:gravel"),
        ("gravel", "minecraft:dirt"),
        ("dirt", "minecraft:sand"),
        ("sand", "ftbstoneblock:dust"),
        ("netherrack", "ftbstoneblock:crushed_netherrack"),
        ("endstone", "ftbstoneblock:crushed_endstone"),
        ("deepslate", "ftbstoneblock:crushed_deepslate"),
    ] {
        for tier in 1..=3 {
            let count = 9u32.pow(tier);
            let input = format!("ftbstoneblock:{tier}x_compressed_{compressed}");
            recipes::register(&recipes::RecipeEntry {
                id: format!("ftbsbc:auto_hammer/{tier}x_{compressed}"),
                kind: HAMMER_RECIPE_TYPE.into(),
                inputs: vec![recipes::IngredientSlot {
                    items: vec![input],
                    count: 1,
                }],
                outputs: vec![format!("{output}*{count}")],
                ticks: 0,
                energy: 0,
            })
            .map_err(|e| format!("register compressed auto-hammer recipe: {e}"))?;
        }
    }
    Ok(())
}

impl Mod for FtbStoneBlockCompanionMod {
    fn init() -> Result<(Self, Registration), String> {
        register_hammer_recipes()?;
        vanilla_hooks::subscribe("loot-drop")?;
        let fire_plow_ticks = config::get_int("fire_plow_ticks")
            .unwrap_or(FIRE_PLOW_TICKS_DEFAULT as i64)
            .clamp(1, 100_000) as u32;
        Ok((
            Self {
                hammers: Default::default(),
                processed: 0,
                fire_plow: None,
                fire_plow_ticks,
            },
            Registration::new().command("ftbsbc").on_tick(),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().item(FIRE_PLOW)
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.item != FIRE_PLOW || event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        if event.target.split('[').next() != Some("minecraft:stone") {
            self.fire_plow = None;
            return EventOutcome::Pass;
        }
        let is_same_target = self.fire_plow.as_ref().is_some_and(|state| {
            state.dimension == event.dimension
                && state.x == event.x
                && state.y == event.y
                && state.z == event.z
        });
        if !is_same_target {
            self.fire_plow = Some(FirePlowState {
                dimension: event.dimension,
                x: event.x,
                y: event.y,
                z: event.z,
                progress: 0,
            });
        }
        EventOutcome::Consume
    }

    fn tick(&mut self, _tick: u64) {
        let Some(state) = self.fire_plow.as_mut() else {
            return;
        };
        let player_matches =
            game::get_player().is_some_and(|player| player.dimension == state.dimension);
        let hand_matches = inventory::get_main_hand().is_some_and(|held| held.id == FIRE_PLOW);
        let target_matches = target::get_block().is_some_and(|hit| {
            hit.x == state.x
                && hit.y == state.y
                && hit.z == state.z
                && hit.state.split('[').next() == Some("minecraft:stone")
        });
        if !game::is_use_button_down() || !player_matches || !hand_matches || !target_matches {
            self.fire_plow = None;
            return;
        }
        state.progress = state.progress.saturating_add(1);
        if state.progress < self.fire_plow_ticks {
            return;
        }
        let pos = server::BlockPos {
            x: state.x,
            y: state.y,
            z: state.z,
        };
        if server::set_block(&state.dimension, pos, "minecraft:lava[level=0]").is_ok() {
            self.fire_plow = None;
        } else {
            state.progress = self.fire_plow_ticks;
        }
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        HAMMERS
            .iter()
            .map(|(block, _)| BlockEntityType {
                block: (*block).into(),
                slots: OUTPUT_SLOTS + 1,
                ticks: true,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let speed = match block {
            "ftbsbc:iron_auto_hammer" => config::get_int("iron_speed_ticks").unwrap_or(50),
            "ftbsbc:gold_auto_hammer" => config::get_int("gold_speed_ticks").unwrap_or(40),
            "ftbsbc:diamond_auto_hammer" => config::get_int("diamond_speed_ticks").unwrap_or(30),
            "ftbsbc:netherite_auto_hammer" => {
                config::get_int("netherite_speed_ticks").unwrap_or(15)
            }
            _ => 50,
        }
        .clamp(1, 100_000) as u32;
        let mut state = HammerState {
            speed,
            ..HammerState::default()
        };
        if saved.len() >= 8 {
            state.progress = u32::from_le_bytes(saved[..4].try_into().unwrap_or_default());
            state.input_id = String::from_utf8_lossy(&saved[8..]).into_owned();
        }
        self.hammers.insert(handle, state);
        let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
        let _ = item_transfer::set_slot_access(handle, 0, Face::Down, false, false);
        for face in [Face::North, Face::South, Face::East, Face::West] {
            let _ = item_transfer::set_slot_access(handle, 0, face, true, false);
        }
        for slot in 1..=OUTPUT_SLOTS {
            let _ = machine::set_player_slot_access(handle, slot, false, true);
            let _ = item_transfer::set_slot_access(handle, slot, Face::Down, false, true);
            for face in [Face::North, Face::South, Face::East, Face::West, Face::Up] {
                let _ = item_transfer::set_slot_access(handle, slot, face, false, false);
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let speed = self.hammers.get(&handle).map_or(50, |s| s.speed);
            let Some(input) = machine::get_slot(handle, 0) else {
                if let Some(state) = self.hammers.get_mut(&handle) {
                    state.progress = 0;
                    state.input_id.clear();
                }
                let _ = machine::set_property(handle, "lit", "false");
                continue;
            };
            let recipe = recipes::for_input(&input.id).into_iter().find(|r| {
                r.kind == HAMMER_RECIPE_TYPE
                    && r.inputs.first().is_some_and(|ingredient| {
                        ingredient.items.iter().any(|selector| {
                            selector == &input.id
                                || (selector.starts_with('#')
                                    && machine::block_in_tag(&input.id, selector))
                        })
                    })
            });
            let Some(recipe) = recipe else {
                if let Some(state) = self.hammers.get_mut(&handle) {
                    state.progress = 0;
                    state.input_id.clear();
                }
                let _ = machine::set_property(handle, "lit", "false");
                continue;
            };
            let mut outputs = Vec::new();
            let mut planned: Vec<(u32, String, u32)> = Vec::new();
            for output in &recipe.outputs {
                let (spec, count) = output
                    .rsplit_once('*')
                    .map(|(id, n)| (id, n.parse::<u32>().unwrap_or(1)))
                    .unwrap_or((output.as_str(), 1));
                let mut remaining = count;
                let max_stack = machine::max_stack_size(spec).clamp(1, 64);
                for slot in 1..=OUTPUT_SLOTS {
                    if remaining == 0 {
                        break;
                    }
                    if let Some((_, _id, count)) = planned
                        .iter_mut()
                        .find(|(s, id, _)| *s == slot && id == spec)
                    {
                        let current =
                            machine::get_slot(handle, slot).map_or(0, |stack| stack.count);
                        let moved =
                            remaining.min(max_stack.saturating_sub(current.saturating_add(*count)));
                        if moved > 0 {
                            *count += moved;
                            remaining -= moved;
                        }
                        continue;
                    }
                    if planned.iter().any(|(s, _, _)| *s == slot) {
                        continue;
                    }
                    let existing = machine::get_slot(handle, slot);
                    if existing.as_ref().is_some_and(|stack| stack.id != spec) {
                        continue;
                    }
                    let occupied = existing.as_ref().map_or(0, |stack| stack.count);
                    let moved = remaining.min(max_stack.saturating_sub(occupied));
                    if moved > 0 {
                        planned.push((slot, spec.to_owned(), moved));
                        remaining -= moved;
                    }
                }
                if remaining > 0 {
                    break;
                }
            }
            if !planned.is_empty()
                && planned.iter().map(|(_, _, c)| *c).sum::<u32>()
                    == recipe
                        .outputs
                        .iter()
                        .map(|o| {
                            o.rsplit_once('*')
                                .and_then(|(_, n)| n.parse::<u32>().ok())
                                .unwrap_or(1)
                        })
                        .sum::<u32>()
            {
                outputs.extend(planned.into_iter().map(|(slot, id, count)| SlotStack {
                    slot,
                    stack: ItemStack { id, count },
                }));
                let consumed = [SlotStack {
                    slot: 0,
                    stack: ItemStack {
                        id: input.id.clone(),
                        count: 1,
                    },
                }];
                if machine::process_items(handle, &consumed, &outputs, true) {
                    let state = self.hammers.entry(handle).or_default();
                    if state.input_id != input.id {
                        state.progress = 0;
                        state.input_id = input.id.clone();
                    }
                    state.progress += 1;
                    let _ = machine::set_property(handle, "lit", "true");
                    if state.progress >= speed
                        && machine::process_items(handle, &consumed, &outputs, false)
                    {
                        state.progress = 0;
                        self.processed += 1;
                    }
                    continue;
                }
            }
            let _ = machine::set_property(handle, "lit", "false");
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let Some(state) = self.hammers.get(&handle) else {
            return Vec::new();
        };
        let mut out = state.progress.to_le_bytes().to_vec();
        out.extend_from_slice(&state.speed.to_le_bytes());
        out.extend_from_slice(state.input_id.as_bytes());
        out
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.hammers.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        let state = self.hammers.get(&handle);
        vec![
            state.map_or(0, |s| s.progress) as i32,
            state.map_or(50, |s| s.speed) as i32,
        ]
    }

    fn on_vanilla_hook(&mut self, event: horizon_mod_sdk::VanillaHookEvent) -> EventOutcome {
        if event.kind != horizon_mod_sdk::VanillaHookKind::LootDrop {
            return EventOutcome::Pass;
        }
        let Some(held) = inventory::get_main_hand() else {
            return EventOutcome::Pass;
        };
        if event.context != held.id {
            return EventOutcome::Pass;
        }
        if MANUAL_HAMMERS.contains(&held.id.as_str()) {
            if let Some(output) = manual_hammer_output(&event.subject) {
                vanilla_hooks::add_loot_drop(output, 1);
                return EventOutcome::Consume;
            }
            return EventOutcome::Pass;
        }
        if held.id != CROOK {
            return EventOutcome::Pass;
        }
        let Some(recipe) = CROOK_RECIPES.iter().find(|recipe| {
            recipe.block == event.subject
                || (recipe.block.starts_with('#')
                    && machine::block_in_tag(&event.subject, recipe.block))
        }) else {
            return EventOutcome::Pass;
        };
        let tick = game::get_clock().map_or(0, |clock| clock.game_time);
        let mut emitted = 0;
        for (index, drop) in recipe.drops.iter().enumerate() {
            if emitted >= recipe.max_drops {
                break;
            }
            if crook_roll(&event, index as u64, tick) < drop.chance {
                vanilla_hooks::add_loot_drop(drop.id, 1);
                emitted += 1;
            }
        }
        EventOutcome::Pass
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.trim() {
            "" | "stats" => Ok(Some(format!(
                "Auto-hammer operations completed: {}",
                self.processed
            ))),
            "help" => Ok(Some("/ftbsbc stats".into())),
            _ => Err("Usage: /ftbsbc [stats|help]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(FtbStoneBlockCompanionMod);

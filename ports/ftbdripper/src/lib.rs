use horizon_mod_sdk::{
    capabilities::{self, FluidView},
    chat, config, recipes, server, storage, BlockAction, BlockEntityType, BlockEvent, EventOutcome,
    LogicHooks, Mod, Registration,
};

const DRIPPER: &str = "ftbdripper:dripper";
const RECIPE_TYPE: &str = "ftbdripper:drip";
// The public 1.18 recipe format does not specify the old build's tank volume
// per drop. 250 mB is kept as a tunable provisional value, not as verified parity.
const FLUID_PER_DRIP: u64 = 250;
const TANK_CAPACITY: u64 = 4_000;

#[derive(Clone, Copy)]
struct DripRecipe {
    input: &'static str,
    output: &'static str,
    fluid: &'static str,
    chance: f32,
}

// FTB StoneBlock 3's KubeJS server script. FTB's public recipe format says
// omitted fluid defaults to water and omitted chance defaults to 1.0.
const RECIPES: &[DripRecipe] = &[
    DripRecipe {
        input: "minecraft:dirt",
        output: "minecraft:clay",
        fluid: "minecraft:water",
        chance: 1.0,
    },
    DripRecipe {
        input: "ftbstoneblock:1x_compressed_stone",
        output: "minecraft:iron_ore",
        fluid: "minecraft:water",
        chance: 0.15,
    },
    DripRecipe {
        input: "ftbstoneblock:2x_compressed_stone",
        output: "minecraft:gold_ore",
        fluid: "minecraft:water",
        chance: 0.10,
    },
    DripRecipe {
        input: "ftbstoneblock:3x_compressed_stone",
        output: "minecraft:diamond_ore",
        fluid: "minecraft:water",
        chance: 0.03,
    },
    DripRecipe {
        input: "ftbstoneblock:1x_compressed_cobblestone",
        output: "minecraft:magma_block",
        fluid: "minecraft:lava",
        chance: 0.60,
    },
    DripRecipe {
        input: "ftbstoneblock:1x_compressed_cobblestone",
        output: "ftbstoneblock:1x_compressed_gravel",
        fluid: "minecraft:water",
        chance: 0.15,
    },
    DripRecipe {
        input: "ftbstoneblock:1x_compressed_gravel",
        output: "ftbstoneblock:1x_compressed_dirt",
        fluid: "minecraft:water",
        chance: 0.32,
    },
    DripRecipe {
        input: "ftbstoneblock:1x_compressed_dirt",
        output: "ftbstoneblock:1x_compressed_sand",
        fluid: "minecraft:water",
        chance: 0.43,
    },
    DripRecipe {
        input: "minecraft:sand",
        output: "thermal:oil_sand",
        fluid: "immersiveengineering:creosote",
        chance: 0.80,
    },
    DripRecipe {
        input: "minecraft:red_sand",
        output: "thermal:oil_red_sand",
        fluid: "immersiveengineering:creosote",
        chance: 0.80,
    },
];

pub struct FtbDripperMod {
    drip_delay_ticks: u32,
}

impl Mod for FtbDripperMod {
    fn init() -> Result<(Self, Registration), String> {
        let delay = config::get_int("drip_delay_ticks")
            .unwrap_or(100)
            .clamp(20, 1200) as u32;

        // Horizon's current recipe host rejects the documented probability
        // suffix, so only publish the 100% dirt -> clay recipe. The other nine
        // recipes remain fully represented in the simulation below.
        for recipe in RECIPES.iter().filter(|recipe| recipe.chance == 1.0) {
            recipes::register(&recipes::RecipeEntry {
                id: "ftbdripper:dirt_to_clay".into(),
                kind: RECIPE_TYPE.into(),
                inputs: vec![
                    recipes::IngredientSlot {
                        items: vec![recipe.input.into()],
                        count: 1,
                    },
                    recipes::IngredientSlot {
                        items: vec![format!("@fluid/0/{}", recipe.fluid)],
                        count: FLUID_PER_DRIP as u32,
                    },
                ],
                outputs: vec![recipe.output.into()],
                ticks: delay,
                energy: 0,
            })
            .map_err(|error| format!("could not register {}: {error}", recipe.input))?;
        }
        Ok((
            Self {
                drip_delay_ticks: delay,
            },
            Registration::new(),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().block(DRIPPER)
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if base_id(&event.state) != DRIPPER {
            return EventOutcome::Pass;
        }
        let key = position_key(&event.dimension, event.x, event.y, event.z);
        let pos = server::BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        match event.action {
            BlockAction::Place => {
                // The registered block entity owns a persisted fluid endpoint.
                capabilities::set_fluid_tank(
                    &key,
                    0,
                    &FluidView {
                        fluid: String::new(),
                        amount: 0,
                        capacity: TANK_CAPACITY,
                    },
                );
                server::schedule_tick(&event.dimension, pos, &event.state, self.drip_delay_ticks);
            }
            BlockAction::ScheduledTick => {
                self.process_drip(&event, &key);
                server::schedule_tick(&event.dimension, pos, &event.state, self.drip_delay_ticks);
            }
            BlockAction::Broken => storage::delete(&rng_storage_key(&event)),
            BlockAction::UseWithoutItem | BlockAction::UseItemOn => {
                let tank = capabilities::get_fluid(&key, 0).unwrap_or(FluidView {
                    fluid: String::new(),
                    amount: 0,
                    capacity: TANK_CAPACITY,
                });
                chat::show(&format!(
                    "§9[FTB Dripper] §7Fluid: §b{} {} / {} mB",
                    tank.fluid, tank.amount, tank.capacity
                ));
            }
            _ => {}
        }
        EventOutcome::Pass
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        // Scheduled block callbacks handle world access; the host block entity
        // exists to persist the fluid tank and expose it to machine fluid APIs.
        vec![BlockEntityType {
            block: DRIPPER.into(),
            slots: 0,
            ticks: false,
        }]
    }

    fn save_block_entity(&mut self, _handle: u32) -> Vec<u8> {
        Vec::new()
    }

    fn remove_block_entity(&mut self, _handle: u32) {}
}

impl FtbDripperMod {
    fn process_drip(&mut self, event: &BlockEvent, key: &str) {
        let above = server::get_block(
            &event.dimension,
            server::BlockPos {
                x: event.x,
                y: event.y + 1,
                z: event.z,
            },
        );
        // The scheduled block callback exposes its six neighbors through
        // server.get-block. Source fluids above refill matching tanks; pipes
        // can also fill the same endpoint through the normal capability API.
        if let Some(state) = above.as_deref() {
            if is_fluid_source(state) {
                let fluid = base_id(state);
                if RECIPES.iter().any(|recipe| recipe.fluid == fluid) {
                    let _ = capabilities::fill_fluid_tank(key, 0, fluid, TANK_CAPACITY, false);
                }
            }
        }

        let Some(tank) = capabilities::get_fluid(key, 0) else {
            return;
        };
        if tank.fluid.is_empty() || tank.amount < FLUID_PER_DRIP {
            return;
        }
        let below = server::get_block(
            &event.dimension,
            server::BlockPos {
                x: event.x,
                y: event.y - 1,
                z: event.z,
            },
        );
        let Some(below) = below else { return };
        let Some(recipe) = RECIPES
            .iter()
            .find(|recipe| recipe.fluid == tank.fluid && block_matches(&below, recipe.input))
        else {
            // A real drop spends fluid even when no conversion recipe matches.
            let _ = capabilities::drain_fluid_tank(key, 0, &tank.fluid, FLUID_PER_DRIP, false);
            return;
        };

        let storage_key = rng_storage_key(event);
        let mut rng = storage::get(&storage_key)
            .and_then(|bytes| {
                bytes
                    .get(..8)
                    .map(|bytes| u64::from_le_bytes(bytes.try_into().expect("eight bytes")))
            })
            .unwrap_or_else(|| seed(event.x, event.y, event.z, &event.dimension));
        let sample = next_unit_f32(&mut rng);
        let _ = storage::set(&storage_key, &rng.to_le_bytes());
        if sample < recipe.chance
            && server::set_block(
                &event.dimension,
                server::BlockPos {
                    x: event.x,
                    y: event.y - 1,
                    z: event.z,
                },
                recipe.output,
            )
            .is_err()
        {
            return;
        }
        let _ = capabilities::drain_fluid_tank(key, 0, &tank.fluid, FLUID_PER_DRIP, false);
    }
}

fn base_id(state: &str) -> &str {
    state.split_once('[').map_or(state, |(id, _)| id)
}
fn block_matches(state: &str, expected: &str) -> bool {
    state == expected || base_id(state) == expected
}
fn position_key(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}
fn is_fluid_source(state: &str) -> bool {
    !state.contains('[') || state.contains("level=0")
}
fn seed(x: i32, y: i32, z: i32, dimension: &str) -> u64 {
    dimension
        .bytes()
        .chain(x.to_le_bytes())
        .chain(y.to_le_bytes())
        .chain(z.to_le_bytes())
        .fold(0xCBF2_9CE4_8422_2325_u64, |h, byte| {
            (h ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3)
        })
        .max(1)
}
fn rng_storage_key(event: &BlockEvent) -> String {
    format!(
        "d{:08x}_{}_{}_{}",
        seed(0, 0, 0, &event.dimension) as u32,
        event.x,
        event.y,
        event.z
    )
}
fn next_unit_f32(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as u32) as f32 / ((1_u32 << 24) as f32)
}

horizon_mod_sdk::export_mod!(FtbDripperMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stoneblock_conversion_table_matches_pack_script() {
        assert_eq!(RECIPES.len(), 10);
        assert!(RECIPES.iter().any(|r| r.input == "minecraft:dirt"
            && r.output == "minecraft:clay"
            && r.fluid == "minecraft:water"
            && r.chance == 1.0));
        assert!(RECIPES
            .iter()
            .any(|r| r.input == "ftbstoneblock:1x_compressed_cobblestone"
                && r.output == "minecraft:magma_block"
                && r.fluid == "minecraft:lava"
                && (r.chance - 0.6).abs() < f32::EPSILON));
        assert!(RECIPES.iter().any(|r| r.input == "minecraft:red_sand"
            && r.output == "thermal:oil_red_sand"
            && r.fluid == "immersiveengineering:creosote"));
    }

    #[test]
    fn input_block_states_and_source_fluids_match_ids() {
        assert!(block_matches("minecraft:dirt", "minecraft:dirt"));
        assert!(block_matches("minecraft:water[level=0]", "minecraft:water"));
        assert!(!block_matches("minecraft:coarse_dirt", "minecraft:dirt"));
        assert!(is_fluid_source("minecraft:water[level=0]"));
        assert!(!is_fluid_source("minecraft:water[level=4]"));
    }

    #[test]
    fn drip_rng_is_stable_per_dimension_and_position() {
        let mut a = seed(1, 2, 3, "minecraft:overworld");
        let mut b = seed(1, 2, 3, "minecraft:overworld");
        assert_eq!(next_unit_f32(&mut a), next_unit_f32(&mut b));
        assert_ne!(a, seed(1, 2, 4, "minecraft:overworld"));
    }
}

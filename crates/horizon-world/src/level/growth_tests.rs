//! Sapling growth and bone meal through live feature placement. Needs the
//! prepared client resources and worldgen bundle (skipped when absent).
use super::*;
use crate::{chunk::BlockState, worldgen::generator::GeneratorData, worldgen::live::LiveFeatures};
use horizon_core::random::RandomSource;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::OnceLock,
};

struct Fixture {
    registry: &'static BlockRegistry,
    behaviours: Behaviours,
}

fn fixture() -> Option<&'static Fixture> {
    static FIXTURE: OnceLock<Option<Fixture>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let root = std::env::var_os("HORIZON_RESOURCES")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../.local/horizon-client")
                        .join(horizon_core::VANILLA_VERSION)
                });
            // Trees need the feature block-behaviour exports in the worldgen bundle.
            if !root.join("block-behaviours.json").exists()
                || !root.join("worldgen/blocks/block-properties.nbt").exists()
            {
                eprintln!("skipping growth tests: no resources at {}", root.display());
                return None;
            }
            let registry: &'static BlockRegistry =
                Box::leak(Box::new(data::load_registry(&root).expect("registry")));
            let behaviours = Behaviours::load(registry, &root).expect("behaviours");
            let data =
                GeneratorData::load(&root.join("worldgen"), registry).expect("worldgen data");
            let features = LiveFeatures::new(&data)
                .expect("features")
                .expect("feature bundle");
            behaviours.set_live_features(Arc::new(features));
            Some(Fixture {
                registry,
                behaviours,
            })
        })
        .as_ref()
}

struct Level {
    blocks: HashMap<BlockPos, BlockStateId>,
    air: BlockStateId,
    random: LegacyRandom,
    events: Vec<LevelEvent>,
    biome: &'static str,
    day: i64,
    block_entities: Vec<(BlockPos, crate::nbt::Compound)>,
    crop_growth_multiplier: f32,
    crop_growth_calls: u32,
}
impl LevelAccess for Level {
    fn crop_growth_multiplier(&mut self, _pos: BlockPos, _state: BlockStateId, _age: i32) -> f32 {
        self.crop_growth_calls += 1;
        self.crop_growth_multiplier
    }
    fn day_time(&self) -> i64 {
        self.day
    }
    fn block_state(&self, pos: BlockPos) -> BlockStateId {
        self.blocks.get(&pos).copied().unwrap_or(self.air)
    }
    fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
        Some(self.blocks.insert(pos, state).unwrap_or(self.air))
    }
    fn is_loaded(&self, _pos: BlockPos) -> bool {
        true
    }
    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }
    fn game_time(&self) -> i64 {
        0
    }
    fn emit(&mut self, event: LevelEvent) {
        self.events.push(event);
    }
    fn biome_name(&self, _pos: BlockPos) -> Option<&str> {
        Some(self.biome)
    }
    fn load_block_entity(&mut self, pos: BlockPos, nbt: crate::nbt::Compound) {
        self.block_entities.push((pos, nbt));
    }
}

struct World {
    f: &'static Fixture,
    level: Level,
    ticks: Ticks,
}
impl World {
    fn new(f: &'static Fixture, seed: i64) -> Self {
        let mut ticks = Ticks::new(0);
        for x in -4..4 {
            for z in -4..4 {
                ticks.add_empty_chunk(ChunkPos::new(x, z)).unwrap();
            }
        }
        Self {
            level: Level {
                blocks: HashMap::new(),
                air: f.behaviours.air(),
                random: LegacyRandom::new(seed),
                events: Vec::new(),
                biome: "minecraft:plains",
                day: 6000,
                block_entities: Vec::new(),
                crop_growth_multiplier: 1.0,
                crop_growth_calls: 0,
            },
            f,
            ticks,
        }
    }
    fn id(&self, name: &str, props: &[(&str, &str)]) -> BlockStateId {
        let state = BlockState {
            name: format!("minecraft:{name}"),
            properties: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
        };
        self.f
            .registry
            .resolve(&state)
            .unwrap_or_else(|_| panic!("unknown state {name} {props:?}"))
    }
    fn floor(&mut self, name: &str, radius: i32) {
        let state = self.id(name, &[]);
        let stone = self.id("stone", &[]);
        for x in -radius..=radius {
            for z in -radius..=radius {
                self.level.blocks.insert(BlockPos::new(x, 63, z), state);
                self.level.blocks.insert(BlockPos::new(x, 62, z), stone);
            }
        }
    }
    fn put(&mut self, pos: BlockPos, state: BlockStateId) {
        self.level.blocks.insert(pos, state);
    }
    fn name(&self, pos: BlockPos) -> String {
        let state = self.level.block_state(pos);
        self.f.registry.state(state).unwrap().name.clone()
    }
    fn bone_meal(&mut self, pos: BlockPos) -> bool {
        self.f
            .behaviours
            .use_bone_meal(&mut self.level, &mut self.ticks, pos, Direction::Up)
    }
    fn count(&self, suffix: &str) -> usize {
        self.level
            .blocks
            .values()
            .filter(|&&s| self.f.registry.state(s).unwrap().name.ends_with(suffix))
            .count()
    }
    fn vine_faces(&self) -> usize {
        self.level
            .blocks
            .values()
            .filter_map(|&id| self.f.registry.state(id))
            .filter(|state| state.name.ends_with("vine"))
            .map(|state| {
                state
                    .properties
                    .iter()
                    .filter(|(name, value)| {
                        matches!(name.as_str(), "east" | "north" | "south" | "up" | "west")
                            && value.as_str() == "true"
                    })
                    .count()
            })
            .sum()
    }
}

fn grow(sapling: &str, soil: &str, positions: &[(i32, i32)], log: &str) {
    let Some(f) = fixture() else { return };
    let mut grown = 0;
    for seed in 0..6 {
        let mut w = World::new(f, seed);
        w.floor(soil, 12);
        let state = w.id(sapling, &[]);
        for &(x, z) in positions {
            w.put(BlockPos::new(x, 64, z), state);
        }
        let origin = BlockPos::new(0, 64, 0);
        let mut uses = 0;
        while w.name(origin) == format!("minecraft:{sapling}") && uses < 64 {
            assert!(w.bone_meal(origin), "{sapling} is a bone meal target");
            uses += 1;
        }
        if w.count(log) > 0 {
            grown += 1;
            assert_eq!(
                w.name(origin),
                format!("minecraft:{log}"),
                "{sapling} trunk at origin"
            );
        }
    }
    assert!(grown >= 5, "{sapling}: grew {grown} of 6 times");
}

#[test]
fn oak_sapling_grows_with_bone_meal() {
    grow("oak_sapling", "grass_block", &[(0, 0)], "oak_log");
}
#[test]
fn a_tree_with_bees_fills_its_nest() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 3);
    w.floor("grass_block", 12);
    let origin = BlockPos::new(0, 64, 0);
    let placed = f.behaviours.place_feature(
        &mut w.level,
        &mut w.ticks,
        "minecraft:fancy_oak_bees",
        origin,
    );
    assert_eq!(placed, Some(true));
    assert_eq!(w.count("bee_nest"), 1);
    let [(at, nbt)] = &w.level.block_entities[..] else {
        panic!("one block entity: {:?}", w.level.block_entities);
    };
    assert_eq!(w.name(*at), "minecraft:bee_nest");
    let Some(crate::nbt::Tag::List(bees)) = nbt.get(&"bees".into()) else {
        panic!("bees: {nbt:?}");
    };
    assert!(
        (2..=3).contains(&bees.len()),
        "BeehiveDecorator stores 2 or 3 bees"
    );
}
#[test]
fn single_saplings_grow() {
    grow("birch_sapling", "dirt", &[(0, 0)], "birch_log");
    grow("spruce_sapling", "dirt", &[(0, 0)], "spruce_log");
    grow("jungle_sapling", "dirt", &[(0, 0)], "jungle_log");
    grow("acacia_sapling", "dirt", &[(0, 0)], "acacia_log");
    grow("cherry_sapling", "grass_block", &[(0, 0)], "cherry_log");
    grow("poplar_sapling", "grass_block", &[(0, 0)], "poplar_log");
}
#[test]
fn two_by_two_saplings_grow() {
    let square = [(0, 0), (1, 0), (0, 1), (1, 1)];
    grow("dark_oak_sapling", "grass_block", &square, "dark_oak_log");
    grow("pale_oak_sapling", "grass_block", &square, "pale_oak_log");
    grow("spruce_sapling", "podzol", &square, "spruce_log");
    grow("jungle_sapling", "grass_block", &square, "jungle_log");
}
#[test]
fn lone_dark_oak_sapling_cannot_grow() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 1);
    w.floor("grass_block", 4);
    let sapling = w.id("dark_oak_sapling", &[]);
    w.put(BlockPos::new(0, 64, 0), sapling);
    assert!(!w.bone_meal(BlockPos::new(0, 64, 0)));
}
#[test]
fn azalea_and_mangrove_grow() {
    let Some(f) = fixture() else { return };
    for (block, soil, log) in [
        ("azalea", "grass_block", "oak_log"),
        ("mangrove_propagule", "mud", "mangrove_log"),
    ] {
        let mut grew = false;
        for seed in 0..6 {
            let mut w = World::new(f, seed);
            w.floor(soil, 12);
            let state = w.id(block, &[]);
            w.put(BlockPos::new(0, 64, 0), state);
            for _ in 0..64 {
                if w.count(log) > 0 {
                    break;
                }
                w.bone_meal(BlockPos::new(0, 64, 0));
            }
            grew |= w.count(log) > 0;
        }
        assert!(grew, "{block} grows into a tree");
    }
}
#[test]
fn random_ticks_advance_sapling_stage_then_grow() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 3);
    w.floor("grass_block", 8);
    let sapling = w.id("oak_sapling", &[]);
    let pos = BlockPos::new(0, 64, 0);
    w.put(pos, sapling);
    for _ in 0..2000 {
        let state = w.level.block_state(pos);
        if !f.behaviours.is_randomly_ticking(state) {
            break;
        }
        f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    }
    assert_eq!(w.name(pos), "minecraft:oak_log");
}
#[test]
fn grass_bone_meal_places_grass_and_flowers() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 7);
    w.floor("grass_block", 8);
    assert!(w.bone_meal(BlockPos::new(0, 63, 0)));
    let plants = w.count("short_grass") + w.count("tall_grass");
    let flowers = w.count("dandelion")
        + w.count("poppy")
        + w.count("cornflower")
        + w.count("oxeye_daisy")
        + w.count("azure_bluet")
        + w.count("_tulip");
    assert!(plants > 10, "grass placed: {plants}");
    assert!(plants + flowers > 15);
    assert!(
        w.level
            .events
            .iter()
            .any(|e| matches!(e, LevelEvent::LevelEvent { id: 1505, .. }))
    );
}
#[test]
fn huge_mushroom_and_fungus_grow() {
    let Some(f) = fixture() else { return };
    let mut grew = (false, false);
    for seed in 0..8 {
        let mut w = World::new(f, seed);
        w.floor("mycelium", 8);
        let mushroom = w.id("red_mushroom", &[]);
        w.put(BlockPos::new(0, 64, 0), mushroom);
        for _ in 0..16 {
            w.bone_meal(BlockPos::new(0, 64, 0));
        }
        grew.0 |= w.count("red_mushroom_block") > 0 && w.count("mushroom_stem") > 0;
        let mut w = World::new(f, seed);
        w.floor("crimson_nylium", 8);
        let fungus = w.id("crimson_fungus", &[]);
        w.put(BlockPos::new(0, 64, 0), fungus);
        for _ in 0..16 {
            if w.count("crimson_stem") > 0 {
                break;
            }
            w.bone_meal(BlockPos::new(0, 64, 0));
        }
        grew.1 |= w.count("crimson_stem") > 0 && w.count("nether_wart_block") > 0;
    }
    assert!(grew.0, "huge red mushroom");
    assert!(grew.1, "huge crimson fungus");
}
#[test]
fn crops_age_with_bone_meal() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 11);
    let farmland = w.id("farmland", &[]);
    let pos = BlockPos::new(0, 64, 0);
    w.put(BlockPos::new(0, 63, 0), farmland);
    w.put(pos, w.id("wheat", &[]));
    let mut uses = 0;
    while w.bone_meal(pos) {
        uses += 1;
    }
    assert!((2..=4).contains(&uses), "wheat matured after {uses} uses");
    assert_eq!(w.level.block_state(pos), w.id("wheat", &[("age", "7")]));
    w.put(pos, w.id("torchflower_crop", &[]));
    assert!(w.bone_meal(pos));
    assert!(w.bone_meal(pos));
    assert_eq!(w.name(pos), "minecraft:torchflower");
    assert!(!w.bone_meal(pos), "torchflowers are not bonemealable");
}

#[test]
fn crop_growth_multiplier_preserves_the_vanilla_roll_and_rng_order() {
    let Some(f) = fixture() else { return };
    for seed in 0..256 {
        let mut vanilla = World::new(f, seed);
        let mut accelerated = World::new(f, seed);
        let mut cancelled = World::new(f, seed);
        let pos = BlockPos::new(0, 64, 0);
        let farmland = vanilla.id("farmland", &[]);
        vanilla.put(BlockPos::new(0, 63, 0), farmland);
        accelerated.put(BlockPos::new(0, 63, 0), farmland);
        cancelled.put(BlockPos::new(0, 63, 0), farmland);
        let wheat = vanilla.id("wheat", &[]);
        vanilla.put(pos, wheat);
        accelerated.put(pos, wheat);
        cancelled.put(pos, wheat);
        accelerated.level.crop_growth_multiplier = 3.0;
        cancelled.level.crop_growth_multiplier = 0.0;

        f.behaviours
            .random_tick(&mut vanilla.level, &mut vanilla.ticks, pos);
        f.behaviours
            .random_tick(&mut accelerated.level, &mut accelerated.ticks, pos);
        f.behaviours
            .random_tick(&mut cancelled.level, &mut cancelled.ticks, pos);

        let vanilla_age = f
            .behaviours
            .table
            .int(vanilla.level.block_state(pos), f.behaviours.props.age);
        let accelerated_age = f
            .behaviours
            .table
            .int(accelerated.level.block_state(pos), f.behaviours.props.age);
        assert_eq!(
            accelerated.level.crop_growth_calls,
            vanilla.level.crop_growth_calls
        );
        let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
        let accelerated_next = accelerated.level.random.next_int_bound(1_000_000);
        let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
        assert_eq!(
            vanilla_next, accelerated_next,
            "growth multiplier consumed additional random draws for seed {seed}"
        );
        assert_eq!(
            vanilla_next, cancelled_next,
            "cancelled growth consumed additional random draws for seed {seed}"
        );
        if vanilla_age > 0 {
            assert_eq!(accelerated_age, (vanilla_age + 2).min(7));
            assert_eq!(vanilla.level.crop_growth_calls, 1);
            assert_eq!(
                f.behaviours
                    .table
                    .int(cancelled.level.block_state(pos), f.behaviours.props.age),
                0,
                "zero multiplier should cancel growth"
            );
            assert_eq!(cancelled.level.crop_growth_calls, 1);
            return;
        }
        assert_eq!(accelerated_age, 0);
        assert_eq!(cancelled.level.crop_growth_calls, 0);
        assert_eq!(
            f.behaviours
                .table
                .int(cancelled.level.block_state(pos), f.behaviours.props.age),
            0
        );
        assert_eq!(vanilla.level.crop_growth_calls, 0);
    }
    panic!("no successful vanilla wheat growth roll found in 256 seeds");
}

#[test]
fn crop_growth_hook_covers_other_age_stage_plants() {
    let Some(f) = fixture() else { return };
    for name in [
        "pumpkin_stem",
        "cocoa",
        "nether_wart",
        "sweet_berry_bush",
        "pitcher_crop",
    ] {
        let mut grew = false;
        for seed in 0..128 {
            let mut w = World::new(f, seed);
            let pos = BlockPos::new(0, 64, 0);
            if matches!(name, "pumpkin_stem" | "pitcher_crop") {
                let farmland = w.id("farmland", &[]);
                w.put(BlockPos::new(0, 63, 0), farmland);
            }
            let plant = if name == "pitcher_crop" {
                w.id(name, &[("half", "lower")])
            } else {
                w.id(name, &[])
            };
            w.put(pos, plant);
            w.level.crop_growth_multiplier = 3.0;
            f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
            if w.level.crop_growth_calls == 0 {
                continue;
            }
            let age = f
                .behaviours
                .table
                .int(w.level.block_state(pos), f.behaviours.props.age);
            assert!(age >= 2, "{name} should advance multiple stages, got {age}");
            assert_eq!(w.level.crop_growth_calls, 1);
            grew = true;
            break;
        }
        assert!(grew, "no successful random growth roll found for {name}");
    }
}

#[test]
fn crop_growth_hook_scales_cactus_and_sugar_cane_age_without_extra_rng() {
    let Some(f) = fixture() else { return };
    for name in ["cactus", "sugar_cane"] {
        let mut vanilla = World::new(f, 91);
        let mut accelerated = World::new(f, 91);
        let mut cancelled = World::new(f, 91);
        let pos = BlockPos::new(0, 64, 0);
        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            let sand = world.id("sand", &[]);
            let plant = world.id(name, &[]);
            world.put(BlockPos::new(0, 63, 0), sand);
            world.put(pos, plant);
        }
        accelerated.level.crop_growth_multiplier = 3.0;
        cancelled.level.crop_growth_multiplier = 0.0;

        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            f.behaviours
                .random_tick(&mut world.level, &mut world.ticks, pos);
        }

        let age = |world: &World| {
            f.behaviours
                .table
                .int(world.level.block_state(pos), f.behaviours.props.age)
        };
        assert_eq!(age(&vanilla), 1, "vanilla {name} age");
        assert_eq!(age(&accelerated), 3, "accelerated {name} age");
        assert_eq!(age(&cancelled), 0, "cancelled {name} age");
        assert_eq!(vanilla.level.crop_growth_calls, 1);
        assert_eq!(accelerated.level.crop_growth_calls, 1);
        assert_eq!(cancelled.level.crop_growth_calls, 1);
        let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
        let accelerated_next = accelerated.level.random.next_int_bound(1_000_000);
        let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
        assert_eq!(vanilla_next, accelerated_next, "{name} accelerated RNG");
        assert_eq!(vanilla_next, cancelled_next, "{name} cancelled RNG");
    }
}

#[test]
fn crop_growth_hook_scales_cactus_and_sugar_cane_height() {
    let Some(f) = fixture() else { return };
    for name in ["cactus", "sugar_cane"] {
        for height in [1, 2] {
            let mut vanilla = World::new(f, 37);
            let mut accelerated = World::new(f, 37);
            let mut cancelled = World::new(f, 37);
            let pos = BlockPos::new(0, 64, 0);
            for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
                let soil_pos = BlockPos::new(0, 64 - height, 0);
                let soil = world.id("sand", &[]);
                world.put(soil_pos, soil);
                if name == "sugar_cane" {
                    let water = world.id("water", &[]);
                    world.put(soil_pos.offset(1, 0, 0), water);
                }
                for segment in 0..height {
                    let at = pos.offset(0, -segment, 0);
                    let plant = if segment == 0 {
                        world.id(name, &[("age", "15")])
                    } else {
                        world.id(name, &[])
                    };
                    world.put(at, plant);
                }
            }
            accelerated.level.crop_growth_multiplier = 3.0;
            cancelled.level.crop_growth_multiplier = 0.0;

            for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
                f.behaviours
                    .random_tick(&mut world.level, &mut world.ticks, pos);
            }

            assert_eq!(vanilla.level.crop_growth_calls, 1);
            assert_eq!(accelerated.level.crop_growth_calls, 1);
            assert_eq!(cancelled.level.crop_growth_calls, 1);
            assert_eq!(
                vanilla.count(name),
                (height + 1) as usize,
                "vanilla {name} height"
            );
            assert_eq!(accelerated.count(name), 3, "accelerated {name} height");
            assert_eq!(
                cancelled.count(name),
                height as usize,
                "cancelled {name} height"
            );
            let age = |world: &World| {
                f.behaviours
                    .table
                    .int(world.level.block_state(pos), f.behaviours.props.age)
            };
            assert_eq!(age(&vanilla), 0, "vanilla {name} age reset");
            assert_eq!(age(&accelerated), 0, "accelerated {name} age reset");
            assert_eq!(age(&cancelled), 15, "cancelled {name} keeps age");
            let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
            let accelerated_next = accelerated.level.random.next_int_bound(1_000_000);
            let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
            assert_eq!(vanilla_next, accelerated_next, "{name} accelerated RNG");
            assert_eq!(vanilla_next, cancelled_next, "{name} cancelled RNG");
        }
    }
}

#[test]
fn crop_growth_hook_scales_mushroom_spread_after_a_valid_target() {
    let Some(f) = fixture() else { return };
    let mut found_accelerated_spread = false;
    for seed in 0..512 {
        let mut vanilla = World::new(f, seed);
        let mut accelerated = World::new(f, seed);
        let mut cancelled = World::new(f, seed);
        let pos = BlockPos::new(0, 64, 0);
        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            world.floor("mycelium", 8);
            let mushroom = world.id("red_mushroom", &[]);
            world.put(pos, mushroom);
        }
        accelerated.level.crop_growth_multiplier = 3.0;
        cancelled.level.crop_growth_multiplier = 0.0;

        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            f.behaviours
                .random_tick(&mut world.level, &mut world.ticks, pos);
        }
        if vanilla.level.crop_growth_calls == 0 {
            continue;
        }

        assert_eq!(vanilla.level.crop_growth_calls, 1);
        assert_eq!(accelerated.level.crop_growth_calls, 1);
        assert_eq!(cancelled.level.crop_growth_calls, 1);
        assert_eq!(vanilla.count("red_mushroom"), 2);
        assert_eq!(cancelled.count("red_mushroom"), 1);
        let accelerated_count = accelerated.count("red_mushroom");
        assert!((2..=5).contains(&accelerated_count));
        let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
        let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
        assert_eq!(vanilla_next, cancelled_next, "cancelled spread RNG");
        if accelerated_count > 2 {
            found_accelerated_spread = true;
            break;
        }
    }
    assert!(found_accelerated_spread, "no extra mushroom target found");
}

#[test]
fn crop_growth_hook_scales_vine_spread_after_a_valid_target() {
    let Some(f) = fixture() else { return };
    let mut found_vanilla_growth = false;
    let mut found_accelerated_spread = false;
    for seed in 0..1024 {
        let seed = i64::from(seed) << 32;
        let mut vanilla = World::new(f, seed);
        let mut accelerated = World::new(f, seed);
        let mut cancelled = World::new(f, seed);
        let pos = BlockPos::new(0, 64, 0);
        let north = pos.offset(0, 0, -1);
        let supports = [
            pos.offset(1, 0, 0),
            north,
            pos.offset(0, 0, 1),
            pos.offset(-1, 0, 0),
            pos.offset(0, 1, 0),
        ];
        let vine = vanilla.id("vine", &[("north", "true")]);
        assert!(
            f.behaviours.is_randomly_ticking(vine),
            "fixture vine state must be randomly ticking"
        );
        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            let stone = world.id("stone", &[]);
            world.put(pos, vine);
            for support in supports {
                world.put(support, stone);
            }
        }
        accelerated.level.crop_growth_multiplier = 3.0;
        cancelled.level.crop_growth_multiplier = 0.0;

        for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
            f.behaviours
                .random_tick(&mut world.level, &mut world.ticks, pos);
        }
        if vanilla.level.crop_growth_calls == 0 {
            continue;
        }
        found_vanilla_growth = true;

        assert_eq!(vanilla.level.crop_growth_calls, 1);
        assert_eq!(accelerated.level.crop_growth_calls, 1);
        assert_eq!(cancelled.level.crop_growth_calls, 1);
        assert_eq!(cancelled.level.blocks, {
            let mut original = HashMap::new();
            original.insert(pos, cancelled.id("vine", &[("north", "true")]));
            for support in supports {
                original.insert(support, cancelled.id("stone", &[]));
            }
            original
        });
        let vanilla_faces = vanilla.vine_faces();
        assert!(vanilla_faces > 0);
        let accelerated_faces = accelerated.vine_faces();
        let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
        let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
        assert_eq!(vanilla_next, cancelled_next, "cancelled spread RNG");
        if accelerated_faces > vanilla_faces {
            found_accelerated_spread = true;
            break;
        }
    }
    assert!(
        found_vanilla_growth,
        "no vanilla vine spread hook dispatched"
    );
    assert!(found_accelerated_spread, "no extra vine spread found");
}

#[test]
fn crop_growth_hook_scales_bamboo_height_after_the_vanilla_roll() {
    let Some(f) = fixture() else { return };
    for kind in ["bamboo_sapling", "bamboo"] {
        let mut found_growth = false;
        for seed in 0..128 {
            let mut vanilla = World::new(f, seed);
            let mut accelerated = World::new(f, seed);
            let mut cancelled = World::new(f, seed);
            let pos = BlockPos::new(0, 64, 0);
            for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
                let soil = world.id("sand", &[]);
                world.put(BlockPos::new(0, 63, 0), soil);
                let plant = world.id(kind, &[]);
                world.put(pos, plant);
                if kind == "bamboo" {
                    let lower = world.id("bamboo", &[]);
                    world.put(BlockPos::new(0, 63, 0), lower);
                }
            }
            accelerated.level.crop_growth_multiplier = 3.0;
            cancelled.level.crop_growth_multiplier = 0.0;
            let initial_bamboo = vanilla.count("bamboo");

            for world in [&mut vanilla, &mut accelerated, &mut cancelled] {
                f.behaviours
                    .random_tick(&mut world.level, &mut world.ticks, pos);
            }
            if vanilla.level.crop_growth_calls == 0 {
                continue;
            }
            assert_eq!(vanilla.level.crop_growth_calls, 1);
            assert_eq!(accelerated.level.crop_growth_calls, 1);
            assert_eq!(cancelled.level.crop_growth_calls, 1);
            let vanilla_segments = if kind == "bamboo" { 1 } else { 2 };
            assert_eq!(
                vanilla.count("bamboo"),
                initial_bamboo + vanilla_segments,
                "vanilla {kind} growth: before={initial_bamboo}, at64={}, at65={}",
                vanilla.name(pos),
                vanilla.name(pos.offset(0, 1, 0))
            );
            assert_eq!(
                accelerated.count("bamboo"),
                vanilla.count("bamboo") + 2,
                "accelerated {kind} height"
            );
            assert_eq!(
                cancelled.count("bamboo"),
                initial_bamboo,
                "cancelled {kind} growth"
            );
            let vanilla_next = vanilla.level.random.next_int_bound(1_000_000);
            let accelerated_next = accelerated.level.random.next_int_bound(1_000_000);
            let cancelled_next = cancelled.level.random.next_int_bound(1_000_000);
            assert_eq!(vanilla_next, accelerated_next, "{kind} accelerated RNG");
            assert_eq!(vanilla_next, cancelled_next, "{kind} cancelled RNG");
            found_growth = true;
            break;
        }
        assert!(found_growth, "no successful vanilla growth roll for {kind}");
    }
}

#[test]
fn tall_flowers_and_seagrass() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 5);
    w.floor("dirt", 2);
    let fern = w.id("fern", &[]);
    w.put(BlockPos::new(0, 64, 0), fern);
    assert!(w.bone_meal(BlockPos::new(0, 64, 0)));
    assert_eq!(w.name(BlockPos::new(0, 64, 0)), "minecraft:large_fern");
    assert_eq!(w.name(BlockPos::new(0, 65, 0)), "minecraft:large_fern");
    let lilac = w.id("lilac", &[("half", "lower")]);
    w.put(BlockPos::new(1, 64, 0), lilac);
    assert!(w.bone_meal(BlockPos::new(1, 64, 0)));
    assert!(w.level.events.iter().any(|e| matches!(
        e,
        LevelEvent::BlockEvent {
            kind: "pop_block_item",
            ..
        }
    )));
}

#[test]
fn snowy_biomes_freeze_and_snow() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 1);
    w.level.biome = "minecraft:snowy_plains";
    w.floor("grass_block", 2);
    let water = w.id("water", &[]);
    w.put(BlockPos::new(1, 63, 0), water);
    let b = &f.behaviours;
    let grass = b
        .precipitation_info(&w.level, BlockPos::new(0, 70, 0))
        .unwrap();
    assert_eq!(grass.top, BlockPos::new(0, 64, 0));
    assert!(grass.snows_at_top && !grass.freezes_below);
    assert_eq!(grass.kind, PrecipitationKind::Snow);
    let pond = b
        .precipitation_info(&w.level, BlockPos::new(1, 70, 0))
        .unwrap();
    assert!(pond.freezes_below, "a water source at the edge freezes");
    w.level.biome = "minecraft:plains";
    let rain = b
        .precipitation_info(&w.level, BlockPos::new(0, 70, 0))
        .unwrap();
    assert!(!rain.snows_at_top && !rain.freezes_below);
    assert_eq!(rain.kind, PrecipitationKind::Rain);
}

#[test]
fn eyeblossoms_open_at_night_and_wake_neighbours() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 2);
    w.floor("grass_block", 4);
    let closed = w.id("closed_eyeblossom", &[]);
    w.put(BlockPos::new(0, 64, 0), closed);
    w.put(BlockPos::new(2, 64, 1), closed);
    f.behaviours
        .random_tick(&mut w.level, &mut w.ticks, BlockPos::new(0, 64, 0));
    assert_eq!(
        w.name(BlockPos::new(0, 64, 0)),
        "minecraft:closed_eyeblossom",
        "closed by day"
    );
    w.level.day = 13000;
    f.behaviours
        .random_tick(&mut w.level, &mut w.ticks, BlockPos::new(0, 64, 0));
    assert_eq!(w.name(BlockPos::new(0, 64, 0)), "minecraft:open_eyeblossom");
    let block = f.behaviours.block(closed);
    assert!(
        w.ticks.has_block_tick(BlockPos::new(2, 64, 1), block),
        "neighbour scheduled"
    );
    assert!(w.level.events.iter().any(|e| matches!(e, LevelEvent::Sound { sound, .. } if &**sound == "minecraft:block.eyeblossom.open_long")));
}
#[test]
fn turtle_eggs_crack_at_dawn_on_sand() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 3);
    w.floor("sand", 1);
    let egg = w.id("turtle_egg", &[]);
    let pos = BlockPos::new(0, 64, 0);
    w.put(pos, egg);
    w.level.day = 21500;
    f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    assert_eq!(
        w.level.block_state(pos),
        w.id("turtle_egg", &[("hatch", "1")])
    );
    f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    assert_eq!(w.name(pos), "minecraft:air", "hatched");
}
#[test]
fn sniffer_eggs_crack_twice_then_hatch_a_snifflet() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f, 5);
    w.floor("moss_block", 1);
    let egg = w.id("sniffer_egg", &[]);
    let pos = BlockPos::new(0, 64, 0);
    f.behaviours
        .set_block(&mut w.level, &mut w.ticks, pos, egg, flags::UPDATE_ALL);
    let block = f.behaviours.block(egg);
    assert!(
        w.ticks.has_block_tick(pos, block),
        "placing schedules the first stage"
    );
    for stage in 1..=2 {
        f.behaviours
            .scheduled_block_tick(&mut w.level, &mut w.ticks, pos);
        assert_eq!(
            w.level.block_state(pos),
            w.id("sniffer_egg", &[("hatch", &stage.to_string())])
        );
    }
    f.behaviours
        .scheduled_block_tick(&mut w.level, &mut w.ticks, pos);
    assert_eq!(w.name(pos), "minecraft:air", "hatched");
    assert!(w.level.events.iter().any(|e| matches!(
        e,
        LevelEvent::BlockEvent {
            kind: "hatch_sniffer",
            ..
        }
    )));
}

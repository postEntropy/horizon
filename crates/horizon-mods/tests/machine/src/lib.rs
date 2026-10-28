//! A block entity and API 0.3/0.4/0.5 logic, machine and deep mod for the
//! runtime's tests: `test_machine:smelter` has an input and an output slot and
//! smelts one input item every recipe's cooking time, `lit` while it does. Its
//! progress is saved and shown as the menu's one data value; TNT in its input
//! makes it panic (after lighting up). `test_machine:crate` holds 4 slots and
//! never ticks. Also exercises API 0.3 (`config`, `server`, `keys`, `target`,
//! `inventory`, `on_block`, `on_item`, `on_server`), API 0.4 (`capabilities`,
//! `ui`, `net`, `recipes`, `world`, `render`, `on_menu_click`,
//! `on_menu_button`, `on_packet`) and API 0.5 (`entities`, `mod-entities`,
//! `models`, `dimensions`, `vanilla-hooks`).
use horizon_mod_sdk::{
    AiGoal, BlockAction, BlockEntityType, BlockEvent, EntityType, EventOutcome, ItemAction,
    ItemEvent, LogicHooks, Mod, Registration, ServerEvent, ServerEventKind, SpawnCategory,
    VanillaHookEvent, VanillaHookKind,
    capabilities::{self, EnergyView, FluidView},
    config,
    dimensions::{self, DimensionSpec, GeneratorKind, PortalSpec},
    entities, item_data,
    keys::{self, KeyBinding},
    machine::{self, CookingKind, ItemStack},
    models::{self, BonePose, DrawPart, DrawPartKind},
    net,
    recipes::{self, IngredientSlot, RecipeEntry},
    render::{self, BoxShape, GhostBlock},
    server::{self, BlockPos, ItemDrop},
    target,
    ui::{self, ScreenSpec, Widget, WidgetKind},
    vanilla_hooks, world,
};
use std::collections::HashMap;

struct Machines {
    /// Each smelter's ticks of cooking so far.
    progress: HashMap<u32, u32>,
    /// Each custom entity's age in ticks.
    entity_ticks: HashMap<u32, u32>,
    last_packet: String,
    clicked_slot: Option<(u32, u8)>,
    clicked_button: Option<u8>,
}

impl Mod for Machines {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: "vein_mine".into(),
            label: "Vein Mine".into(),
            default_key: "KeyV".into(),
        });
        let _ = vanilla_hooks::subscribe("loot-drop");
        let _ = vanilla_hooks::subscribe("entity-damage");
        let _ = vanilla_hooks::subscribe("xp-gain");
        let _ = dimensions::register_dimension(&DimensionSpec {
            id: "test_machine:twilight".into(),
            ultrawarm: false,
            natural: true,
            coordinate_scale: 1.0,
            has_skylight: true,
            has_ceiling: false,
            min_y: 0,
            height: 256,
            ambient_light: 0.1,
            fixed_time: Some(12000),
            generator: GeneratorKind::Noise,
            surface_block: "minecraft:grass_block".into(),
        });
        let _ = dimensions::register_portal(&PortalSpec {
            id: "test_machine:twilight_portal".into(),
            frame_block: "minecraft:mossy_cobblestone".into(),
            activation_item: "minecraft:diamond".into(),
            target_dimension: "test_machine:twilight".into(),
            tint_color: 0x55AAFF,
        });
        let progress = HashMap::new();
        Ok((
            Machines {
                progress,
                entity_ticks: HashMap::new(),
                last_packet: String::new(),
                clicked_slot: None,
                clicked_button: None,
            },
            Registration::new().command("machine_api"),
        ))
    }
    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![
            BlockEntityType {
                block: "test_machine:smelter".into(),
                slots: 2,
                ticks: true,
            },
            BlockEntityType {
                block: "test_machine:crate".into(),
                slots: 4,
                ticks: false,
            },
        ]
    }
    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
            .block("test_machine:smelter")
            .item("test_machine:wrench")
            .server_event(ServerEventKind::PlayerJoin)
            .channel("test_machine:sync")
    }
    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            let face = server::get_block_face().unwrap_or_default();
            let _ = ui::open_screen(&ScreenSpec {
                id: "test_machine:smelter_menu".into(),
                title: "Smelter".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 1,
                        kind: WidgetKind::ProgressArrow,
                        x: 79,
                        y: 34,
                        width: 24,
                        height: 17,
                        text: "0%".into(),
                        value: 0,
                        max_value: 100,
                    },
                    Widget {
                        id: 2,
                        kind: WidgetKind::Slot,
                        x: 20,
                        y: 20,
                        width: 18,
                        height: 18,
                        text: String::new(),
                        value: 0,
                        max_value: 0,
                    },
                ],
            });
            ui::set_slot("test_machine:smelter_menu", 2, "minecraft:iron_ingot", 3);
            ui::set_backgrounds(
                "test_machine:smelter_menu",
                &[ui::ScreenBackground {
                    texture: "test_machine:smelter".into(),
                    x: 0,
                    y: 0,
                    width: 176,
                    height: 166,
                }],
            );
            if face != "north" {
                return EventOutcome::Pass;
            }
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseOnBlock {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            let _ = server::set_block(&event.dimension, pos, "test_machine:smelter[lit=true]");
            server::spawn_drop(
                &event.dimension,
                pos,
                &ItemDrop {
                    id: "minecraft:iron_ingot".into(),
                    count: 2,
                },
            );
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if event.kind == ServerEventKind::PlayerJoin {
            let _ = server::set_rule("keepInventory", "true");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
    fn on_menu_click(&mut self, screen_id: &str, slot: u32, button: u8) {
        self.clicked_slot = Some((slot, button));
        ui::set_gauge(screen_id, 1, 50, 100, "50%");
    }
    fn on_menu_button(&mut self, _screen_id: &str, button: u8) {
        self.clicked_button = Some(button);
        ui::close_screen();
    }
    fn on_packet(&mut self, channel: &str, payload: &[u8], from_player: Option<&str>) {
        self.last_packet = format!(
            "{channel}:{}:{}",
            from_player.unwrap_or("server"),
            String::from_utf8_lossy(payload)
        );
        let _ = net::send_to_server("test_machine:sync", b"ack");
    }
    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args {
            "config" => {
                let before = config::get_int("speed").unwrap_or(0);
                config::set_int("speed", 5)?;
                let after = config::get_int("speed").unwrap_or(0);
                Ok(Some(format!("speed {before}->{after}")))
            }
            "client_reads" => {
                let pressed = keys::was_pressed("vein_mine");
                let hit = target::get_block()
                    .map(|b| b.state)
                    .unwrap_or_else(|| "none".into());
                let hand = horizon_mod_sdk::inventory::get_main_hand()
                    .map(|s| format!("{}*{}", s.id, s.count))
                    .unwrap_or_else(|| "empty".into());
                let blk = world::get_block(0, 64, 0).unwrap_or_else(|| "air".into());
                let biome = world::get_biome(0, 64, 0).unwrap_or_else(|| "none".into());
                let light = world::get_light(0, 64, 0).unwrap_or(0);
                render::set_shapes(&[BoxShape {
                    min_x: 0.0,
                    min_y: 64.0,
                    min_z: 0.0,
                    max_x: 1.0,
                    max_y: 65.0,
                    max_z: 1.0,
                    color: 0x8000_FF00,
                }]);
                render::set_ghost_blocks(&[GhostBlock {
                    x: 1,
                    y: 64,
                    z: 0,
                    state: "test_machine:smelter".into(),
                    alpha: 0.5,
                }]);
                Ok(Some(format!(
                    "key={pressed} hit={hit} hand={hand} blk={blk} biome={biome} light={light}"
                )))
            }
            "caps_and_recipes" => {
                let pos = "minecraft:overworld:0,64,0";
                capabilities::set_energy_buffer(
                    pos,
                    EnergyView {
                        stored: 1000,
                        capacity: 5000,
                        max_receive: 500,
                        max_extract: 500,
                    },
                );
                let recv = capabilities::receive_energy(pos, 800, false);
                let ext = capabilities::extract_energy(pos, 300, false);
                let stored = capabilities::get_energy(pos).map_or(0, |e| e.stored);

                capabilities::set_fluid_tank(
                    pos,
                    0,
                    &FluidView {
                        fluid: String::new(),
                        amount: 0,
                        capacity: 4000,
                    },
                );
                let filled = capabilities::fill_fluid(pos, "minecraft:lava", 1500, false);
                let drained = capabilities::drain_fluid(pos, "minecraft:lava", 500, false);

                capabilities::set_attachment("entity:player", "test_machine:mana", &[42])?;
                let mana = capabilities::get_attachment("entity:player", "test_machine:mana")
                    .and_then(|b| b.first().copied())
                    .unwrap_or(0);
                capabilities::set_accessory("player", "ring", 0, "test_machine:ring", 1)?;
                let ring = capabilities::get_accessory("player", "ring", 0)
                    .map(|s| s.id)
                    .unwrap_or_default();

                recipes::register(&RecipeEntry {
                    id: "test_machine:crush_iron".into(),
                    kind: "test_machine:crusher".into(),
                    inputs: vec![IngredientSlot {
                        items: vec!["#c:raw_iron".into()],
                        count: 1,
                    }],
                    outputs: vec!["minecraft:iron_ingot*2".into()],
                    ticks: 100,
                    energy: 2000,
                })?;
                let found = recipes::by_type("test_machine:crusher").len();
                let found_by_item = recipes::for_input("minecraft:raw_iron").len();
                Ok(Some(format!(
                    "fe={recv}/{ext}/{stored} fluid={filled}/{drained} mana={mana} ring={ring} recipes={found}/{found_by_item}"
                )))
            }
            "state" => Ok(Some(format!(
                "pkt={} click={:?} btn={:?}",
                self.last_packet, self.clicked_slot, self.clicked_button
            ))),
            "deep_api" => {
                let h = entities::spawn(
                    "minecraft:overworld",
                    "test_machine:automaton",
                    0.0,
                    64.0,
                    0.0,
                )?;
                let near = entities::nearby("minecraft:overworld", 0.0, 64.0, 0.0, 5.0).len();
                models::set_bone_poses(
                    "block:0,64,0",
                    &[BonePose {
                        bone: "shaft".into(),
                        rot_x: 0.0,
                        rot_y: 45.0,
                        rot_z: 0.0,
                        pos_x: 0.0,
                        pos_y: 0.0,
                        pos_z: 0.0,
                        scale_x: 1.0,
                        scale_y: 1.0,
                        scale_z: 1.0,
                    }],
                );
                models::set_draw_list(
                    "block:0,64,0",
                    &[DrawPart {
                        kind: DrawPartKind::ItemDisplay,
                        asset: "minecraft:diamond".into(),
                        x: 0.5,
                        y: 1.2,
                        z: 0.5,
                        rot_x: 0.0,
                        rot_y: 90.0,
                        rot_z: 0.0,
                        scale_x: 0.5,
                        scale_y: 0.5,
                        scale_z: 0.5,
                        color: 0xFFFF_FFFF,
                    }],
                );
                let dims = dimensions::list_dimensions().len();
                dimensions::teleport("Player", "test_machine:twilight", 0.5, 80.0, 0.5)?;
                Ok(Some(format!("spawned={h} near={near} dims={dims}")))
            }
            _ => Err("unknown subcommand".into()),
        }
    }
    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        match event.kind {
            VanillaHookKind::LootDrop => {
                vanilla_hooks::add_loot_drop("minecraft:diamond", 1);
                EventOutcome::Consume
            }
            VanillaHookKind::EntityDamage => {
                vanilla_hooks::set_damage_multiplier(0.5);
                EventOutcome::Consume
            }
            VanillaHookKind::XpGain => {
                vanilla_hooks::set_xp_multiplier(2.0);
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }
    fn entity_types(&mut self) -> Vec<EntityType> {
        vec![EntityType {
            id: "test_machine:automaton".into(),
            width: 0.6,
            height: 1.8,
            max_health: 40.0,
            move_speed: 0.25,
            attack_damage: 6.0,
            category: SpawnCategory::Monster,
            fire_immune: true,
            goals: vec![
                AiGoal::FloatInWater,
                AiGoal::MeleeAttack,
                AiGoal::RandomStroll,
                AiGoal::LookAtPlayer,
            ],
        }]
    }
    fn create_entity(&mut self, handle: u32, kind: &str, saved: &[u8]) {
        if kind == "test_machine:automaton" {
            let age = saved.try_into().map_or(0, u32::from_le_bytes);
            self.entity_ticks.insert(handle, age);
        }
    }
    fn tick_entities(&mut self, handles: &[u32]) {
        for &h in handles {
            let age = self.entity_ticks.entry(h).or_default();
            *age += 1;
            let _ = entities::navigate_to(h, 10.0, 64.0, 10.0, 0.5);
            models::set_animation(&format!("entity:{h}"), "walk", true, 1.0);
        }
    }
    fn save_entity(&mut self, handle: u32) -> Vec<u8> {
        self.entity_ticks
            .get(&handle)
            .copied()
            .unwrap_or(0)
            .to_le_bytes()
            .to_vec()
    }
    fn remove_entity(&mut self, handle: u32) {
        self.entity_ticks.remove(&handle);
    }
    fn on_entity_interact(&mut self, handle: u32, _player: &str, held_item: &str) -> bool {
        if held_item == "test_machine:wrench" {
            let _ = entities::set_health(handle, 20.0);
            return true;
        }
        false
    }
    fn on_entity_hurt(&mut self, _handle: u32, _amount: f32, _source: &str) -> bool {
        true
    }
    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        if block == "test_machine:smelter" {
            let progress = saved.try_into().map_or(0, u32::from_le_bytes);
            self.progress.insert(handle, progress);
        }
    }
    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &h in handles {
            let components = item_data::get_slot_components(h, 0);
            if !components.is_empty() {
                let _ = item_data::set_slot_components(h, 0, &components);
            }
            let progress = self.progress.entry(h).or_default();
            if machine::get_slot(h, 0).is_some_and(|s| s.id == "minecraft:tnt") {
                machine::set_property(h, "lit", "true");
                panic!("tnt in the smelter");
            }
            let recipe = machine::get_slot(h, 0)
                .and_then(|input| machine::cooking(CookingKind::Smelting, &input.id));
            let Some(recipe) = recipe else {
                *progress = 0;
                machine::set_property(h, "lit", "false");
                continue;
            };
            machine::set_property(h, "lit", "true");
            *progress += 1;
            if *progress < recipe.cooking_time {
                continue;
            }
            *progress = 0;
            let out = match machine::get_slot(h, 1) {
                Some(o) if o.id == recipe.output.id => ItemStack {
                    count: o.count + recipe.output.count,
                    ..o
                },
                Some(_) => continue,
                None => recipe.output,
            };
            if !machine::set_slot(h, 1, Some(&out)) {
                continue;
            }
            let input = machine::get_slot(h, 0).filter(|i| i.count > 1);
            let rest = input.map(|i| ItemStack {
                count: i.count - 1,
                ..i
            });
            machine::set_slot(h, 0, rest.as_ref());
        }
    }
    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let progress = self.progress.get(&handle).copied().unwrap_or(0);
        progress.to_le_bytes().to_vec()
    }
    fn remove_block_entity(&mut self, handle: u32) {
        self.progress.remove(&handle);
    }
    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        vec![self.progress.get(&handle).copied().unwrap_or(0) as i32]
    }
}

horizon_mod_sdk::export_mod!(Machines);

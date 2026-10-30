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
    entities, item_data, item_transfer,
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
    last_server_event: String,
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
        let _ = vanilla_hooks::subscribe("crop-grow");
        let _ = vanilla_hooks::subscribe("explosion-detonate");
        let _ = vanilla_hooks::subscribe("enchantment-level");
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
                last_server_event: String::new(),
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
            .block("minecraft:composter")
            .item("test_machine:wrench")
            .server_event(ServerEventKind::PlayerJoin)
            .server_event(ServerEventKind::PlayerLeave)
            .server_event(ServerEventKind::BlockBreak)
            .server_event(ServerEventKind::BlockPlace)
            .server_event(ServerEventKind::EntitySpawn)
            .server_event(ServerEventKind::EntityDeath)
            .channel("test_machine:sync")
    }
    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::ScheduledTick
            && event.state.starts_with("minecraft:composter")
        {
            let _ = server::set_block(
                &event.dimension,
                BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                },
                "minecraft:emerald_block",
            );
            return EventOutcome::Pass;
        }
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
                    Widget {
                        id: 3,
                        kind: WidgetKind::Button,
                        x: 100,
                        y: 55,
                        width: 56,
                        height: 20,
                        text: "Power".into(),
                        value: 0,
                        max_value: 0,
                    },
                    Widget {
                        id: 4,
                        kind: WidgetKind::Button,
                        x: 100,
                        y: 20,
                        width: 56,
                        height: 20,
                        text: "Restore?".into(),
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
        if event.action == ItemAction::UseOnBlock && event.item != "minecraft:bone_meal" {
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
            let _ = server::spawn_mob(&event.dimension, pos, "minecraft:iron_golem");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        let kind = match &event.kind {
            ServerEventKind::PlayerJoin => "player-join",
            ServerEventKind::PlayerLeave => "player-leave",
            ServerEventKind::EntitySpawn => "entity-spawn",
            ServerEventKind::EntityDeath => "entity-death",
            ServerEventKind::BlockBreak => "block-break",
            ServerEventKind::BlockPlace => "block-place",
        };
        self.last_server_event = format!(
            "{kind}:{}:{}:{},{},{}:{}",
            event.subject, event.dimension, event.x, event.y, event.z, event.detail
        );
        if event.kind == ServerEventKind::PlayerJoin {
            let _ = server::set_rule("keepInventory", "true");
            if event.subject == "ModQueueProbe" {
                let _ = server::set_block(
                    "minecraft:the_nether",
                    BlockPos { x: 1, y: 64, z: 1 },
                    "minecraft:gold_block",
                );
                server::schedule_tick(
                    "minecraft:the_nether",
                    BlockPos { x: 1, y: 64, z: 1 },
                    "minecraft:gold_block",
                    1,
                );
                server::spawn_drop(
                    "minecraft:the_nether",
                    BlockPos { x: 2, y: 64, z: 2 },
                    &ItemDrop {
                        id: "minecraft:diamond".into(),
                        count: 1,
                    },
                );
                let _ = server::spawn_mob(
                    "minecraft:the_nether",
                    BlockPos { x: 3, y: 64, z: 3 },
                    "minecraft:cow",
                );
            }
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
    fn on_menu_click(&mut self, screen_id: &str, slot: u32, button: u8) {
        self.clicked_slot = Some((slot, button));
        ui::set_gauge(screen_id, 1, 50, 100, "50%");
        let _ = capabilities::set_attachment(
            "level:minecraft:overworld",
            "test_machine:menu_callback_seen",
            &[42],
        );
        if screen_id == "test_machine:smelter_menu" && slot == 0 && button == 0 {
            let _ = server::set_block(
                "minecraft:overworld",
                BlockPos {
                    x: -28,
                    y: 70,
                    z: -9,
                },
                "minecraft:emerald_block",
            );
        }
    }
    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        self.clicked_button = Some(button);
        let restored = capabilities::get_attachment(
            "level:minecraft:overworld",
            "test_machine:menu_callback_seen",
        )
        .as_deref()
            == Some(&[42][..]);
        if screen_id == "test_machine:smelter_menu" && restored && button == 3 {
            let _ = server::set_block(
                "minecraft:overworld",
                BlockPos {
                    x: -27,
                    y: 70,
                    z: -9,
                },
                "minecraft:gold_block",
            );
        }
        if screen_id == "test_machine:smelter_menu" && button == 4 {
            let _ = server::set_block(
                "minecraft:overworld",
                BlockPos {
                    x: -26,
                    y: 70,
                    z: -9,
                },
                if restored {
                    "minecraft:diamond_block"
                } else {
                    "minecraft:redstone_block"
                },
            );
        }
        if button == 2 || button == 3 {
            ui::close_screen();
        }
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
                let int_rejected = config::set_int("speed", 99).is_err();
                let int_after_rejection = config::get_int("speed").unwrap_or(0);

                let bool_default = config::get_bool("enabled").unwrap_or(false);
                config::set_bool("enabled", false)?;
                let bool_after = config::get_bool("enabled").unwrap_or(true);

                let float_default = config::get_float("scale").unwrap_or(0.0);
                config::set_float("scale", 1.5)?;
                let float_rejected = config::set_float("scale", 3.0).is_err();
                let float_after = config::get_float("scale").unwrap_or(0.0);

                let string_default = config::get_string("mode").unwrap_or_default();
                config::set_string("mode", "fast")?;
                let choice_rejected = config::set_string("mode", "chaotic").is_err();
                let string_after = config::get_string("mode").unwrap_or_default();

                Ok(Some(format!(
                    "speed={before}->{after}/reject={int_rejected}/kept={int_after_rejection} bool={bool_default}->{bool_after} float={float_default}->{float_after}/reject={float_rejected} mode={string_default}->{string_after}/reject={choice_rejected}"
                )))
            }
            "item_facts" => {
                let remainder =
                    machine::remainder("minecraft:milk_bucket").unwrap_or_else(|| "none".into());
                Ok(Some(format!("remainder={remainder}")))
            }
            "client_reads" => {
                let pressed = keys::was_pressed("vein_mine");
                let hit = target::get_block()
                    .map(|b| b.state)
                    .unwrap_or_else(|| "none".into());
                let hand = horizon_mod_sdk::inventory::get_main_hand()
                    .map(|s| format!("{}*{}", s.id, s.count))
                    .unwrap_or_else(|| "empty".into());
                let container = horizon_mod_sdk::inventory::get_open_container()
                    .first()
                    .map(|s| format!("{}*{}@{}", s.id, s.count, s.slot))
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
                    "key={pressed} hit={hit} hand={hand} container={container} blk={blk} biome={biome} light={light}"
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
                let recv_sim = capabilities::receive_energy(pos, 800, true);
                let recv = capabilities::receive_energy(pos, 800, false);
                let recv_again = capabilities::receive_energy(pos, 800, false);
                let ext_sim = capabilities::extract_energy(pos, 300, true);
                let ext = capabilities::extract_energy(pos, 300, false);
                let ext_again = capabilities::extract_energy(pos, 300, false);
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
                let incompatible = capabilities::fill_fluid(pos, "minecraft:water", 500, true);
                let simulated_drain = capabilities::drain_fluid(pos, "minecraft:lava", 400, true);
                let after_simulation =
                    capabilities::get_fluid(pos, 0).map_or(0, |tank| tank.amount);

                capabilities::set_attachment("entity:player", "test_machine:mana", &[42])?;
                let mana = capabilities::get_attachment("entity:player", "test_machine:mana")
                    .and_then(|b| b.first().copied())
                    .unwrap_or(0);
                capabilities::set_accessory("player", "ring", 0, "test_machine:ring", 1)?;
                let ring = capabilities::get_accessory("player", "ring", 0)
                    .map(|s| s.id)
                    .unwrap_or_default();

                let recipe = RecipeEntry {
                    id: "test_machine:crush_iron".into(),
                    kind: "test_machine:crusher".into(),
                    inputs: vec![IngredientSlot {
                        items: vec!["minecraft:raw_iron".into()],
                        count: 1,
                    }],
                    outputs: vec!["minecraft:iron_ingot*2".into()],
                    ticks: 100,
                    energy: 2000,
                };
                recipes::register(&recipe)?;
                let by_id = recipes::get("test_machine:crush_iron").is_some_and(|entry| {
                    entry.kind == "test_machine:crusher" && entry.outputs == recipe.outputs
                });
                let found = recipes::by_type("test_machine:crusher").len();
                let found_by_item = recipes::for_input("minecraft:raw_iron").len();
                let found_by_output = recipes::for_output("minecraft:iron_ingot").len();
                let mut replacement = recipe;
                replacement.outputs = vec!["minecraft:gold_ingot*1".into()];
                recipes::register(&replacement)?;
                let replaced = recipes::get("test_machine:crush_iron")
                    .is_some_and(|entry| entry.outputs == replacement.outputs);
                let old_output = recipes::for_output("minecraft:iron_ingot").len();
                let new_output = recipes::for_output("minecraft:gold_ingot").len();
                let resource_id = recipes::get("minecraft:raw_iron_smelting").is_some();
                let resource_type = recipes::by_type("minecraft:smelting").len();
                let replaced_resource_type = recipes::by_type("test_machine:resource_recipe").len();
                let replaced_resource_output = recipes::for_output("minecraft:copper_ingot").len();
                Ok(Some(format!(
                    "fe={recv_sim}/{recv}/{recv_again}/{ext_sim}/{ext}/{ext_again}/{stored} fluid={filled}/{drained}/{incompatible}/{simulated_drain}/{after_simulation} mana={mana} ring={ring} recipes={by_id}/{found}/{found_by_item}/{found_by_output}/{replaced}/{old_output}/{new_output} resource={resource_id}/{resource_type}/{replaced_resource_type}/{replaced_resource_output}"
                )))
            }
            "state" => Ok(Some(format!(
                "pkt={} click={:?} btn={:?}",
                self.last_packet, self.clicked_slot, self.clicked_button
            ))),
            "last_server_event" => Ok(Some(self.last_server_event.clone())),
            "set_other_dimension" => {
                server::set_block(
                    "minecraft:the_nether",
                    BlockPos { x: 1, y: 64, z: 1 },
                    "minecraft:gold_block",
                )?;
                Ok(Some("requested block in the Nether".into()))
            }
            "schedule_test_tick" => {
                // Keep the target in the player's block-ticking chunk so the
                // end-to-end fixture proves execution, not only queueing.
                let pos = BlockPos {
                    x: -31,
                    y: 70,
                    z: -9,
                };
                server::set_block("minecraft:overworld", pos, "minecraft:composter[level=7]")?;
                server::schedule_tick("minecraft:overworld", pos, "minecraft:composter", 1);
                Ok(Some("scheduled composter tick".into()))
            }
            "set_machine_block" => {
                server::set_block(
                    "minecraft:overworld",
                    BlockPos {
                        x: -31,
                        y: 70,
                        z: -9,
                    },
                    "test_machine:smelter",
                )?;
                Ok(Some("requested test machine block".into()))
            }
            "queue_inframe_far" => {
                let pos = BlockPos {
                    x: 240,
                    y: 64,
                    z: 0,
                };
                server::set_block("minecraft:overworld", pos, "minecraft:gold_block")?;
                server::spawn_drop(
                    "minecraft:overworld",
                    BlockPos {
                        x: 241,
                        y: 64,
                        z: 0,
                    },
                    &ItemDrop {
                        id: "minecraft:diamond".into(),
                        count: 1,
                    },
                );
                server::spawn_mob(
                    "minecraft:overworld",
                    BlockPos {
                        x: 242,
                        y: 64,
                        z: 0,
                    },
                    "minecraft:cow",
                )?;
                Ok(Some("queued far-world actions".into()))
            }
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
            "spawn_automaton" => {
                let h = entities::spawn(
                    "minecraft:overworld",
                    "test_machine:automaton",
                    -36.5,
                    70.0,
                    -9.5,
                )?;
                let near = entities::nearby("minecraft:overworld", -36.5, 70.0, -9.5, 4.0)
                    .into_iter()
                    .filter(|s| s.kind == "test_machine:automaton")
                    .count();
                let temp = entities::spawn(
                    "minecraft:overworld",
                    "test_machine:automaton",
                    -36.5,
                    70.0,
                    -9.5,
                )?;
                let despawned = entities::despawn(temp) && entities::get_state(temp).is_none();
                Ok(Some(format!(
                    "spawned={h} near={near} despawned={despawned}"
                )))
            }
            "entity_status" => {
                let mut found = None;
                for (&h, &age) in &self.entity_ticks {
                    if let Some(state) = entities::get_state(h) {
                        found = Some((h, age, state));
                        break;
                    }
                }
                let Some((h, age, state)) = found else {
                    return Ok(Some(format!("count={}", self.entity_ticks.len())));
                };
                Ok(Some(format!(
                    "count={} handle={h} age={age} hp={:.1} pos={:.1},{:.1},{:.1}",
                    self.entity_ticks.len(),
                    state.health,
                    state.x,
                    state.y,
                    state.z
                )))
            }
            "control_automaton" => {
                let Some(&h) = self
                    .entity_ticks
                    .keys()
                    .find(|&&h| entities::get_state(h).is_some())
                else {
                    return Err("no active automaton".into());
                };
                if !entities::set_position(h, -36.5, 70.0, -10.5, 90.0, 0.0)
                    || !entities::set_velocity(h, 0.0, 0.0, 0.0)
                    || !entities::navigate_to(h, -36.5, 70.0, -10.5, 0.25)
                {
                    return Err("failed to control automaton".into());
                }
                Ok(Some(format!("controlled={h}")))
            }
            _ => Err("unknown subcommand".into()),
        }
    }
    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        match event.kind {
            VanillaHookKind::LootDrop => {
                if event.subject.starts_with("minecraft:chest") {
                    if event.context != "minecraft:container_contents" {
                        return EventOutcome::Cancel;
                    }
                    if event.x == -33 {
                        return EventOutcome::Cancel;
                    }
                    if event.x == -34 {
                        vanilla_hooks::add_loot_drop("minecraft:emerald", 2);
                        return EventOutcome::Pass;
                    }
                    vanilla_hooks::add_loot_drop("minecraft:diamond", 1);
                    return EventOutcome::Consume;
                }
                if matches!(
                    event.subject.as_str(),
                    "minecraft:stone" | "minecraft:blocks/gravel" | "minecraft:coal_ore"
                ) {
                    vanilla_hooks::add_loot_drop("minecraft:diamond", 1);
                    return EventOutcome::Consume;
                }
                EventOutcome::Pass
            }
            VanillaHookKind::EntityDamage => {
                if event.subject == "minecraft:player" && event.context.contains("cactus") {
                    return EventOutcome::Cancel;
                }
                vanilla_hooks::set_damage_multiplier(0.5);
                EventOutcome::Consume
            }
            VanillaHookKind::XpGain => {
                vanilla_hooks::set_xp_multiplier(2.0);
                EventOutcome::Consume
            }
            VanillaHookKind::CropGrow => {
                vanilla_hooks::set_crop_growth_multiplier(3.0);
                EventOutcome::Pass
            }
            VanillaHookKind::ExplosionDetonate if event.subject == "minecraft:stone" => {
                EventOutcome::Cancel
            }
            VanillaHookKind::EnchantmentLevel => match event.subject.as_str() {
                "minecraft:looting" => {
                    vanilla_hooks::add_enchantment_bonus(2);
                    EventOutcome::Pass
                }
                "minecraft:fortune" => {
                    vanilla_hooks::add_enchantment_bonus(4);
                    EventOutcome::Consume
                }
                "minecraft:sharpness" if event.context.starts_with("minecraft:wooden_sword") => {
                    vanilla_hooks::add_enchantment_bonus(5);
                    EventOutcome::Consume
                }
                "minecraft:silk_touch" => EventOutcome::Cancel,
                _ => EventOutcome::Pass,
            },
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
            if entities::get_state(h).is_none_or(|s| s.x.abs() < 1.0 && s.z.abs() < 1.0) {
                let _ = entities::navigate_to(h, 10.0, 64.0, 10.0, 0.5);
            }
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
        if held_item == "test_machine:wrench" || held_item == "minecraft:iron_ingot" {
            let _ = entities::set_health(handle, 20.0);
            return true;
        }
        false
    }
    fn on_entity_hurt(&mut self, _handle: u32, _amount: f32, source: &str) -> bool {
        source != "minecraft:cactus" && source != "cactus"
    }
    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        if block == "test_machine:smelter" {
            let progress = saved.try_into().map_or(0, u32::from_le_bytes);
            self.progress.insert(handle, progress);
            let _ = item_transfer::set_slot_access(handle, 0, item_transfer::Face::Up, false, true);
            let _ =
                item_transfer::set_slot_access(handle, 0, item_transfer::Face::Down, true, false);
            let _ =
                item_transfer::set_slot_access(handle, 0, item_transfer::Face::North, true, false);
            let _ = item_transfer::set_slot_access(handle, 1, item_transfer::Face::Up, false, true);
        }
    }
    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &h in handles {
            if let Some(position) = machine::get_position(h) {
                let key = format!(
                    "{}:{},{},{}",
                    position.dimension, position.x, position.y, position.z
                );
                if capabilities::get_energy(&key).is_none() {
                    capabilities::set_energy_buffer(
                        &key,
                        EnergyView {
                            stored: 0,
                            capacity: 10_000,
                            max_receive: 25,
                            max_extract: 25,
                        },
                    );
                }
                capabilities::receive_energy(&key, 5, false);
                let peer_key = format!(
                    "{}:{},{},{}",
                    position.dimension,
                    position.x + 1,
                    position.y,
                    position.z
                );
                if capabilities::get_energy(&peer_key).is_some() {
                    for _ in 0..2 {
                        let accepted = capabilities::receive_energy(&peer_key, 1, true);
                        let extracted = capabilities::extract_energy(&key, accepted, false);
                        let received = capabilities::receive_energy(&peer_key, extracted, false);
                        if received < extracted {
                            capabilities::receive_energy(&key, extracted - received, false);
                        }
                    }
                }
                if capabilities::get_fluid(&key, 0).is_none() {
                    capabilities::set_fluid_tank(
                        &key,
                        0,
                        &FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 1_000,
                        },
                    );
                }
                capabilities::fill_fluid_tank(&key, 0, "minecraft:water", 10, false);
                if capabilities::get_fluid(&peer_key, 0).is_some() {
                    for _ in 0..2 {
                        let accepted = capabilities::fill_fluid_tank(
                            &peer_key,
                            0,
                            "minecraft:water",
                            10,
                            true,
                        );
                        let drained = capabilities::drain_fluid_tank(
                            &key,
                            0,
                            "minecraft:water",
                            accepted,
                            false,
                        );
                        let received = capabilities::fill_fluid_tank(
                            &peer_key,
                            0,
                            "minecraft:water",
                            drained,
                            false,
                        );
                        if received < drained {
                            capabilities::fill_fluid_tank(
                                &key,
                                0,
                                "minecraft:water",
                                drained - received,
                                false,
                            );
                        }
                    }
                }
            }
            let components = item_data::get_slot_components(h, 0);
            if !components.is_empty() {
                let _ = item_data::set_slot_components(h, 0, &components);
            }
            let progress = self.progress.entry(h).or_default();
            if machine::get_slot(h, 0).is_some_and(|s| s.id == "minecraft:tnt") {
                machine::set_property(h, "lit", "true");
                panic!("tnt in the smelter");
            }
            if let Some(input) = machine::get_slot(h, 0)
                && matches!(
                    input.id.as_str(),
                    "minecraft:milk_bucket" | "minecraft:honey_bottle"
                )
            {
                let Some(remainder) = machine::remainder(&input.id) else {
                    continue;
                };
                if !machine::insert_slot(
                    h,
                    1,
                    &ItemStack {
                        id: remainder,
                        count: 1,
                    },
                ) {
                    continue;
                }
                let rest = (input.count > 1).then(|| ItemStack {
                    count: input.count - 1,
                    ..input
                });
                machine::set_slot(h, 0, rest.as_ref());
                continue;
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
            // A full or incompatible output slot must not consume the input.
            // `insert_slot` is all-or-nothing, so retries cannot duplicate a
            // partial result on the next tick.
            if !machine::insert_slot(h, 1, &recipe.output) {
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

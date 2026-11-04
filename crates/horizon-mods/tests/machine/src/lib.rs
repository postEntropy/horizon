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
    entities, inventory, item_data, item_transfer,
    keys::{self, KeyBinding},
    machine::{self, CookingKind, ItemMoveKind, ItemStack},
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
    shared_crates: Vec<u32>,
    item_move_requests: HashMap<u32, Vec<u64>>,
    item_pickup_requests: HashMap<u32, Vec<u64>>,
    /// Each custom entity's age in ticks.
    entity_ticks: HashMap<u32, u32>,
    last_packet: String,
    last_server_event: String,
    last_block_event: String,
    last_item_event: String,
    spyglass_air_fallback: bool,
    spyglass_finished: bool,
    server_events: Vec<String>,
    clicked_slot: Option<(u32, u8)>,
    clicked_button: Option<u8>,
}

impl Mod for Machines {
    fn init() -> Result<(Self, Registration), String> {
        let spec = EnergyView {
            stored: 0,
            capacity: 2500,
            max_receive: 800,
            max_extract: 300,
        };
        // A namespace-mismatch fixture reuses this WASM under another mod id.
        let _ = item_data::register_energy_item("test_machine:battery", spec);
        let dynamic_spec = EnergyView {
            capacity: 2500,
            ..spec
        };
        // A test below reuses this WASM with a different owner id to verify
        // manifest block ownership; that fixture must reach manifest checks
        // instead of panicking on this deliberately foreign registration.
        let _ = item_data::register_energy_item_capacity_component(
            "test_machine:capacity_battery",
            dynamic_spec,
            "test_machine:capacity".into(),
        );
        assert!(!item_data::register_energy_item_capacity_component(
            "test_machine:invalid_capacity_path",
            dynamic_spec,
            "test_peer:capacity".into(),
        ));
        assert!(!item_data::register_energy_item("minecraft:stick", spec));
        assert!(!item_data::register_energy_item("test_peer:battery", spec));
        assert!(!item_data::register_energy_item(
            "test_machine:invalid",
            EnergyView { stored: 1, ..spec }
        ));
        assert!(!item_data::register_energy_item(
            "test_machine:invalid",
            EnergyView {
                capacity: 0,
                ..spec
            }
        ));
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
            coordinate_scale: 2.0,
            has_skylight: true,
            has_ceiling: false,
            min_y: -64,
            height: 384,
            ambient_light: 0.1,
            fixed_time: Some(12000),
            generator: GeneratorKind::Flat,
            surface_block: "minecraft:moss_block".into(),
        });
        let _ = dimensions::register_dimension(&DimensionSpec {
            id: "test_machine:noisy".into(),
            ultrawarm: false,
            natural: true,
            coordinate_scale: 1.0,
            has_skylight: true,
            has_ceiling: false,
            min_y: -64,
            height: 384,
            ambient_light: 0.0,
            fixed_time: None,
            generator: GeneratorKind::Noise,
            surface_block: String::new(),
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
                shared_crates: Vec::new(),
                item_move_requests: HashMap::new(),
                item_pickup_requests: HashMap::new(),
                entity_ticks: HashMap::new(),
                last_packet: String::new(),
                last_server_event: String::new(),
                last_block_event: String::new(),
                last_item_event: String::new(),
                spyglass_air_fallback: false,
                spyglass_finished: false,
                server_events: Vec::new(),
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
            BlockEntityType {
                block: "test_machine:shared_router".into(),
                slots: 4,
                ticks: true,
            },
        ]
    }
    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
            .block("test_machine:smelter")
            .block("minecraft:composter")
            .item("test_machine:wrench")
            .item("minecraft:spyglass")
            .server_event(ServerEventKind::PlayerJoin)
            .server_event(ServerEventKind::PlayerLeave)
            .server_event(ServerEventKind::BlockBreak)
            .server_event(ServerEventKind::BlockPlace)
            .server_event(ServerEventKind::EntitySpawn)
            .server_event(ServerEventKind::EntityDeath)
            .channel("test_machine:sync")
    }
    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::EntityInside {
            self.last_block_event = format!(
                "entity-inside:{}:{},{},{}:{}",
                event.state, event.x, event.y, event.z, event.actor
            );
            return EventOutcome::Pass;
        }
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
        let (consume_first, consume_second) =
            if event.action == ItemAction::UseOnBlock && event.item == "test_machine:wrench" {
                let pos = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                (
                    server::replace_block_and_consume_active_item(
                        &event.dimension,
                        pos,
                        "test_machine:smelter",
                        "test_machine:smelter[lit=true]",
                    ),
                    server::replace_block_and_consume_active_item(
                        &event.dimension,
                        pos,
                        "test_machine:smelter",
                        "test_machine:smelter[lit=true]",
                    ),
                )
            } else {
                (false, false)
            };
        let action_name = match event.action {
            ItemAction::UseInAir => "use-in-air",
            ItemAction::UseOnBlock => "use-on-block",
            ItemAction::FinishUsing => "finish-using",
            ItemAction::AttackEntity => "attack-entity",
            ItemAction::InteractEntity => "interact-entity",
        };
        self.last_item_event = format!(
            "{action_name}:{}:{}:{},{},{}:{}:consume={consume_first}/{consume_second}",
            event.item, event.dimension, event.x, event.y, event.z, event.target
        );
        if event.item == "minecraft:spyglass" {
            if event.action == ItemAction::UseInAir {
                self.spyglass_air_fallback = true;
            } else if event.action == ItemAction::FinishUsing {
                self.spyglass_finished = true;
            }
            return EventOutcome::Pass;
        }
        if event.action == ItemAction::AttackEntity
            && event.item == "test_machine:wrench"
            && event.target == "test_machine:automaton"
        {
            return EventOutcome::Cancel;
        }
        if event.action == ItemAction::UseOnBlock && event.item != "minecraft:bone_meal" {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
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
        if self.server_events.len() >= 64 {
            self.server_events.remove(0);
        }
        self.server_events
            .push(format!("{kind}:{}:{}", event.subject, event.detail));
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
            arg if arg.starts_with("player_access ") => {
                let words: Vec<_> = arg.split_whitespace().collect();
                if words.len() != 5 {
                    return Err("player_access HANDLE SLOT INSERT EXTRACT".into());
                }
                let handle = words[1].parse::<u32>().map_err(|e| e.to_string())?;
                let slot = words[2].parse::<u32>().map_err(|e| e.to_string())?;
                let insert = words[3].parse::<bool>().map_err(|e| e.to_string())?;
                let extract = words[4].parse::<bool>().map_err(|e| e.to_string())?;
                Ok(Some(
                    machine::set_player_slot_access(handle, slot, insert, extract).to_string(),
                ))
            }
            arg if arg.starts_with("adjacent ") => {
                let handle = arg
                    .split_whitespace()
                    .nth(1)
                    .ok_or_else(|| "adjacent HANDLE".to_owned())?
                    .parse::<u32>()
                    .map_err(|e| e.to_string())?;
                machine::set_world_rule("test_machine:adjacent_target", &handle.to_string())?;
                Ok(None)
            }
            "adjacent_result" => Ok(Some(
                machine::get_world_rule("test_machine:adjacent_result").unwrap_or_default(),
            )),

            "atomic_items" => {
                machine::set_world_rule("test_machine:atomic_items", "true")?;
                Ok(None)
            }
            "block_tags" => Ok(Some(format!(
                "heat={} cold={} unknown={}",
                machine::block_in_tag("minecraft:magma_block", "farmersdelight:heat_sources"),
                machine::block_in_tag("minecraft:stone", "farmersdelight:heat_sources"),
                machine::block_in_tag("unknown:missing", "unknown:missing")
            ))),
            arg if arg.starts_with("item_slots ") => {
                let slots = capabilities::get_item_slots(arg.trim_start_matches("item_slots "));
                Ok(Some(
                    slots
                        .iter()
                        .map(|s| format!("{}={}*{}", s.slot, s.id, s.count))
                        .collect::<Vec<_>>()
                        .join(","),
                ))
            }
            "move_pull" | "move_push" | "move_push_remove" | "move_trap" => {
                machine::set_world_rule("test_machine:item_move_operation", args)?;
                for &handle in self.progress.keys() {
                    assert!(
                        machine::request_item_move(
                            handle,
                            0,
                            item_transfer::Face::East,
                            ItemMoveKind::Pull,
                            1
                        )
                        .is_none()
                    );
                }
                Ok(None)
            }
            "pickup_request" | "pickup_trap" | "pickup_stop" => {
                machine::set_world_rule("test_machine:item_pickup_operation", args)?;
                Ok(None)
            }
            "item_energy_receive" | "item_energy_extract" => {
                machine::set_world_rule("test_machine:item_energy_operation", args)?;
                for &handle in self.progress.keys() {
                    assert!(item_data::get_slot_energy(handle, 0).is_none());
                    assert_eq!(item_data::receive_slot_energy(handle, 0, 10, false), 0);
                }
                Ok(None)
            }
            "item_energy_dynamic_capacity" => {
                machine::set_world_rule("test_machine:item_energy_operation", args)?;
                for &handle in self.progress.keys() {
                    assert!(item_data::get_slot_energy(handle, 0).is_none());
                    assert_eq!(item_data::receive_slot_energy(handle, 0, 10, false), 0);
                }
                Ok(None)
            }
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
            "read_config" => {
                let speed = config::get_int("speed").unwrap_or(0);
                let enabled = config::get_bool("enabled").unwrap_or(false);
                let scale = config::get_float("scale").unwrap_or(0.0);
                let mode = config::get_string("mode").unwrap_or_default();
                Ok(Some(format!(
                    "speed={speed} bool={enabled} float={scale} mode={mode}"
                )))
            }
            "slot_cap_bridge" => {
                let pos = "minecraft:overworld:0,64,0";
                capabilities::set_item_slot(pos, 0, "minecraft:raw_iron", 2);
                capabilities::set_item_slot(pos, 9, "minecraft:diamond", 1);
                capabilities::set_item_slot(pos, 1, "minecraft:iron_ingot", 99);
                let summary = capabilities::get_item_slots(pos)
                    .into_iter()
                    .map(|s| format!("{}={}*{}", s.slot, s.id, s.count))
                    .collect::<Vec<_>>()
                    .join(",");
                Ok(Some(format!("slots={summary}")))
            }
            "slot_cap_bridge_e2e" => {
                let pos = "minecraft:overworld:-31,70,-9";
                capabilities::set_item_slot(pos, 0, "minecraft:raw_iron", 3);
                capabilities::set_item_slot(pos, 9, "minecraft:diamond", 1);
                let slots = capabilities::get_item_slots(pos);
                let slot0 = slots
                    .iter()
                    .find(|s| s.slot == 0)
                    .map(|s| format!("{}*{}", s.id, s.count))
                    .unwrap_or_else(|| "none".into());
                Ok(Some(format!("slot0={slot0} len={}", slots.len())))
            }
            "read_slot_caps" => {
                let pos = "minecraft:overworld:0,64,0";
                let summary = capabilities::get_item_slots(pos)
                    .into_iter()
                    .map(|s| format!("{}={}*{}", s.slot, s.id, s.count))
                    .collect::<Vec<_>>()
                    .join(",");
                Ok(Some(format!("slots={summary}")))
            }
            "get_ring" => {
                let ring = capabilities::get_accessory("player", "ring", 0)
                    .map(|s| format!("{}*{}", s.id, s.count))
                    .unwrap_or_else(|| "none".into());
                let over_stack_rejected =
                    capabilities::set_accessory("player", "ring", 1, "test_machine:ring", 99)
                        .is_err();
                let bad_index_rejected =
                    capabilities::set_accessory("player", "ring", 32, "test_machine:ring", 1)
                        .is_err();
                Ok(Some(format!(
                    "ring={ring} reject_stack={over_stack_rejected} reject_idx={bad_index_rejected}"
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
                let before_revision = recipes::revision();
                recipes::register(&recipe)?;
                assert_ne!(recipes::revision(), before_revision);
                let by_id = recipes::get("test_machine:crush_iron").is_some_and(|entry| {
                    entry.kind == "test_machine:crusher" && entry.outputs == recipe.outputs
                });
                let found = recipes::by_type("test_machine:crusher").len();
                let found_by_item = recipes::for_input("minecraft:raw_iron").len();
                let found_by_output = recipes::for_output("minecraft:iron_ingot").len();
                let mut replacement = recipe;
                replacement.outputs = vec!["minecraft:gold_ingot*1".into()];
                let before_revision = recipes::revision();
                recipes::register(&replacement)?;
                assert_ne!(recipes::revision(), before_revision);
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
            "dims" => Ok(Some(format!(
                "dims={}",
                dimensions::list_dimensions().join(",")
            ))),
            "tp_twilight" => {
                dimensions::teleport("@s", "test_machine:twilight", 8.5, 64.0, 8.5)?;
                Ok(Some("tp_twilight=ok".into()))
            }
            "tp_noisy" => {
                dimensions::teleport("@s", "test_machine:noisy", 0.5, 120.0, 0.5)?;
                Ok(Some("tp_noisy=ok".into()))
            }
            "tp_overworld" => {
                dimensions::teleport("@s", "minecraft:overworld", -31.5, 70.0, -11.5)?;
                Ok(Some("tp_overworld=ok".into()))
            }
            "set_models" => {
                let entity_target = self
                    .entity_ticks
                    .keys()
                    .find(|&&h| entities::get_state(h).is_some())
                    .copied();
                if let Some(h) = entity_target {
                    let target = format!("entity:{h}");
                    models::set_animation(&target, "spin", true, 2.0);
                    models::set_bone_poses(
                        &target,
                        &[BonePose {
                            bone: "head".into(),
                            rot_x: 0.25,
                            rot_y: 0.5,
                            rot_z: 0.0,
                            pos_x: 0.0,
                            pos_y: 2.0,
                            pos_z: 0.0,
                            scale_x: 1.2,
                            scale_y: 1.2,
                            scale_z: 1.2,
                        }],
                    );
                    models::set_draw_list(
                        &target,
                        &[DrawPart {
                            kind: DrawPartKind::FloatingText,
                            asset: "Automaton Active".into(),
                            x: 0.0,
                            y: 2.4,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 0.0,
                            rot_z: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            scale_z: 1.0,
                            color: 0xFFFF_FFFF,
                        }],
                    );
                }
                let block_target = "block:minecraft:overworld:-31,70,-9";
                models::set_animation(block_target, "smelt_spin", true, f32::NAN);
                models::set_bone_poses(
                    block_target,
                    &[
                        BonePose {
                            bone: "rotor".into(),
                            rot_x: 0.0,
                            rot_y: 45.0,
                            rot_z: 0.0,
                            pos_x: f32::NAN,
                            pos_y: 1.0,
                            pos_z: 0.0,
                            scale_x: 0.0,
                            scale_y: 1.5,
                            scale_z: f32::INFINITY,
                        },
                        BonePose {
                            bone: String::new(),
                            rot_x: 0.0,
                            rot_y: 0.0,
                            rot_z: 0.0,
                            pos_x: 0.0,
                            pos_y: 0.0,
                            pos_z: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            scale_z: 1.0,
                        },
                    ],
                );
                models::set_draw_list(
                    block_target,
                    &[
                        DrawPart {
                            kind: DrawPartKind::ItemDisplay,
                            asset: "minecraft:diamond".into(),
                            x: 0.0,
                            y: 1.1,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 45.0,
                            rot_z: 0.0,
                            scale_x: 0.5,
                            scale_y: 0.5,
                            scale_z: 0.5,
                            color: 0xFFFF_FFFF,
                        },
                        DrawPart {
                            kind: DrawPartKind::FluidCuboid,
                            asset: "minecraft:water".into(),
                            x: 0.0,
                            y: 0.5,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 0.0,
                            rot_z: 0.0,
                            scale_x: 0.8,
                            scale_y: 0.6,
                            scale_z: 0.8,
                            color: 0x8033_99FF,
                        },
                        DrawPart {
                            kind: DrawPartKind::ModelPart,
                            asset: "minecraft:iron_block".into(),
                            x: 0.0,
                            y: 0.8,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 30.0,
                            rot_z: 0.0,
                            scale_x: 0.4,
                            scale_y: 0.4,
                            scale_z: 0.4,
                            color: 0xFFFF_FFFF,
                        },
                        DrawPart {
                            kind: DrawPartKind::FloatingText,
                            asset: "Smelter Ready".into(),
                            x: 0.0,
                            y: 1.6,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 0.0,
                            rot_z: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            scale_z: 1.0,
                            color: 0xFFFF_FFFF,
                        },
                        DrawPart {
                            kind: DrawPartKind::ItemDisplay,
                            asset: String::new(),
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                            rot_x: 0.0,
                            rot_y: 0.0,
                            rot_z: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            scale_z: 1.0,
                            color: 0xFFFF_FFFF,
                        },
                    ],
                );
                Ok(Some(format!("models_set entity={entity_target:?}")))
            }
            "clear_models" => {
                let block_target = "block:minecraft:overworld:-31,70,-9";
                models::set_animation(block_target, "", false, 1.0);
                models::set_bone_poses(block_target, &[]);
                models::set_draw_list(block_target, &[]);
                Ok(Some("models_cleared=true".into()))
            }
            "clear_logic_events" => {
                self.last_block_event.clear();
                self.last_item_event.clear();
                self.spyglass_air_fallback = false;
                self.spyglass_finished = false;
                self.server_events.clear();
                Ok(Some("logic_events_cleared=true".into()))
            }
            "logic_events" => Ok(Some(format!(
                "block={} item={} air_fallback={} finished={} server=[{}]",
                self.last_block_event,
                self.last_item_event,
                self.spyglass_air_fallback,
                self.spyglass_finished,
                self.server_events.join(";")
            ))),
            "api_0_3_reads" => {
                let block = target::get_block().map_or_else(
                    || "none".into(),
                    |b| format!("{}@{},{},{}:{}:{}", b.state, b.x, b.y, b.z, b.face, b.owner),
                );
                let entity = target::get_entity().map_or_else(
                    || "none".into(),
                    |e| format!("{}:{:.1}/{:.1}:{}", e.kind, e.health, e.max_health, e.name),
                );
                let main = inventory::get_main_hand().map_or_else(
                    || "none".into(),
                    |s| {
                        format!(
                            "{}:{}*{}({}/{})",
                            s.slot, s.id, s.count, s.damage, s.max_damage
                        )
                    },
                );
                let off = inventory::get_off_hand().map_or_else(
                    || "none".into(),
                    |s| {
                        format!(
                            "{}:{}*{}({}/{})",
                            s.slot, s.id, s.count, s.damage, s.max_damage
                        )
                    },
                );
                let inv = inventory::get_player_inventory();
                let armor = inv
                    .iter()
                    .filter(|s| (36..40).contains(&s.slot))
                    .map(|s| format!("{}:{}*{}", s.slot, s.id, s.count))
                    .collect::<Vec<_>>()
                    .join(",");
                let open = inventory::get_open_container()
                    .iter()
                    .map(|s| format!("{}:{}*{}", s.slot, s.id, s.count))
                    .collect::<Vec<_>>()
                    .join(",");
                Ok(Some(format!(
                    "block={block} entity={entity} main={main} off={off} inv_len={} armor=[{armor}] open=[{open}]",
                    inv.len()
                )))
            }
            sub if sub.starts_with("api_0_3_world") => {
                let parts: Vec<&str> = sub.split_whitespace().collect();
                if parts.get(1).copied() == Some("probe") && parts.len() == 5 {
                    let x = parts[2].parse::<i32>().unwrap_or(0);
                    let y = parts[3].parse::<i32>().unwrap_or(0);
                    let z = parts[4].parse::<i32>().unwrap_or(0);
                    let b = world::get_block(x, y, z).unwrap_or_else(|| "none".into());
                    let bio = world::get_biome(x, y, z).unwrap_or_else(|| "none".into());
                    let l =
                        world::get_light(x, y, z).map_or_else(|| "none".into(), |v| v.to_string());
                    return Ok(Some(format!(
                        "probe={x},{y},{z} block={b} biome={bio} light={l}"
                    )));
                }
                let (px, py, pz) = if parts.len() == 4 {
                    (
                        parts[1].parse::<i32>().unwrap_or(0),
                        parts[2].parse::<i32>().unwrap_or(70),
                        parts[3].parse::<i32>().unwrap_or(0),
                    )
                } else {
                    (0, 70, 0)
                };
                let below = world::get_block(px, py - 1, pz).unwrap_or_else(|| "none".into());
                let biome = world::get_biome(px, py, pz).unwrap_or_else(|| "none".into());
                let light =
                    world::get_light(px, py, pz).map_or_else(|| "none".into(), |v| v.to_string());
                let far_block = world::get_block(px + 10, py, pz).unwrap_or_else(|| "none".into());
                let far_biome = world::get_biome(px + 10, py, pz).unwrap_or_else(|| "none".into());
                let low_block = world::get_block(px, -65, pz).unwrap_or_else(|| "none".into());
                let high_block = world::get_block(px, 320, pz).unwrap_or_else(|| "none".into());
                Ok(Some(format!(
                    "at={px},{py},{pz} below={below} biome={biome} light={light} far_block={far_block} far_biome={far_biome} low={low_block} high={high_block}"
                )))
            }
            sub if sub.starts_with("api_0_3_server") => {
                let parts: Vec<&str> = sub.split_whitespace().collect();
                match parts.get(1).copied().unwrap_or("rules") {
                    "rules" => {
                        let keep_snake =
                            server::get_rule("keep_inventory").unwrap_or_else(|| "none".into());
                        let keep_camel =
                            server::get_rule("keepInventory").unwrap_or_else(|| "none".into());
                        let tick_speed =
                            server::get_rule("random_tick_speed").unwrap_or_else(|| "none".into());
                        Ok(Some(format!(
                            "keep_inventory={keep_snake} keepInventory={keep_camel} random_tick_speed={tick_speed}"
                        )))
                    }
                    "set_rule" if parts.len() >= 4 => {
                        let res = server::set_rule(parts[2], parts[3]).is_ok();
                        Ok(Some(format!("set_rule={res}")))
                    }
                    "invalid_block" if parts.len() == 6 => {
                        let dim = parts[2];
                        let pos = BlockPos {
                            x: parts[3].parse().unwrap_or(0),
                            y: parts[4].parse().unwrap_or(70),
                            z: parts[5].parse().unwrap_or(0),
                        };
                        let res = server::set_block(dim, pos, "minecraft:not_a_real_block[bad=1]")
                            .is_ok();
                        Ok(Some(format!("invalid_block_queued={res}")))
                    }
                    "get_block" if parts.len() == 6 => {
                        let dim = parts[2];
                        let pos = BlockPos {
                            x: parts[3].parse().unwrap_or(0),
                            y: parts[4].parse().unwrap_or(70),
                            z: parts[5].parse().unwrap_or(0),
                        };
                        let b = server::get_block(dim, pos).unwrap_or_else(|| "none".into());
                        Ok(Some(format!("server_block={b}")))
                    }
                    "sound_and_invalid" if parts.len() == 6 => {
                        let dim = parts[2];
                        let pos = BlockPos {
                            x: parts[3].parse().unwrap_or(0),
                            y: parts[4].parse().unwrap_or(70),
                            z: parts[5].parse().unwrap_or(0),
                        };
                        server::play_sound(dim, pos, "minecraft:block.note_block.pling", 1.0, 1.0);
                        let bad_syntax = server::spawn_mob(dim, pos, "bad_id").is_err();
                        let unknown_mob =
                            server::spawn_mob(dim, pos, "minecraft:not_a_mob").is_ok();
                        let unknown_dim =
                            server::set_block("test_machine:unknown_dim", pos, "minecraft:stone")
                                .is_ok();
                        Ok(Some(format!(
                            "sound=true bad_mob_rejected={bad_syntax} unknown_mob_queued={unknown_mob} unknown_dim_queued={unknown_dim}"
                        )))
                    }
                    _ => Err("unknown api_0_3_server subcommand".into()),
                }
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
            if *age == 1 {
                models::set_animation(&format!("entity:{h}"), "walk", true, 1.0);
            }
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
        if block == "test_machine:crate" || block == "test_machine:shared_router" {
            assert!(!machine::bind_shared_inventory(
                handle.wrapping_add(1),
                "public"
            ));
            assert!(!machine::bind_shared_inventory(handle, ""));
            assert!(!machine::bind_shared_inventory(handle, &"x".repeat(257)));
            assert!(machine::bind_shared_inventory(handle, "public"));
            self.shared_crates.push(handle);
        }
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
        // Previously seen handles outside this loan must not resolve through an old index.
        for &handle in self.progress.keys().filter(|h| !handles.contains(h)) {
            assert!(machine::get_slot(handle, 0).is_none());
            assert!(!machine::set_slot(handle, 0, None));
        }
        // Handles are allocated globally, but neighboring owners' slots stay private.
        for &handle in handles {
            for neighbor in [handle.wrapping_sub(1), handle.wrapping_add(1)] {
                if !handles.contains(&neighbor) {
                    assert!(machine::get_slot(neighbor, 0).is_none());
                    assert!(!machine::set_slot(neighbor, 0, None));
                }
            }
        }
        for &h in handles {
            let target_handle = h.to_string();
            if machine::get_world_rule("test_machine:adjacent_target").as_deref()
                == Some(target_handle.as_str())
            {
                let neighbors = machine::get_adjacent_block_entities(h)
                    .into_iter()
                    .map(|neighbor| {
                        let face = match neighbor.face {
                            machine::Face::Down => "down",
                            machine::Face::Up => "up",
                            machine::Face::North => "north",
                            machine::Face::South => "south",
                            machine::Face::West => "west",
                            machine::Face::East => "east",
                        };
                        format!("{face}={}", neighbor.handle)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let _ = machine::set_world_rule("test_machine:adjacent_result", &neighbors);
            }
            if machine::get_world_rule("test_machine:atomic_items").as_deref() == Some("true") {
                let input = machine::SlotStack {
                    slot: 0,
                    stack: ItemStack {
                        id: "minecraft:raw_iron".into(),
                        count: 2,
                    },
                };
                let outputs = [
                    machine::SlotStack {
                        slot: 1,
                        stack: ItemStack {
                            id: "minecraft:iron_ingot".into(),
                            count: 1,
                        },
                    },
                    machine::SlotStack {
                        slot: 2,
                        stack: ItemStack {
                            id: "minecraft:gold_ingot".into(),
                            count: 1,
                        },
                    },
                ];
                let before = (0..4)
                    .map(|slot| machine::get_slot(h, slot).map(|s| (s.id, s.count)))
                    .collect::<Vec<_>>();
                let possible = machine::process_items(h, &[input.clone()], &outputs, true);
                assert_eq!(
                    (0..4)
                        .map(|slot| machine::get_slot(h, slot).map(|s| (s.id, s.count)))
                        .collect::<Vec<_>>(),
                    before
                );
                assert!(!machine::process_items(
                    h,
                    &[input.clone(), input.clone()],
                    &outputs,
                    false
                ));
                assert_eq!(
                    machine::process_items(h, &[input], &outputs, false),
                    possible
                );
                continue;
            }
            if let Some(operation) = machine::get_world_rule("test_machine:item_energy_operation") {
                let before = item_data::get_slot_components(h, 0);
                if operation == "item_energy_dynamic_capacity" {
                    let energy =
                        item_data::get_slot_energy(h, 0).expect("dynamic battery capability");
                    assert_eq!(energy.capacity, 5000);
                    assert_eq!(energy.stored, 4200);
                    assert_eq!(item_data::receive_slot_energy(h, 0, 2000, true), 800);
                    assert_eq!(item_data::get_slot_components(h, 0), before);
                    assert_eq!(item_data::receive_slot_energy(h, 0, 2000, false), 800);
                    assert_eq!(item_data::get_slot_energy(h, 0).unwrap().stored, 5000);
                    let charged = item_data::get_slot_components(h, 0);
                    assert!(item_data::set_slot_energy_capacity(h, 0, 6000));
                    assert_eq!(item_data::get_slot_energy(h, 0).unwrap().capacity, 6000);
                    assert_eq!(item_data::get_slot_energy(h, 0).unwrap().stored, 5000);
                    let expanded = item_data::get_slot_components(h, 0);
                    assert_ne!(expanded, charged);
                    assert!(!item_data::set_slot_energy_capacity(h, 0, 4500));
                    assert_eq!(item_data::get_slot_components(h, 0), expanded);
                } else if operation == "item_energy_extract" {
                    let expected = item_data::extract_slot_energy(h, 0, 1000, true);
                    assert_eq!(item_data::get_slot_components(h, 0), before);
                    assert_eq!(item_data::extract_slot_energy(h, 0, 1000, false), expected);
                } else {
                    let expected = item_data::receive_slot_energy(h, 0, 1000, true);
                    assert_eq!(item_data::get_slot_components(h, 0), before);
                    assert_eq!(item_data::receive_slot_energy(h, 0, 1000, false), expected);
                }
                continue;
            }
            if let Some(operation) = machine::get_world_rule("test_machine:item_pickup_operation") {
                let requests = self.item_pickup_requests.entry(h).or_default();
                let mut completed = false;
                requests.retain(|request| {
                    if let Some(moved) = machine::take_item_pickup_result(*request) {
                        *self.progress.entry(h).or_default() += moved;
                        assert!(machine::take_item_pickup_result(*request).is_none());
                        completed = true;
                        false
                    } else {
                        true
                    }
                });
                if operation == "pickup_request" || operation == "pickup_trap" {
                    for (handle, slot) in [(h.wrapping_add(100000), 0), (h, u32::MAX)] {
                        assert!(machine::request_item_pickup(handle, slot).is_none());
                    }
                    if !completed && requests.is_empty() {
                        let before = machine::get_slot(h, 0).map(|s| (s.id, s.count));
                        let request = machine::request_item_pickup(h, 0).unwrap();
                        assert_eq!(machine::get_slot(h, 0).map(|s| (s.id, s.count)), before);
                        requests.push(request);
                        if operation == "pickup_trap" {
                            panic!("pickup intent before trap");
                        }
                    }
                }
                continue;
            }
            if let Some(operation) = machine::get_world_rule("test_machine:item_move_operation") {
                let requests = self.item_move_requests.entry(h).or_default();
                requests.retain(|request| {
                    if let Some(moved) = machine::take_item_move_result(*request) {
                        *self.progress.entry(h).or_default() += moved;
                        assert!(machine::take_item_move_result(*request).is_none());
                        false
                    } else {
                        true
                    }
                });
                for (handle, slot, count) in [
                    (h.wrapping_add(100000), 0, 1),
                    (h, u32::MAX, 1),
                    (h, 0, 0),
                    (h, 0, 65),
                ] {
                    assert!(
                        machine::request_item_move(
                            handle,
                            slot,
                            item_transfer::Face::East,
                            ItemMoveKind::Pull,
                            count
                        )
                        .is_none()
                    );
                }
                let before = machine::get_slot(h, 0).map(|s| (s.id, s.count));
                let kind = if operation == "move_push" || operation == "move_push_remove" {
                    ItemMoveKind::Push
                } else {
                    ItemMoveKind::Pull
                };
                if operation == "move_push_remove"
                    && let Some(position) = machine::get_position(h)
                {
                    server::set_block(
                        &position.dimension,
                        BlockPos {
                            x: position.x + 1,
                            y: position.y,
                            z: position.z,
                        },
                        "minecraft:air",
                    )
                    .unwrap();
                }
                let request =
                    machine::request_item_move(h, 0, item_transfer::Face::East, kind, 1).unwrap();
                assert_eq!(machine::get_slot(h, 0).map(|s| (s.id, s.count)), before);
                requests.push(request);
                if operation == "move_trap" {
                    panic!("intent before trap");
                }
                continue;
            }
            if self.shared_crates.contains(&h) {
                if let Some(mut input) = machine::get_slot(h, 0) {
                    input.count -= 1;
                    let rest = (input.count > 0).then_some(input);
                    assert!(machine::set_slot(h, 0, rest.as_ref()));
                    let produced = machine::get_slot(h, 1).map_or(0, |stack| stack.count);
                    assert!(machine::set_slot(
                        h,
                        1,
                        Some(&ItemStack {
                            id: "minecraft:iron_ingot".into(),
                            count: produced + 1
                        })
                    ));
                }
                continue;
            }
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
            if machine::get_slot(h, 0).is_some_and(|s| s.id == "minecraft:gunpowder") {
                machine::set_property(h, "lit", "true");
                loop {
                    std::hint::spin_loop();
                }
            }
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
            // A full or incompatible output slot must not consume the input.
            // `insert_slot` is all-or-nothing, so retries cannot duplicate a
            // partial result on the next tick.
            if !machine::insert_slot(h, 1, &recipe.output) {
                *progress = recipe.cooking_time;
                continue;
            }
            *progress = 0;
            let input = machine::get_slot(h, 0).filter(|i| i.count > 1);
            let rest = input.map(|i| ItemStack {
                count: i.count - 1,
                ..i
            });
            machine::set_slot(h, 0, rest.as_ref());
        }
    }
    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        assert!(machine::get_slot(handle, 0).is_none());
        assert!(!machine::set_slot(handle, 0, None));
        let progress = self.progress.get(&handle).copied().unwrap_or(0);
        progress.to_le_bytes().to_vec()
    }
    fn remove_block_entity(&mut self, handle: u32) {
        self.item_move_requests.remove(&handle);
        self.item_pickup_requests.remove(&handle);
        self.progress.remove(&handle);
        self.shared_crates.retain(|h| *h != handle);
    }
    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        vec![self.progress.get(&handle).copied().unwrap_or(0) as i32]
    }
}

horizon_mod_sdk::export_mod!(Machines);

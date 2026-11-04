//! Loads real mods built with the SDK: the coords-hud example and a guest
//! that misbehaves on command. Needs `rustup target add wasm32-unknown-unknown`.
use horizon_mods::{
    BlockAction, BlockEntityView, BlockEvent, BlockHit, Clock, CookingKind, CookingRecipe, Corner,
    EventOutcome, IngredientSlot, ItemAction, ItemDrop, ItemEvent, ItemFacts, Mods, Output,
    Package, Player, Property, RecipeEntry, ServerAction, ServerEvent, ServerEventKind, Snapshot,
    Stack, StackView, Status, VanillaHookEvent, VanillaHookKind, WorldCell, manifest::componentize,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Instant,
};

const GUEST_MANIFEST: &str = r#"
id = "test_guest"
name = "Test Guest"
version = "0.1.0"
api = "0.1"
capabilities = ["game-read", "chat", "storage"]
"#;

const MACHINE_MANIFEST: &str = r#"
id = "test_machine"
name = "Test Machine"
version = "0.1.0"
api = "0.5"
side = "both"
capabilities = ["config", "server", "keys", "target", "inventory-read", "capabilities", "ui", "net", "recipes", "world-read", "render", "entities", "models", "dimensions", "vanilla-hooks", "block-entities", "item-data", "item-transfer", "registries"]

[config.speed]
type = "int"
default = 2
min = 1
max = 10

[config.enabled]
type = "bool"
default = true

[config.scale]
type = "float"
default = 1.0
min = 0.5
max = 2.0

[config.mode]
type = "string"
default = "safe"
choices = ["safe", "fast"]
"#;

const MACHINE_PEER_MANIFEST: &str = r#"
id = "test_peer"
name = "Test Machine Peer"
version = "0.1.0"
api = "0.4"
side = "both"
capabilities = ["server", "capabilities", "block-entities", "registries", "config"]

[config.one_slot]
type = "bool"
default = false
"#;

const EFFECTS_MANIFEST: &str = r#"
id = "test_effects"
name = "Test Effects"
version = "0.1.0"
api = "0.6"
side = "both"
capabilities = ["status-effects", "registries"]
"#;

/// Builds the mods once and writes their `.hmod` files; returns the folder.
fn mods_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = root.join("target/mods-test");
        let status = Command::new(env!("CARGO"))
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
            .args([
                "-p",
                "coords-hud",
                "-p",
                "horizon-mods-test-guest",
                "-p",
                "horizon-mods-test-machine",
                "-p",
                "horizon-mods-test-machine-peer",
                "-p",
                "horizon-mods-test-effects",
                "--target-dir",
            ])
            .arg(&target)
            .status()
            .expect("cargo runs");
        assert!(
            status.success(),
            "building the test mods needs `rustup target add wasm32-unknown-unknown`"
        );
        let built = target.join("wasm32-unknown-unknown/release");
        let dir = target.join("mods");
        std::fs::create_dir_all(&dir).unwrap();
        let example = root.join("crates/horizon-mod-sdk/examples/coords-hud/horizon-mod.toml");
        let mods = [
            ("coords_hud", std::fs::read_to_string(example).unwrap()),
            ("horizon_mods_test_guest", GUEST_MANIFEST.to_string()),
            ("horizon_mods_test_machine", MACHINE_MANIFEST.to_string()),
            ("horizon_mods_test_effects", EFFECTS_MANIFEST.to_string()),
            (
                "horizon_mods_test_machine_peer",
                MACHINE_PEER_MANIFEST.to_string(),
            ),
        ];
        for (name, manifest) in mods {
            let core = std::fs::read(built.join(format!("{name}.wasm"))).unwrap();
            let component = componentize(&core).unwrap();
            let content_files: &[(&str, &[u8])] = match name {
                "horizon_mods_test_machine" => &[
                    (
                        "data/test_machine/dimension/twilight.json",
                        br#"{"type":"test_machine:twilight","generator":{"type":"minecraft:flat","settings":{"biome":"test_machine:crystal_grove","layers":[{"block":"minecraft:bedrock","height":1},{"block":"minecraft:gold_block","height":1},{"block":"minecraft:stone","height":125},{"block":"minecraft:moss_block","height":1}]}}}"#,
                    ),
                    (
                        "data/test_machine/dimension/noisy.json",
                        br#"{"type":"minecraft:overworld","generator":{"type":"minecraft:noise","settings":"minecraft:overworld","biome_source":{"type":"minecraft:fixed","biome":"test_machine:crystal_grove"}}}"#,
                    ),
                    (
                        "data/test_machine/dimension/data_only.json",
                        br#"{"type":"test_machine:data_only","generator":{"type":"minecraft:flat","settings":{"biome":"test_machine:crystal_grove","layers":[{"block":"minecraft:bedrock","height":1},{"block":"minecraft:gold_block","height":4}]}}}"#,
                    ),
                    (
                        "data/test_machine/dimension_type/data_only.json",
                        br#"{"min_y":0,"height":256,"logical_height":256,"coordinate_scale":1.0,"has_skylight":true,"has_ceiling":false,"ultrawarm":false,"fixed_time":18000}"#,
                    ),
                    (
                        "data/test_machine/worldgen/biome/crystal_grove.json",
                        br#"{"has_precipitation":false,"temperature":0.7,"downfall":0.0,"effects":{"water_color":4159204,"grass_color":6737151,"foliage_color":6737151},"attributes":{"minecraft:visual/sky_color":5945561},"carvers":[],"features":[[],[],[],[],[],[],[],[],[],[],[]]}"#,
                    ),
                    (
                        "assets/test_machine/horizon/entity_models/automaton.json",
                        br#"{"texture":"test_machine:entity/automaton","geo":"test_machine:entity/automaton"}"#,
                    ),
                    (
                        "assets/test_machine/geo/entity/automaton.geo.json",
                        br#"{"format_version":"1.12.0","minecraft:geometry":[{"description":{"identifier":"geometry.automaton","texture_width":16,"texture_height":16},"bones":[{"name":"head","pivot":[0,16,0],"cubes":[{"origin":[-4,16,-4],"size":[8,8,8],"uv":[0,0]}]}]}]}"#,
                    ),
                    (
                        "assets/test_machine/textures/entity/automaton.png",
                        include_bytes!("machine/automaton.png"),
                    ),
                    (
                        "data/test_machine/horizon/block/smelter.json",
                        br#"{"template":"minecraft:furnace","properties":{"hardness":3.5}}"#,
                    ),
                    (
                        "data/test_machine/horizon/item/wrench.json",
                        br#"{"template":"minecraft:stick"}"#,
                    ),
                ],
                "horizon_mods_test_machine_peer" => &[(
                    "data/test_peer/horizon/block/cell.json",
                    br#"{"template":"minecraft:furnace","properties":{"hardness":3.5}}"#,
                )],
                "horizon_mods_test_effects" => &[(
                    "data/test_effects/horizon/item/test_food.json",
                    br#"{"template":"minecraft:apple","food":{"nutrition":4,"saturation":2.4,"can_always_eat":false,"consume_seconds":0.8,"effects":[{"id":"test_effects:steady","duration":60,"amplifier":0,"probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}}"#,
                )],
                _ => &[],
            };
            let hmod = if !content_files.is_empty() {
                // The game-level machine fixture must carry its block and item
                // as real API 0.2 content definitions, so its API 0.4 block
                // entity and API 0.3 item hooks can be exercised in-game.
                Package::write_files(&manifest, Some(&component), content_files).unwrap()
            } else {
                Package::write(&manifest, &component).unwrap()
            };
            std::fs::write(dir.join(format!("{name}.hmod")), hmod).unwrap();
        }
        dir
    })
}

fn load(name: &str) -> Mods {
    Mods::load(&[mods_dir().join(name)], |_| true, |c| c == "give")
}

#[test]
fn registered_effects_are_owner_scoped_and_tick_authoritatively() {
    let mut mods = load("horizon_mods_test_effects.hmod");
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(mods.has_status_effect("test_effects:steady"));
    assert!(!mods.has_status_effect("foreign:steady"));
    mods.set_status_effect_context(
        "test-player-uuid".into(),
        vec![horizon_mods::ActiveEffect {
            id: "test_effects:steady".into(),
            duration: 10,
            amplifier: 0,
            ambient: false,
            visible: false,
            show_icon: false,
            food: 8,
            saturation: 2.0,
        }],
    );
    mods.tick(1);
    assert!(
        mods.drain().is_empty(),
        "player effects must use the per-player lifecycle"
    );
    mods.player_tick(1);
    assert_eq!(
        mods.drain(),
        [Output::SetPlayerFood {
            food: 9,
            saturation: 2.25,
        }]
    );
    mods.set_status_effect_context("test-player-uuid".into(), Vec::new());
    mods.player_tick(2);
    assert!(
        mods.drain().is_empty(),
        "an expired effect must stop ticking"
    );
}

fn player() -> Snapshot {
    Snapshot {
        player: Some(Player {
            x: 12.5,
            y: 64.0,
            z: -3.25,
            yaw: 180.0,
            pitch: 0.0,
            health: 20.0,
            food: 20,
            dimension: "minecraft:overworld".into(),
        }),
        clock: Some(Clock {
            game_time: 100,
            day_time: 24000 + 6000,
        }),
    }
}

#[test]
fn coords_hud_draws_and_answers() {
    let mut mods = load("coords_hud.hmod");
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(!mods.wants_world_read());
    assert_eq!(mods.commands().collect::<Vec<_>>(), ["where", "coords"]);
    mods.set_snapshot(player());
    mods.tick(1);
    let panel = mods.panels().next().expect("a panel after a tick");
    assert_eq!(panel.corner, Corner::TopLeft);
    let lines: Vec<&str> = panel.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        lines,
        [
            "XYZ 12.5 / 64.0 / -3.2",
            "Facing North (-Z)",
            "Day 2, 12:00"
        ]
    );

    assert!(mods.command("where"));
    let chat = Output::Chat {
        name: "Coordinates HUD".into(),
        text: "12, 64, -4 in minecraft:overworld".into(),
    };
    assert_eq!(mods.drain(), [chat]);
    assert!(mods.command("coords"));
    mods.tick(2);
    assert_eq!(mods.panels().count(), 0);
    let hidden = Output::Feedback {
        text: "Coordinates panel hidden".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [hidden]);
    assert!(!mods.command("gamemode creative"));
}

#[test]
fn command_player_identity_is_scoped_to_authenticated_dispatch() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(mods.commands().any(|name| name == "identity_probe"));
    let identity = horizon_mods::CommandPlayer {
        id: "12345678-1234-5678-9abc-def012345678".into(),
        name: "Guest".into(),
    };
    assert!(mods.command_as("identity_probe", Some(identity)));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "12345678-1234-5678-9abc-def012345678 Guest".into(),
            error: false,
        }]
    );
    // A command dispatched without a player identity (such as server console)
    // cannot observe the previous player's context.
    assert!(mods.command("identity_probe"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "no-player".into(),
            error: false,
        }]
    );
}

#[test]
fn off_mods_cost_nothing() {
    let mods = Mods::load(&[mods_dir().join("coords_hud.hmod")], |_| false, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Off);
    assert!(!mods.is_running());
    assert_eq!(mods.commands().count(), 0);
}

#[test]
fn undeclared_capabilities_are_refused() {
    let dir = mods_dir().join("undeclared");
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = Package::read(&mods_dir().join("horizon_mods_test_guest.hmod"))
        .unwrap()
        .wasm
        .expect("the guest has code");
    let manifest = GUEST_MANIFEST.replace("\"chat\", ", "");
    std::fs::write(dir.join("horizon-mod.toml"), manifest).unwrap();
    std::fs::write(dir.join("mod.wasm"), wasm).unwrap();
    let mods = Mods::load(&[dir], |_| true, |_| false);
    let refused = Status::Failed("uses capability chat, missing from horizon-mod.toml".into());
    assert_eq!(mods.entries()[0].status, refused);
}

#[test]
fn vanilla_commands_win() {
    let mods = Mods::load(
        &[mods_dir().join("coords_hud.hmod")],
        |_| true,
        |c| c == "where",
    );
    assert_eq!(mods.commands().collect::<Vec<_>>(), ["coords"]);
}

#[test]
fn a_misbehaving_mod_is_stopped_alone() {
    for (command, reason) in [
        ("spin", "it took too long (an endless loop?)"),
        ("panic", "it crashed (a panic)"),
        ("grow", "it crashed (a panic)"),
    ] {
        let paths = [
            mods_dir().join("coords_hud.hmod"),
            mods_dir().join("horizon_mods_test_guest.hmod"),
        ];
        let mut mods = Mods::load(&paths, |_| true, |_| false);
        let started = Instant::now();
        assert!(mods.command(command));
        assert!(started.elapsed().as_secs() < 5, "/{command} was cut short");
        let stopped = Output::Stopped {
            name: "Test Guest".into(),
            reason: reason.into(),
        };
        assert_eq!(mods.drain(), [stopped], "/{command}");
        assert_eq!(mods.entries()[1].status, Status::Failed(reason.into()));
        // The other mod runs on, and the stopped one's command is gone.
        assert!(mods.is_running());
        assert!(!mods.command("hello"));
        mods.set_snapshot(player());
        mods.tick(1);
        assert_eq!(mods.panels().count(), 1);
    }
}

#[test]
fn hot_reload_restarts_a_changed_mod() {
    let dir = mods_dir().join("reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    mods.tick(1);
    assert!(!mods.reload_changed(|_| false));
    // A rebuild: same bytes, newer file.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    assert!(mods.command("hello world"));
    let chat = Output::Chat {
        name: "Test Guest".into(),
        text: "hello world after 0 ticks".into(),
    };
    assert_eq!(mods.drain(), [chat]);
}

#[test]
fn storage_persists_across_reload() {
    let dir = mods_dir().join("storage_reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    assert!(mods.command("save remembered"));
    let saved = Output::Feedback {
        text: "saved".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [saved]);

    // A rebuild (hot_reload_restarts_a_changed_mod's pattern): a fresh Store,
    // but the same on-disk data directory.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));

    assert!(mods.command("load"));
    let chat = Output::Chat {
        name: "Test Guest".into(),
        text: "remembered".into(),
    };
    assert_eq!(mods.drain(), [chat]);
}

#[test]
fn storage_write_budget_limits_one_call() {
    let dir = mods_dir().join("storage_budget");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    // Under the 4 MiB on-disk quota, but over the 1 MiB a single call may
    // write: a loop replacing one key can never bypass the quota by
    // writing a lot in one call, only across many.
    let over_budget = "a".repeat((1 << 20) + 1024);
    assert!(mods.command(&format!("save {over_budget}")));
    let refused = Output::Feedback {
        text: "too much storage traffic in one call".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn storage_quota_is_enforced() {
    let dir = mods_dir().join("storage_quota");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    // Each call is under the per-call write budget; distinct keys so the
    // quota (not the fixed "note" key being overwritten) is what's tested.
    let chunk = "a".repeat(900 << 10);
    for i in 0..4 {
        assert!(mods.command(&format!("save_key k{i} {chunk}")));
        assert_eq!(
            mods.drain(),
            [Output::Feedback {
                text: "saved".into(),
                error: false,
            }]
        );
    }
    assert!(mods.command(&format!("save_key k4 {chunk}")));
    let refused = Output::Feedback {
        text: "would use over 4 MiB of storage".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn storage_key_count_is_bounded() {
    let dir = mods_dir().join("storage_keys");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    for i in 0..64 {
        assert!(mods.command(&format!("save_key k{i} x")));
        assert_eq!(
            mods.drain(),
            [Output::Feedback {
                text: "saved".into(),
                error: false,
            }],
            "key {i}"
        );
    }
    assert!(mods.command("save_key k64 x"));
    let refused = Output::Feedback {
        text: "would hold over 64 keys".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn duplicate_mod_ids_are_refused() {
    let dir = mods_dir().join("duplicate_ids");
    std::fs::create_dir_all(&dir).unwrap();
    let first = dir.join("first.hmod");
    let second = dir.join("second.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &first).unwrap();
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &second).unwrap();
    // Both declare id "test_guest" (the guest's own manifest): sharing a
    // `data_dir` would let the second read, overwrite or delete the
    // first's storage, so only the first-loaded one may start.
    let mods = Mods::load(&[first, second], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    let refused = Status::Failed("another running mod already uses id test_guest".into());
    assert_eq!(mods.entries()[1].status, refused);
}

/// `cargo test -p horizon-mods --release -- --ignored --nocapture`: the
/// cost of the per-tick and per-frame hooks (docs/mods.md, Performance).
#[test]
#[ignore]
fn hook_overhead() {
    let time = |mods: &mut Mods, frame: bool| {
        let n = 200_000;
        for i in 0..1000 {
            mods.tick(i);
        }
        let started = Instant::now();
        for i in 0..n {
            if frame {
                mods.frame(0.016);
            } else {
                mods.tick(i);
            }
        }
        started.elapsed().as_nanos() as f64 / n as f64
    };
    let mut guest = load("horizon_mods_test_guest.hmod");
    let mut hud = load("coords_hud.hmod");
    hud.set_snapshot(player());
    let mut none = Mods::default();
    println!("no mods, tick:        {:8.1} ns", time(&mut none, false));
    println!("empty tick hook:      {:8.1} ns", time(&mut guest, false));
    println!("empty frame hook:     {:8.1} ns", time(&mut guest, true));
    println!("coords-hud tick hook: {:8.1} ns", time(&mut hud, false));
    let started = Instant::now();
    let _ = load("coords_hud.hmod");
    println!(
        "load coords-hud:      {:8.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn content_only_mods_load_without_an_engine() {
    let manifest = r#"
id = "ruby"
name = "Ruby"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
"#;
    let block: &[u8] = br#"{"template": "minecraft:iron_block"}"#;
    let dir = mods_dir().join("content");
    std::fs::create_dir_all(&dir).unwrap();
    let ok = dir.join("ruby.hmod");
    let files: &[(&str, &[u8])] = &[("data/ruby/horizon/block/ruby_block.json", block)];
    std::fs::write(&ok, Package::write_files(manifest, None, files).unwrap()).unwrap();
    let bad = dir.join("bad.hmod");
    let files: &[(&str, &[u8])] = &[("data/other/recipe/stone.json", b"{}")];
    let bad_manifest = manifest.replace("\"ruby\"", "\"bad\"");
    std::fs::write(
        &bad,
        Package::write_files(&bad_manifest, None, files).unwrap(),
    )
    .unwrap();
    let mods = Mods::load(&[ok, bad], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(matches!(&mods.entries()[1].status, Status::Failed(e) if e.contains("own namespace")));
    // Content has no hooks: the game's per-tick mod work stays skipped.
    assert!(!mods.is_running());
}

/// Raw iron smelts into iron ingots in 3 ticks; `test:ghost` is a recipe
/// output no item table knows.
struct Facts;
impl ItemFacts for Facts {
    fn block_in_tag(&self, id: &str, tag: &str) -> bool {
        id == "minecraft:magma_block" && tag == "farmersdelight:heat_sources"
    }
    fn max_stack_size(&self, id: &str) -> u32 {
        if id == "test:ghost" { 0 } else { 64 }
    }
    fn burn_time(&self, _id: &str) -> u32 {
        0
    }
    fn remainder(&self, id: &str) -> Option<String> {
        (id == "minecraft:milk_bucket").then(|| "minecraft:bucket".into())
    }
    fn cooking_recipe(&self, kind: CookingKind, input: &str) -> Option<CookingRecipe> {
        let output = match (kind, input) {
            (CookingKind::Smelting, "minecraft:raw_iron") => "minecraft:iron_ingot",
            (CookingKind::Smelting, "test:haunted") => "test:ghost",
            _ => return None,
        };
        Some(CookingRecipe {
            output: stack(output, 1),
            cooking_time: 3,
            experience: 0.7,
        })
    }
    fn item_in_tag(&self, id: &str, tag: &str) -> bool {
        tag == "c:raw_iron" && id == "minecraft:raw_iron"
    }
}

fn stack(id: &str, count: u32) -> Stack {
    Stack {
        id: id.into(),
        count,
    }
}

fn smelter(handle: u32, input: Option<Stack>) -> BlockEntityView {
    BlockEntityView {
        handle,
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        slots: vec![input, None],
        slot_components: vec![Vec::new(); 2],
        energy: None,
        fluids: Default::default(),
        properties: vec![Property {
            name: "lit".into(),
            value: "false".into(),
            values: vec!["false".into(), "true".into()],
        }],
        changed: false,
    }
}

fn machines() -> Mods {
    let mut mods = load("horizon_mods_test_machine.hmod");
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(mods.wants_world_read());
    mods.set_item_facts(std::sync::Arc::new(Facts));
    mods
}

#[test]
fn player_slot_policies_are_owned_bounded_and_removed_with_handles() {
    let paths = [
        mods_dir().join("horizon_mods_test_machine.hmod"),
        mods_dir().join("horizon_mods_test_machine_peer.hmod"),
    ];
    let mut mods = Mods::load(&paths, |_| true, |_| false);
    mods.set_item_facts(std::sync::Arc::new(Facts));
    let (peer, _) = mods.create_block_entity("test_peer:cell", &[]).unwrap();
    let (handle, _) = mods.create_block_entity("test_machine:crate", &[]).unwrap();
    assert_eq!(mods.player_slot_access(handle, 1), (true, true));
    assert!(mods.command(&format!("machine_api player_access {handle} 1 false false")));
    assert_eq!(mods.player_slot_access(handle, 1), (false, false));
    assert_eq!(mods.player_slot_access(handle, 0), (true, true));
    assert_eq!(mods.player_slot_access(handle, 4), (false, false));
    assert_eq!(mods.player_slot_access(u32::MAX, 0), (false, false));
    assert!(mods.command(&format!("machine_api player_access {handle} 1 false true")));
    assert_eq!(mods.player_slot_access(handle, 1), (false, true));
    assert!(mods.command(&format!("machine_api player_access {handle} 1 true true")));
    assert_eq!(mods.player_slot_access(handle, 1), (true, true));
    mods.drain();
    for (target, slot) in [(peer, 0), (u32::MAX, 0), (handle, 4)] {
        assert!(mods.command(&format!(
            "machine_api player_access {target} {slot} false false"
        )));
        assert!(mods.drain().iter().any(|output| matches!(output,
            Output::Feedback { text, error: false } if text == "false")));
    }
    assert_eq!(mods.player_slot_access(peer, 0), (false, false));
    mods.remove_block_entity(handle);
    assert_eq!(mods.player_slot_access(handle, 1), (false, false));
    let (replacement, _) = mods.create_block_entity("test_machine:crate", &[]).unwrap();
    assert_ne!(replacement, handle);
    assert_eq!(mods.player_slot_access(replacement, 1), (true, true));
    let (crashing, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    mods.tick_block_entities(&mut [smelter(crashing, Some(stack("minecraft:tnt", 1)))]);
    assert_eq!(mods.player_slot_access(replacement, 1), (false, false));
    assert_eq!(mods.player_slot_access(peer, 0), (false, false));
    assert_eq!(
        mods.entries()
            .iter()
            .find(|entry| entry
                .manifest
                .as_ref()
                .is_some_and(|manifest| manifest.id == "test_peer"))
            .unwrap()
            .status,
        Status::Running
    );
}

#[test]
fn item_processing_is_atomic_across_all_outputs_and_preserves_remaining_components() {
    use horizon_world::nbt::{Limits, Tag, write_network};
    let mut mods = machines();
    let (handle, _) = mods.create_block_entity("test_machine:crate", &[]).unwrap();
    assert!(mods.command("machine_api atomic_items"));
    let component = write_network(
        &Tag::compound([("minecraft:custom_name", Tag::String("Ore".into()))]),
        Limits::default(),
    )
    .unwrap();
    for (input_count, output1, output2, success) in [
        (3, None, Some(stack("minecraft:gold_ingot", 64)), false),
        (3, Some(stack("minecraft:diamond", 1)), None, false),
        (1, None, None, false),
        (3, None, None, true),
    ] {
        let mut view = BlockEntityView {
            handle,
            dimension: "minecraft:overworld".into(),
            y: 64,
            slots: vec![
                Some(stack("minecraft:raw_iron", input_count)),
                output1,
                output2,
                None,
            ],
            slot_components: vec![component.clone(), Vec::new(), Vec::new(), Vec::new()],
            ..Default::default()
        };
        let before = view.clone();
        mods.tick_block_entities(std::slice::from_mut(&mut view));
        if success {
            assert_eq!(view.slots[0], Some(stack("minecraft:raw_iron", 1)));
            assert_eq!(view.slot_components[0], component);
            assert_eq!(view.slots[1], Some(stack("minecraft:iron_ingot", 1)));
            assert_eq!(view.slots[2], Some(stack("minecraft:gold_ingot", 1)));
        } else {
            assert_eq!(view, before);
        }
    }
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn machine_block_tags_use_the_active_game_facts() {
    let mut mods = machines();
    assert!(mods.command("machine_api block_tags"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "heat=true cold=false unknown=false".into(),
            error: false
        }]
    );
}

#[test]
fn item_move_intents_are_deferred_confirmed_once_and_invalidated_on_removal_or_trap() {
    let mut mods = machines();
    let (a, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let (b, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    assert!(mods.command("machine_api move_pull"));
    let mut views = [
        smelter(b, None),
        smelter(a, Some(stack("minecraft:diamond", 7))),
    ];
    mods.tick_block_entities(&mut views);
    assert_eq!(
        views[1].slots[0],
        Some(stack("minecraft:diamond", 7)),
        "request does not modify snapshots"
    );
    let requests: Vec<_> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::MoveItem {
                handle,
                request,
                slot,
                count,
                face,
                kind,
                ..
            }) => {
                assert_eq!(
                    (slot, count, face.as_str(), kind),
                    (0, 1, "east", horizon_mods::ItemMoveKind::Pull)
                );
                Some((handle, request))
            }
            _ => None,
        })
        .collect();
    assert_eq!(requests.len(), 2);
    for &(handle, request) in &requests {
        assert!(mods.item_move_pending(handle, request));
        let wrong = if handle == a { b } else { a };
        assert!(!mods.item_move_pending(wrong, request));
        mods.complete_item_move(wrong, request, 50);
        assert!(mods.item_move_pending(handle, request));
        mods.complete_item_move(handle, request, 1);
        mods.complete_item_move(handle, request, 99);
        assert!(!mods.item_move_pending(handle, request));
    }
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(a), [1]);
    assert_eq!(mods.block_entity_data(b), [1]);
    let pending: Vec<_> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::MoveItem {
                handle, request, ..
            }) => Some((handle, request)),
            _ => None,
        })
        .collect();
    mods.remove_block_entity(a);
    for &(handle, request) in &pending {
        assert_eq!(mods.item_move_pending(handle, request), handle == b);
    }
    assert!(mods.command("machine_api move_trap"));
    mods.tick_block_entities(&mut views[..1]);
    assert!(
        matches!(&mods.entries()[0].status, Status::Failed(reason) if reason.contains("panic"))
    );
    for output in mods.drain() {
        if let Output::Server(ServerAction::MoveItem {
            handle, request, ..
        }) = output
        {
            assert!(
                !mods.item_move_pending(handle, request),
                "trap must revoke queued authority"
            );
        }
    }
}

#[test]
fn item_move_request_ids_cannot_cross_a_hot_reload_even_when_reused() {
    let dir = mods_dir().join("item_move_reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("machine.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_machine.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    let (old, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert!(mods.command("machine_api move_pull"));
    let mut views = [smelter(old, None)];
    mods.tick_block_entities(&mut views);
    let old_request = mods
        .drain()
        .into_iter()
        .find_map(|output| match output {
            Output::Server(ServerAction::MoveItem { request, .. }) => Some(request),
            _ => None,
        })
        .unwrap();
    assert!(mods.item_move_pending(old, old_request));
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    let (new, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_ne!(old, new);
    views[0].handle = new;
    mods.tick_block_entities(&mut views);
    let new_request = mods
        .drain()
        .into_iter()
        .find_map(|output| match output {
            Output::Server(ServerAction::MoveItem { request, .. }) => Some(request),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        old_request, new_request,
        "exercise a reused per-instance request id"
    );
    assert!(!mods.item_move_pending(old, old_request));
    mods.complete_item_move(old, old_request, 7);
    assert!(mods.item_move_pending(new, new_request));
    mods.complete_item_move(new, new_request, 0);
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(new), [0]);
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn item_pickup_intents_are_confirmed_once_and_invalidated_on_failure_removal_or_trap() {
    let mut mods = machines();
    let (a, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let (b, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut views = [
        smelter(a, Some(stack("minecraft:diamond", 3))),
        smelter(b, None),
    ];

    assert!(mods.command("machine_api pickup_request"));
    mods.tick_block_entities(&mut views);
    let requests: Vec<_> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::PickupItem {
                handle,
                request,
                slot,
                ..
            }) => {
                assert_eq!(slot, 0);
                Some((handle, request))
            }
            _ => None,
        })
        .collect();
    assert_eq!(requests.len(), 2);
    for &(handle, request) in &requests {
        assert!(mods.item_pickup_pending(handle, request));
        let wrong = if handle == a { b } else { a };
        assert!(!mods.item_pickup_pending(wrong, request));
        mods.complete_item_pickup(wrong, request, 99);
        assert!(mods.item_pickup_pending(handle, request));
        mods.complete_item_pickup(handle, request, 1);
        mods.complete_item_pickup(handle, request, 9);
        assert!(!mods.item_pickup_pending(handle, request));
    }

    assert!(mods.command("machine_api pickup_stop"));
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(a), [1]);
    assert_eq!(mods.block_entity_data(b), [1]);
    assert!(
        mods.entries()
            .iter()
            .all(|entry| entry.status == Status::Running)
    );

    // A zero result is a confirmed failure and a duplicate completion cannot
    // turn it into a successful pickup.
    assert!(mods.command("machine_api pickup_request"));
    mods.tick_block_entities(&mut views[..1]);
    let (handle, request) = mods
        .drain()
        .into_iter()
        .find_map(|output| match output {
            Output::Server(ServerAction::PickupItem {
                handle, request, ..
            }) => Some((handle, request)),
            _ => None,
        })
        .unwrap();
    mods.complete_item_pickup(handle, request, 0);
    mods.complete_item_pickup(handle, request, 1);
    assert!(!mods.item_pickup_pending(handle, request));
    assert!(mods.command("machine_api pickup_stop"));
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(handle), [1]);

    // Removal revokes only that machine's request; the peer request remains valid.
    assert!(mods.command("machine_api pickup_request"));
    mods.tick_block_entities(&mut views);
    let pending: Vec<_> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::PickupItem {
                handle, request, ..
            }) => Some((handle, request)),
            _ => None,
        })
        .collect();
    mods.remove_block_entity(a);
    for &(handle, request) in &pending {
        assert_eq!(mods.item_pickup_pending(handle, request), handle == b);
    }
    for &(handle, request) in &pending {
        if handle == b {
            mods.complete_item_pickup(handle, request, 0);
        }
    }
    assert!(mods.command("machine_api pickup_stop"));
    mods.tick_block_entities(&mut views[1..]);

    assert!(mods.command("machine_api pickup_trap"));
    mods.tick_block_entities(&mut views[1..]);
    assert!(
        matches!(&mods.entries()[0].status, Status::Failed(reason) if reason.contains("panic"))
    );
    let trap_requests: Vec<_> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::PickupItem {
                handle, request, ..
            }) => Some((handle, request)),
            _ => None,
        })
        .collect();
    assert!(
        !trap_requests.is_empty(),
        "trap must follow an emitted pickup request"
    );
    for (handle, request) in trap_requests {
        assert!(
            !mods.item_pickup_pending(handle, request),
            "trap must revoke queued pickup authority"
        );
    }
}

#[test]
fn item_pickup_request_ids_cannot_cross_a_hot_reload_even_when_reused() {
    let dir = mods_dir().join("item_pickup_reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("machine.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_machine.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    let (old, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert!(mods.command("machine_api pickup_request"));
    let mut views = [smelter(old, None)];
    mods.tick_block_entities(&mut views);
    let old_request = mods
        .drain()
        .into_iter()
        .find_map(|output| match output {
            Output::Server(ServerAction::PickupItem { request, .. }) => Some(request),
            _ => None,
        })
        .unwrap();
    assert!(mods.item_pickup_pending(old, old_request));
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    let (new, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_ne!(old, new);
    views[0].handle = new;
    mods.tick_block_entities(&mut views);
    let new_request = mods
        .drain()
        .into_iter()
        .find_map(|output| match output {
            Output::Server(ServerAction::PickupItem { request, .. }) => Some(request),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        old_request, new_request,
        "exercise a reused per-instance request id"
    );
    assert!(!mods.item_pickup_pending(old, old_request));
    mods.complete_item_pickup(old, old_request, 7);
    assert!(mods.item_pickup_pending(new, new_request));
    mods.complete_item_pickup(new, new_request, 0);
    assert!(!mods.item_pickup_pending(new, new_request));
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn item_energy_transfers_preserve_components_and_refuse_invalid_slots_and_stacks() {
    use horizon_world::nbt::{Limits, Tag, read_network, write_network};
    let mut mods = machines();
    let (handle, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut views = [smelter(handle, Some(stack("test_machine:battery", 1)))];
    let name = Tag::String("Battery name".into());
    views[0].slot_components[0] = write_network(
        &Tag::compound([("minecraft:custom_name", name.clone())]),
        Limits::default(),
    )
    .unwrap();
    assert!(mods.command("machine_api item_energy_receive"));
    for expected in [800, 1600, 2400, 2500, 2500] {
        views[0].changed = false;
        mods.tick_block_entities(&mut views);
        let (tag, _) = read_network(&views[0].slot_components[0], Limits::default()).unwrap();
        assert_eq!(tag.get("horizon:energy"), Some(&Tag::Long(expected)));
        assert_eq!(tag.get("minecraft:custom_name"), Some(&name));
    }
    assert!(!views[0].changed, "full item must not dirty the slot");
    assert!(mods.command("machine_api item_energy_extract"));
    mods.tick_block_entities(&mut views);
    let (tag, _) = read_network(&views[0].slot_components[0], Limits::default()).unwrap();
    assert_eq!(tag.get("horizon:energy"), Some(&Tag::Long(2200)));
    for (id, count, component) in [
        ("minecraft:stick", 1, Tag::Long(0)),
        ("test_machine:battery", 2, Tag::Long(10)),
        ("test_machine:battery", 1, Tag::Long(-1)),
        ("test_machine:battery", 1, Tag::Long(2501)),
        ("test_machine:battery", 1, Tag::Int(10)),
    ] {
        views[0].slots[0] = Some(stack(id, count));
        let bytes = write_network(
            &Tag::compound([("horizon:energy", component)]),
            Limits::default(),
        )
        .unwrap();
        views[0].slot_components[0] = bytes.clone();
        for operation in ["item_energy_receive", "item_energy_extract"] {
            assert!(mods.command(&format!("machine_api {operation}")));
            mods.tick_block_entities(&mut views);
            assert_eq!(views[0].slot_components[0], bytes);
        }
    }
    // A valid component at the byte limit leaves no room for the energy key.
    // Both simulation and commit must refuse without dropping the other data.
    views[0].slots[0] = Some(stack("test_machine:battery", 1));
    let overhead = write_network(
        &Tag::compound([("padding", Tag::ByteArray(Vec::new()))]),
        Limits::default(),
    )
    .unwrap()
    .len();
    let bytes = write_network(
        &Tag::compound([("padding", Tag::ByteArray(vec![0; (64 << 10) - overhead]))]),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(bytes.len(), 64 << 10);
    views[0].slot_components[0] = bytes.clone();
    assert!(mods.command("machine_api item_energy_receive"));
    mods.tick_block_entities(&mut views);
    assert_eq!(views[0].slot_components[0], bytes);
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn stack_energy_capacity_uses_owner_registered_component_and_preserves_charge() {
    use horizon_world::nbt::{Limits, Tag, read_network, write_network};
    let mut mods = machines();
    let (handle, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut view = smelter(handle, Some(stack("test_machine:capacity_battery", 1)));
    let name = Tag::String("capacity test battery".into());
    view.slot_components[0] = write_network(
        &Tag::compound([
            ("test_machine:capacity", Tag::Int(5000)),
            ("horizon:energy", Tag::Long(4200)),
            ("minecraft:custom_name", name.clone()),
        ]),
        Limits::default(),
    )
    .unwrap();
    assert!(mods.command("machine_api item_energy_dynamic_capacity"));
    mods.tick_block_entities(std::slice::from_mut(&mut view));
    let (updated, _) = read_network(&view.slot_components[0], Limits::default()).unwrap();
    assert_eq!(updated.get("test_machine:capacity"), Some(&Tag::Int(6000)));
    assert_eq!(updated.get("horizon:energy"), Some(&Tag::Long(5000)));
    assert_eq!(updated.get("minecraft:custom_name"), Some(&name));

    for invalid_capacity in [
        Tag::Int(0),
        Tag::Int(-1),
        Tag::Long(-1),
        Tag::String("5000".into()),
    ] {
        view.slot_components[0] = write_network(
            &Tag::compound([
                ("test_machine:capacity", invalid_capacity),
                ("horizon:energy", Tag::Long(0)),
            ]),
            Limits::default(),
        )
        .unwrap();
        let before = view.slot_components[0].clone();
        assert!(mods.command("machine_api item_energy_receive"));
        mods.tick_block_entities(std::slice::from_mut(&mut view));
        assert_eq!(view.slot_components[0], before);
    }

    view.slot_components[0] = write_network(
        &Tag::compound([
            ("test_machine:capacity", Tag::Int(4000)),
            ("horizon:energy", Tag::Long(4001)),
        ]),
        Limits::default(),
    )
    .unwrap();
    let before = view.slot_components[0].clone();
    assert!(mods.command("machine_api item_energy_receive"));
    mods.tick_block_entities(std::slice::from_mut(&mut view));
    assert_eq!(view.slot_components[0], before);

    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn block_entities_tick_save_and_load() {
    let mut mods = machines();
    let types: Vec<(&str, u32, bool)> = mods
        .block_entity_types()
        .map(|t| (t.block.as_str(), t.slots, t.ticks))
        .collect();
    assert_eq!(
        types,
        [
            ("test_machine:smelter", 2, true),
            ("test_machine:crate", 4, false),
            ("test_machine:shared_router", 4, true)
        ]
    );
    assert_eq!(mods.create_block_entity("minecraft:furnace", &[]), None);
    let (h, slots) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_eq!(slots, 2);
    assert_eq!(mods.slot_access(h, 0, "up"), (false, true));
    assert_eq!(mods.slot_access(h, 0, "down"), (true, false));
    assert_eq!(mods.slot_access(h, 1, "up"), (false, true));
    let mut views = [smelter(h, Some(stack("minecraft:raw_iron", 2)))];
    let components = horizon_world::nbt::write_network(
        &horizon_world::nbt::Tag::compound([(
            "minecraft:custom_name",
            horizon_world::nbt::Tag::String("Forged iron".into()),
        )]),
        horizon_world::nbt::Limits::default(),
    )
    .unwrap();
    views[0].slot_components[0] = components.clone();
    mods.tick_block_entities(&mut views);
    assert_eq!(views[0].slot_components[0], components);
    assert!(views[0].changed, "lit turned on");
    assert_eq!(views[0].properties[0].value, "true");
    assert_eq!(mods.block_entity_data(h), [1]);
    views[0].changed = false;
    mods.tick_block_entities(&mut views);
    assert!(views[0].changed, "capability state changed");
    assert_eq!(views[0].energy.unwrap().stored, 10);
    assert_eq!(views[0].fluids[&0].amount, 20);
    mods.tick_block_entities(&mut views);
    assert_eq!(
        views[0].slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:iron_ingot", 1))
        ]
    );
    assert_eq!(views[0].slot_components[0], components);
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(h), [1]);

    // Saved and loaded again (a chunk unloads and comes back).
    let saved = mods.save_block_entity(h).unwrap();
    mods.remove_block_entity(h);
    assert_eq!(mods.block_entity_data(h), Vec::<i32>::new());
    assert_eq!(mods.slot_access(h, 0, "up"), (true, true));
    let (again, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &saved,
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_ne!(again, h);
    assert_eq!(mods.block_entity_data(again), [1]);
    assert!(mods.drain().is_empty());
}

#[test]
fn machine_output_insertion_is_atomic_and_respects_stack_capacity() {
    let mut mods = machines();
    let (handle, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut view = smelter(handle, Some(stack("minecraft:raw_iron", 2)));
    view.slots[1] = Some(stack("minecraft:iron_ingot", 64));
    for _ in 0..3 {
        mods.tick_block_entities(std::slice::from_mut(&mut view));
    }
    assert_eq!(
        view.slots,
        [
            Some(stack("minecraft:raw_iron", 2)),
            Some(stack("minecraft:iron_ingot", 64))
        ],
        "a full output must not consume input or change the output"
    );

    view.slots[1] = Some(stack("minecraft:iron_ingot", 63));
    let components = horizon_world::nbt::write_network(
        &horizon_world::nbt::Tag::compound([(
            "minecraft:custom_name",
            horizon_world::nbt::Tag::String("Marked ingot".into()),
        )]),
        horizon_world::nbt::Limits::default(),
    )
    .unwrap();
    view.slot_components[1] = components.clone();
    for _ in 0..3 {
        mods.tick_block_entities(std::slice::from_mut(&mut view));
    }
    assert_eq!(
        view.slots,
        [
            Some(stack("minecraft:raw_iron", 2)),
            Some(stack("minecraft:iron_ingot", 63))
        ],
        "a componentful output stack must not merge with a default result"
    );
    assert_eq!(view.slot_components[1], components);

    view.slot_components[1].clear();
    for _ in 0..3 {
        mods.tick_block_entities(std::slice::from_mut(&mut view));
    }
    assert_eq!(
        view.slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:iron_ingot", 64))
        ],
        "a result should fit exactly at the stack limit"
    );

    view.slots[1] = Some(stack("minecraft:coal", 1));
    for _ in 0..3 {
        mods.tick_block_entities(std::slice::from_mut(&mut view));
    }
    assert_eq!(
        view.slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:coal", 1))
        ],
        "a different output item must leave both stacks untouched"
    );
}

#[test]
fn shared_inventory_machine_calls_and_capabilities_use_one_canonical_slot() {
    let mut mods = machines();
    let (a, _) = mods
        .create_block_entity_at(
            "test_machine:crate",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    let (b, _) = mods
        .create_block_entity_at(
            "test_machine:crate",
            &[],
            Some(("minecraft:overworld".into(), 10, 64, 0)),
        )
        .unwrap();
    let make = |handle, x| BlockEntityView {
        handle,
        dimension: "minecraft:overworld".into(),
        x,
        y: 64,
        slots: vec![Some(stack("minecraft:raw_iron", 3)), None, None, None],
        slot_components: vec![Vec::new(); 4],
        ..Default::default()
    };
    let mut views = [make(b, 10), make(a, 0)];
    mods.tick_block_entities(&mut views);
    for view in &views {
        assert_eq!(view.slots[0], Some(stack("minecraft:raw_iron", 1)));
        assert_eq!(view.slots[1], Some(stack("minecraft:iron_ingot", 2)));
        assert!(view.changed);
    }
    mods.tick_block_entities(&mut views[1..]);
    assert_eq!(views[1].slots[0], None);
    assert_eq!(views[1].slots[1], Some(stack("minecraft:iron_ingot", 3)));
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn shared_inventory_bindings_belong_to_live_handles() {
    let mut mods = machines();
    let (a, _) = mods.create_block_entity("test_machine:crate", &[]).unwrap();
    let (b, _) = mods.create_block_entity("test_machine:crate", &[]).unwrap();
    let expected = Some(("test_machine".into(), "public".into()));
    assert_eq!(mods.shared_inventory_channel(a), expected);
    assert_eq!(mods.shared_inventory_channel(b), expected);
    assert_eq!(mods.shared_inventory_channel(b + 1), None);
    mods.remove_block_entity(a);
    assert_eq!(mods.shared_inventory_channel(a), None);
    assert_eq!(mods.shared_inventory_channel(b), expected);
}

#[test]
fn block_entity_views_follow_handles_across_reordered_subsets() {
    let paths = [
        mods_dir().join("horizon_mods_test_machine.hmod"),
        mods_dir().join("horizon_mods_test_machine_peer.hmod"),
    ];
    let mut mods = Mods::load(&paths, |_| true, |_| false);
    mods.set_item_facts(std::sync::Arc::new(Facts));
    let (a, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let (peer, _) = mods.create_block_entity("test_peer:cell", &[]).unwrap();
    let (b, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut peer_view = smelter(peer, Some(stack("minecraft:diamond", 7)));
    peer_view.x = 20;
    let mut views = [
        smelter(b, Some(stack("minecraft:raw_iron", 4))),
        peer_view.clone(),
        smelter(a, Some(stack("minecraft:raw_iron", 2))),
    ];
    views[0].x = 40;
    for _ in 0..3 {
        mods.tick_block_entities(&mut views);
    }
    assert_eq!(
        views[0].slots,
        [
            Some(stack("minecraft:raw_iron", 3)),
            Some(stack("minecraft:iron_ingot", 1))
        ]
    );
    assert_eq!(
        views[2].slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:iron_ingot", 1))
        ]
    );
    // Only a is lent next; b's old index must no longer resolve to a.
    let b_before = views[0].clone();
    for _ in 0..3 {
        mods.tick_block_entities(&mut views[2..]);
    }
    assert_eq!(views[0], b_before);
    assert_eq!(
        views[2].slots,
        [None, Some(stack("minecraft:iron_ingot", 2))]
    );
    // Put b at a different index and switch the active subset again.
    views.swap(0, 2);
    for _ in 0..3 {
        mods.tick_block_entities(&mut views[1..]);
    }
    assert_eq!(views[2].handle, b);
    assert_eq!(
        views[2].slots,
        [
            Some(stack("minecraft:raw_iron", 2)),
            Some(stack("minecraft:iron_ingot", 2))
        ]
    );
    assert_eq!(
        views[0].slots,
        [None, Some(stack("minecraft:iron_ingot", 2))]
    );
    assert_eq!(views[1].handle, peer);
    assert_eq!(views[1].slots, peer_view.slots);
    assert!(
        mods.entries()
            .iter()
            .all(|entry| entry.status == Status::Running)
    );
}

#[test]
fn block_entities_see_only_their_own() {
    let mut mods = machines();
    let (h, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    // A view with a handle the mod never got, and an output no table knows:
    // the stranger is not lent, and the ghost item is refused.
    let mut views = [
        smelter(h + 100, Some(stack("minecraft:raw_iron", 1))),
        smelter(h, Some(stack("test:haunted", 1))),
    ];
    for _ in 0..3 {
        mods.tick_block_entities(&mut views);
    }
    assert_eq!(
        views[0],
        smelter(h + 100, Some(stack("minecraft:raw_iron", 1)))
    );
    assert_eq!(views[1].slots, [Some(stack("test:haunted", 1)), None]);
    assert_eq!(views[1].energy.unwrap().stored, 15);
    assert_eq!(views[1].fluids[&0].amount, 30);
    assert!(views[1].changed);
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn machine_api_reports_only_cardinal_same_dimension_neighbors_in_face_order() {
    let mut mods = machines();
    let create = |mods: &mut Mods, dimension: &str, x, y, z| {
        mods.create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some((dimension.into(), x, y, z)),
        )
        .unwrap()
        .0
    };
    let center = create(&mut mods, "minecraft:overworld", 0, 64, 0);
    let east = create(&mut mods, "minecraft:overworld", 1, 64, 0);
    let up = create(&mut mods, "minecraft:overworld", 0, 65, 0);
    let north = create(&mut mods, "minecraft:overworld", 0, 64, -1);
    let diagonal = create(&mut mods, "minecraft:overworld", 1, 65, 0);
    let other_dimension = create(&mut mods, "minecraft:the_nether", -1, 64, 0);

    assert!(mods.command(&format!("machine_api adjacent {center}")));
    let mut views = [
        smelter(center, None),
        smelter(east, None),
        smelter(up, None),
        smelter(north, None),
        smelter(diagonal, None),
        smelter(other_dimension, None),
    ];
    mods.tick_block_entities(&mut views);
    assert!(mods.command("machine_api adjacent_result"));
    assert!(mods.drain().contains(&Output::Feedback {
        text: format!("up={up},north={north},east={east}"),
        error: false,
    }));
}

#[test]
fn a_mod_cannot_redefine_another_machines_capability_storage() {
    let paths = [
        mods_dir().join("horizon_mods_test_machine.hmod"),
        mods_dir().join("horizon_mods_test_machine_peer.hmod"),
    ];
    let mut mods = Mods::load(&paths, |_| true, |_| false);
    mods.set_item_facts(std::sync::Arc::new(Facts));
    let (machine_handle, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    let (peer_handle, _) = mods
        .create_block_entity_at(
            "test_peer:cell",
            &[],
            Some(("minecraft:overworld".into(), 1, 64, 0)),
        )
        .unwrap();
    let mut views = [
        smelter(machine_handle, None),
        BlockEntityView {
            handle: peer_handle,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            ..BlockEntityView::default()
        },
    ];

    mods.tick_block_entities(&mut views);

    let donor_energy = views[0].energy.unwrap();
    assert_eq!(donor_energy.capacity, 10_000);
    assert_eq!(donor_energy.max_receive, 25);
    assert_eq!(views[0].fluids[&0].fluid, "minecraft:water");
    assert_eq!(views[0].fluids[&0].capacity, 1_000);
    assert_eq!(views[0].slots, [None, None]);
    assert_eq!(views[1].energy.unwrap().capacity, 1_000);
    assert_eq!(views[1].fluids[&0].fluid, "");
    assert_eq!(views[1].fluids[&0].capacity, 16);
}

#[test]
fn block_entities_need_their_capability_and_blocks() {
    let wasm = Package::read(&mods_dir().join("horizon_mods_test_machine.hmod"))
        .unwrap()
        .wasm
        .unwrap();
    let cases = [
        (
            MACHINE_MANIFEST.replace(", \"block-entities\"", ""),
            "uses capability block-entities, missing from horizon-mod.toml",
        ),
        (
            MACHINE_MANIFEST.replace("id = \"test_machine\"", "id = \"other\""),
            "declares a block entity for test_machine:smelter, which is not one of its blocks",
        ),
    ];
    for (n, (manifest, why)) in cases.into_iter().enumerate() {
        let dir = mods_dir().join(format!("machine-refused-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("horizon-mod.toml"), manifest).unwrap();
        std::fs::write(dir.join("mod.wasm"), &wasm).unwrap();
        let mods = Mods::load(&[dir], |_| true, |_| false);
        assert_eq!(mods.entries()[0].status, Status::Failed(why.into()));
        assert_eq!(mods.block_entity_types().count(), 0);
    }
}

#[test]
fn a_block_entity_that_traps_or_exhausts_fuel_gives_its_slots_back() {
    for input in ["minecraft:tnt", "minecraft:gunpowder"] {
        let mut mods = machines();
        let (h, _) = mods
            .create_block_entity("test_machine:smelter", &[])
            .unwrap();
        let (other, _) = mods
            .create_block_entity("test_machine:smelter", &[])
            .unwrap();
        let mut untouched = smelter(other, Some(stack("minecraft:raw_iron", 2)));
        untouched.x = 20;
        let mut views = [smelter(h, Some(stack(input, 3))), untouched.clone()];
        mods.tick_block_entities(&mut views);
        assert!(matches!(mods.entries()[0].status, Status::Failed(_)));
        assert!(matches!(mods.drain()[..], [Output::Stopped { .. }]));
        // Its slots are back, with what it wrote before it stopped.
        assert_eq!(views[0].handle, h);
        assert_eq!(views[0].slots, [Some(stack(input, 3)), None]);
        assert_eq!(views[0].properties[0].value, "true");
        assert_eq!(views[1], untouched);
        // A stopped mod's block entities are left alone after that.
        mods.tick_block_entities(&mut views);
        assert_eq!(views[0].slots, [Some(stack(input, 3)), None]);
        assert_eq!(mods.save_block_entity(h), None);
    }
}

#[test]
fn api_0_3_and_0_4_logic_and_machine_hooks_run() {
    let _ = std::fs::remove_file(
        mods_dir()
            .join(".horizon-mod-data")
            .join("test_machine.config.json"),
    );
    let mut mods = machines();
    mods.add_resource_recipes([
        RecipeEntry {
            id: "minecraft:raw_iron_smelting".into(),
            kind: "minecraft:smelting".into(),
            inputs: vec![IngredientSlot {
                items: vec!["#c:raw_iron".into()],
                count: 1,
            }],
            outputs: vec!["minecraft:iron_ingot*1".into()],
            ticks: 200,
            energy: 0,
        },
        RecipeEntry {
            id: "test_machine:crush_iron".into(),
            kind: "test_machine:resource_recipe".into(),
            inputs: vec![IngredientSlot {
                items: vec!["minecraft:raw_iron".into()],
                count: 1,
            }],
            outputs: vec!["minecraft:copper_ingot*1".into()],
            ticks: 80,
            energy: 100,
        },
    ]);
    mods.set_snapshot(player());
    assert!(mods.has_block_hook("test_machine:smelter"));
    assert!(!mods.has_block_hook("test_machine:unregistered"));
    assert!(mods.has_block_hook("test_machine:smelter[lit=false]"));

    // 1. Config read/write (API 0.3)
    assert!(mods.command("machine_api config"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "speed=2->5/reject=true/kept=5 bool=true->false float=1->1.5/reject=true mode=safe->fast/reject=true".into(),
            error: false,
        }]
    );
    assert!(mods.command("machine_api item_facts"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "remainder=minecraft:bucket".into(),
            error: false,
        }]
    );

    // 2. Keys, crosshair target, inventory-read, world-read, and render overlays (API 0.3 & 0.4)
    mods.set_key_state("vein_mine", true, true);
    mods.set_target(
        Some(BlockHit {
            x: 0,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            face: "north".into(),
            owner: "test_machine".into(),
        }),
        None,
    );
    mods.set_inventory(
        Some(StackView {
            slot: 0,
            id: "test_machine:wrench".into(),
            count: 1,
            damage: 0,
            max_damage: 250,
        }),
        None,
        Vec::new(),
        vec![StackView {
            slot: 0,
            id: "minecraft:iron_ingot".into(),
            count: 4,
            damage: 0,
            max_damage: 0,
        }],
    );
    mods.set_world_view(
        "minecraft:overworld",
        vec![WorldCell {
            x: 0,
            y: 64,
            z: 0,
            block: "test_machine:smelter[lit=false]".into(),
            biome: "minecraft:plains".into(),
            light: 0xF8,
        }],
    );
    mods.set_world_view(
        "test:moon",
        vec![WorldCell {
            x: 0,
            y: 64,
            z: 0,
            block: "test_machine:crate".into(),
            biome: "test:moon_surface".into(),
            light: 0,
        }],
    );
    assert!(mods.command("machine_api client_reads"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "key=true hit=test_machine:smelter[lit=false] hand=test_machine:wrench*1 container=minecraft:iron_ingot*4@0 blk=test_machine:smelter[lit=false] biome=minecraft:plains light=248".into(),
            error: false,
        }]
    );
    assert_eq!(mods.shapes().count(), 1);
    assert_eq!(mods.ghost_blocks().count(), 1);

    // 3. Capabilities (FE, fluid tanks, attachments, Curios accessories) & custom recipes (API 0.4)
    let (_capability_handle, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert!(mods.command("machine_api caps_and_recipes"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "fe=500/500/0/300/300/200/1000 fluid=1500/500/0/400/1000 mana=42 ring=test_machine:ring recipes=true/1/2/2/true/1/1 resource=true/1/0/0".into(),
            error: false,
        }]
    );
    mods.tick(1);
    assert!(mods.command("machine_api caps_and_recipes"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "fe=500/500/0/300/300/200/1000 fluid=1500/500/0/400/1000 mana=42 ring=test_machine:ring recipes=true/1/2/2/true/1/1 resource=true/1/0/0".into(),
            error: false,
        }]
    );

    // 4. Block, item, and server callbacks + server mutations + UI menus + net packets (API 0.3 & 0.4)
    let outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::UseWithoutItem,
            dimension: "minecraft:overworld".into(),
            x: 0,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: "Player".into(),
            held_item: String::new(),
        },
        "north".into(),
    );
    assert_eq!(outcome, EventOutcome::Consume);
    assert_eq!(mods.open_screens().count(), 1);
    let (_, backgrounds) = mods.open_screen_backgrounds().next().unwrap();
    assert_eq!(backgrounds.len(), 1);
    assert_eq!(backgrounds[0].texture, "test_machine:smelter");
    let (_, screen_slots) = mods.open_screen_slots().next().unwrap();
    assert_eq!(
        screen_slots.get(&2),
        Some(&("minecraft:iron_ingot".into(), 3))
    );
    let _ = mods.drain();

    mods.menu_click("test_machine:smelter_menu", 0, 1);
    let status_after_click = mods.entries()[0].status.clone();
    assert_eq!(
        mods.open_screens()
            .next()
            .map(|(_, spec)| spec.widgets[0].value),
        Some(50),
        "mod status after click: {:?}",
        status_after_click
    );
    mods.menu_button("test_machine:smelter_menu", 2);
    assert_eq!(mods.open_screens().count(), 0);
    assert_eq!(mods.open_screen_slots().count(), 0);
    assert_eq!(
        mods.drain(),
        [Output::ScreenClosed {
            mod_id: "test_machine".into()
        }]
    );

    let place_outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::Place,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: "Player".into(),
            held_item: "test_machine:smelter".into(),
        },
        "up".into(),
    );
    assert_eq!(place_outcome, EventOutcome::Pass);
    let scheduled_outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::ScheduledTick,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: String::new(),
            held_item: String::new(),
        },
        String::new(),
    );
    assert_eq!(scheduled_outcome, EventOutcome::Pass);
    let inside_outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::EntityInside,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: "TestPlayer".into(),
            held_item: String::new(),
        },
        String::new(),
    );
    assert_eq!(inside_outcome, EventOutcome::Pass);
    let neighbor_outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::NeighborChanged,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: String::new(),
            held_item: String::new(),
        },
        String::new(),
    );
    assert_eq!(neighbor_outcome, EventOutcome::Pass);
    let random_tick_outcome = mods.block_event(
        BlockEvent {
            action: BlockAction::RandomTick,
            dimension: "minecraft:overworld".into(),
            x: 1,
            y: 64,
            z: 0,
            state: "test_machine:smelter[lit=false]".into(),
            actor: String::new(),
            held_item: String::new(),
        },
        String::new(),
    );
    assert_eq!(random_tick_outcome, EventOutcome::Pass);

    let item_out = mods.item_event(ItemEvent {
        action: ItemAction::UseOnBlock,
        item: "test_machine:wrench".into(),
        player: "Player".into(),
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        target: "test_machine:smelter".into(),
    });
    assert_eq!(item_out, EventOutcome::Consume);
    assert_eq!(
        mods.drain(),
        [
            Output::Server(ServerAction::ReplaceBlockAndConsumeHeldItem {
                player: "Player".into(),
                dimension: "minecraft:overworld".into(),
                item: "test_machine:wrench".into(),
                x: 0,
                y: 64,
                z: 0,
                expected_block: "test_machine:smelter".into(),
                replacement: "test_machine:smelter[lit=true]".into(),
            }),
            Output::Server(ServerAction::SpawnDrop {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 64,
                z: 0,
                id: "minecraft:iron_ingot".into(),
                count: 2,
            }),
            Output::Server(ServerAction::SpawnMob {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 64,
                z: 0,
                entity: "minecraft:iron_golem".into(),
            }),
        ]
    );
    assert!(mods.command("machine_api logic_events"));
    let consume_feedback = mods.drain();
    let [Output::Feedback { text, error: false }] = consume_feedback.as_slice() else {
        panic!("expected item-consumption feedback, got {consume_feedback:?}");
    };
    assert!(
        text.contains("item=use-on-block:test_machine:wrench:minecraft:overworld:0,64,0:test_machine:smelter:consume=true/false"),
        "{text}"
    );
    assert!(mods.has_any_block_hooks());
    assert!(!Mods::default().has_any_block_hooks());
    assert_eq!(
        mods.block_event(
            BlockEvent {
                action: BlockAction::EntityInside,
                state: "test_machine:smelter[lit=false]".into(),
                dimension: "minecraft:overworld".into(),
                x: -31,
                y: 70,
                z: -9,
                actor: "minecraft:item/42".into(),
                held_item: String::new(),
            },
            String::new(),
        ),
        EventOutcome::Pass
    );
    assert_eq!(
        mods.item_event(ItemEvent {
            action: ItemAction::UseOnBlock,
            item: "minecraft:spyglass".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -31,
            y: 70,
            z: -9,
            target: "minecraft:stone".into(),
        }),
        EventOutcome::Pass
    );
    assert_eq!(
        mods.item_event(ItemEvent {
            action: ItemAction::UseInAir,
            item: "minecraft:spyglass".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -31,
            y: 70,
            z: -9,
            target: String::new(),
        }),
        EventOutcome::Pass
    );
    assert_eq!(
        mods.item_event(ItemEvent {
            action: ItemAction::FinishUsing,
            item: "minecraft:spyglass".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -31,
            y: 70,
            z: -9,
            target: String::new(),
        }),
        EventOutcome::Pass
    );
    assert_eq!(
        mods.item_event(ItemEvent {
            action: ItemAction::AttackEntity,
            item: "test_machine:wrench".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -36,
            y: 70,
            z: -10,
            target: "test_machine:automaton".into(),
        }),
        EventOutcome::Cancel
    );
    assert_eq!(
        mods.item_entity_interaction_event(ItemEvent {
            action: ItemAction::InteractEntity,
            item: "test_machine:wrench".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -36,
            y: 70,
            z: -10,
            target: "minecraft:wolf;tamed=false;owner_is_player=false".into(),
        }),
        Some(EventOutcome::Pass)
    );
    assert_eq!(
        mods.item_entity_interaction_event(ItemEvent {
            action: ItemAction::InteractEntity,
            item: "other_mod:unknown".into(),
            player: "Player".into(),
            dimension: "minecraft:overworld".into(),
            x: -36,
            y: 70,
            z: -10,
            target: "minecraft:wolf;tamed=true;owner_is_player=true".into(),
        }),
        None
    );
    assert!(mods.command("machine_api logic_events"));
    let logic_feedback = mods.drain();
    assert_eq!(logic_feedback.len(), 1);
    let Output::Feedback { text, error: false } = &logic_feedback[0] else {
        panic!("expected logic_events feedback, got {logic_feedback:?}");
    };
    assert!(
        text.contains(
            "block=entity-inside:test_machine:smelter[lit=false]:-31,70,-9:minecraft:item/42"
        ),
        "{text}"
    );
    assert!(
        text.contains("item=interact-entity:test_machine:wrench:minecraft:overworld:-36,70,-10:minecraft:wolf;tamed=false;owner_is_player=false"),
        "{text}"
    );
    assert!(text.contains("air_fallback=true finished=true"), "{text}");

    let srv_out = mods.server_event(ServerEvent {
        kind: ServerEventKind::PlayerJoin,
        subject: "Player".into(),
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        detail: String::new(),
    });
    assert_eq!(srv_out, EventOutcome::Consume);
    assert_eq!(
        mods.drain(),
        [Output::Server(ServerAction::SetRule {
            name: "keepInventory".into(),
            value: "true".into(),
        })]
    );
    for (kind, name) in [
        (ServerEventKind::PlayerLeave, "player-leave"),
        (ServerEventKind::BlockBreak, "block-break"),
        (ServerEventKind::BlockPlace, "block-place"),
        (ServerEventKind::EntitySpawn, "entity-spawn"),
        (ServerEventKind::EntityDeath, "entity-death"),
    ] {
        assert_eq!(
            mods.server_event(ServerEvent {
                kind,
                subject: "Player".into(),
                dimension: "minecraft:overworld".into(),
                x: 1,
                y: 64,
                z: 0,
                detail: "test_machine:smelter".into(),
            }),
            EventOutcome::Pass
        );
        assert!(mods.command("machine_api last_server_event"));
        assert_eq!(
            mods.drain(),
            [Output::Feedback {
                text: format!("{name}:Player:minecraft:overworld:1,64,0:test_machine:smelter"),
                error: false,
            }]
        );
    }

    assert!(mods.command("machine_api set_other_dimension"));
    assert_eq!(
        mods.drain(),
        [
            Output::Server(ServerAction::SetBlock {
                dimension: "minecraft:the_nether".into(),
                x: 1,
                y: 64,
                z: 1,
                state: "minecraft:gold_block".into(),
            }),
            Output::Feedback {
                text: "requested block in the Nether".into(),
                error: false,
            },
        ]
    );
    assert!(mods.command("machine_api schedule_test_tick"));
    assert_eq!(
        mods.drain(),
        [
            Output::Server(ServerAction::SetBlock {
                dimension: "minecraft:overworld".into(),
                x: -31,
                y: 70,
                z: -9,
                state: "minecraft:composter[level=7]".into(),
            }),
            Output::Server(ServerAction::ScheduleTick {
                dimension: "minecraft:overworld".into(),
                x: -31,
                y: 70,
                z: -9,
                block: "minecraft:composter".into(),
                delay: 1,
            }),
            Output::Feedback {
                text: "scheduled composter tick".into(),
                error: false,
            },
        ]
    );
    assert!(mods.command("machine_api set_machine_block"));
    assert_eq!(
        mods.drain(),
        [
            Output::Server(ServerAction::SetBlock {
                dimension: "minecraft:overworld".into(),
                x: -31,
                y: 70,
                z: -9,
                state: "test_machine:smelter".into(),
            }),
            Output::Feedback {
                text: "requested test machine block".into(),
                error: false,
            },
        ]
    );

    mods.deliver_packet("test_machine:sync", b"hello", Some("Alice"));
    assert_eq!(
        mods.drain(),
        [Output::PacketToServer {
            mod_id: "test_machine".into(),
            channel: "test_machine:sync".into(),
            payload: b"ack".to_vec(),
        }]
    );
    assert!(mods.command("machine_api state"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "pkt=test_machine:sync:Alice:hello click=Some((0, 1)) btn=Some(2)".into(),
            error: false,
        }]
    );
}

#[test]
fn api_0_5_deep_entities_models_dimensions_and_vanilla_hooks() {
    let mut mods = machines();
    mods.set_snapshot(player());
    assert!(mods.has_vanilla_hook_subscription(&VanillaHookKind::CropGrow));
    assert!(!Mods::default().has_vanilla_hook_subscription(&VanillaHookKind::CropGrow));

    // 1. Custom entity types, lifecycle, ticking, saving, interaction & hurt callbacks
    let entity_types: Vec<(&str, f32)> = mods
        .entity_types()
        .map(|(_, t)| (t.id.as_str(), t.max_health))
        .collect();
    assert_eq!(entity_types, [("test_machine:automaton", 40.0)]);
    let h = mods.create_entity("test_machine:automaton", &[]).unwrap();
    assert_eq!(mods.entity_state(h).map(|s| s.health), Some(40.0));
    mods.tick_entities(&[h]);
    assert_eq!(
        mods.model_animation(&format!("entity:{h}")),
        Some(("walk".into(), true, 1.0))
    );
    let saved = mods.save_entity(h).unwrap();
    assert_eq!(saved, 1u32.to_le_bytes());
    assert!(mods.entity_interact(h, "Player", "test_machine:wrench"));
    assert_eq!(mods.entity_state(h).map(|s| s.health), Some(20.0));
    let changes = mods.take_entity_changes();
    assert_eq!(changes.health, [(h, 20.0)]);
    assert_eq!(changes.navigation, [(h, 10.0, 64.0, 10.0, 0.5)]);
    assert!(mods.entity_hurt(h, 5.0, "player"));
    mods.remove_entity(h);
    assert!(mods.entity_state(h).is_none());

    // 2. Entities host API, animated models, draw lists, dimensions & portals
    assert_eq!(mods.dimensions().len(), 2);
    assert!(
        mods.dimensions()
            .iter()
            .any(|d| d.id == "test_machine:twilight")
    );
    assert!(
        mods.dimensions()
            .iter()
            .any(|d| d.id == "test_machine:noisy")
    );
    assert_eq!(mods.portals().len(), 1);
    assert_eq!(mods.portals()[0].id, "test_machine:twilight_portal");

    assert!(mods.command("machine_api deep_api"));
    assert_eq!(
        mods.drain(),
        [
            Output::Teleported {
                subject: "Player".into(),
                dimension: "test_machine:twilight".into(),
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
            Output::Feedback {
                text: "spawned=2 near=1 dims=5".into(),
                error: false,
            },
        ]
    );
    assert_eq!(mods.model_bones("block:0,64,0").len(), 1);
    assert_eq!(mods.model_draw_list("block:0,64,0").len(), 1);

    // 3. Named vanilla hooks (loot, damage and experience)
    let (outcome, dmg_mult, _, _, loot) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::LootDrop,
        subject: "minecraft:stone".into(),
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        value: 1.0,
        context: "minecraft:diamond_pickaxe".into(),
    });
    assert_eq!(outcome, EventOutcome::Consume);
    assert_eq!(dmg_mult, 1.0);
    assert_eq!(
        loot,
        [ItemDrop {
            id: "minecraft:diamond".into(),
            count: 1,
        }]
    );

    let (outcome2, dmg_mult2, _, _, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::EntityDamage,
        subject: "Player".into(),
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        value: 10.0,
        context: "minecraft:zombie".into(),
    });
    assert_eq!(outcome2, EventOutcome::Consume);
    assert_eq!(dmg_mult2, 0.5);

    let (outcome3, _, xp_mult, _, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::XpGain,
        subject: "minecraft:player".into(),
        dimension: "minecraft:overworld".into(),
        x: 0,
        y: 64,
        z: 0,
        value: 5.0,
        context: "minecraft:experience_orb".into(),
    });
    assert_eq!(outcome3, EventOutcome::Consume);
    assert_eq!(xp_mult, 2.0);

    let (outcome4, crop_growth) = mods.crop_growth_hook(&VanillaHookEvent {
        kind: VanillaHookKind::CropGrow,
        subject: "minecraft:wheat[age=2]".into(),
        dimension: "minecraft:overworld".into(),
        x: 1,
        y: 64,
        z: 1,
        value: 3.0,
        context: "minecraft:random_tick".into(),
    });
    assert_eq!(outcome4, EventOutcome::Pass);
    assert_eq!(crop_growth, 3.0);

    let (outcome5, _, _, _, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::ExplosionDetonate,
        subject: "minecraft:stone".into(),
        dimension: "minecraft:overworld".into(),
        x: 2,
        y: 64,
        z: -1,
        value: 4.0,
        context: "minecraft:explosion;Destroy;fire=false".into(),
    });
    assert_eq!(outcome5, EventOutcome::Cancel);

    let (outcome6, _, _, looting_bonus, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::EnchantmentLevel,
        subject: "minecraft:looting".into(),
        dimension: "minecraft:overworld".into(),
        x: 2,
        y: 64,
        z: -1,
        value: 1.0,
        context: "minecraft:diamond_sword".into(),
    });
    assert_eq!(outcome6, EventOutcome::Pass);
    assert_eq!(looting_bonus, 2);

    let (outcome7, _, _, consumed_bonus, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::EnchantmentLevel,
        subject: "minecraft:fortune".into(),
        dimension: "minecraft:overworld".into(),
        x: 2,
        y: 64,
        z: -1,
        value: 1.0,
        context: "minecraft:iron_pickaxe".into(),
    });
    assert_eq!(outcome7, EventOutcome::Consume);
    assert_eq!(consumed_bonus, 4);

    let (outcome8, _, _, cancelled_bonus, _) = mods.vanilla_hook(&VanillaHookEvent {
        kind: VanillaHookKind::EnchantmentLevel,
        subject: "minecraft:silk_touch".into(),
        dimension: "minecraft:overworld".into(),
        x: 2,
        y: 64,
        z: -1,
        value: 1.0,
        context: "minecraft:diamond_pickaxe".into(),
    });
    assert_eq!(outcome8, EventOutcome::Cancel);
    assert_eq!(cancelled_bonus, 0);
}

#[test]
fn config_overrides_persist_and_validate_on_reload() {
    let dir = mods_dir().join("config_reload");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("machine.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_machine.hmod"), &path).unwrap();

    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    assert!(mods.command("machine_api config"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "speed=2->5/reject=true/kept=5 bool=true->false float=1->1.5/reject=true mode=safe->fast/reject=true".into(),
            error: false,
        }]
    );

    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    assert!(mods.command("machine_api read_config"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "speed=5 bool=false float=1.5 mode=fast".into(),
            error: false,
        }]
    );

    let config_file = dir
        .join(".horizon-mod-data")
        .join("test_machine.config.json");
    std::fs::write(
        &config_file,
        br#"{"speed":999,"enabled":false,"scale":1.25,"mode":"bogus"}"#,
    )
    .unwrap();
    let restarted = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    let mut restarted = restarted;
    assert!(restarted.command("machine_api read_config"));
    assert_eq!(
        restarted.drain(),
        [Output::Feedback {
            text: "speed=2 bool=false float=1.25 mode=safe".into(),
            error: false,
        }]
    );
}

#[test]
fn item_slot_capabilities_bridge_with_block_entity_views_and_reload() {
    let dir = mods_dir().join("slot_bridge_reload");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("machine.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_machine.hmod"), &path).unwrap();

    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    mods.set_item_facts(std::sync::Arc::new(Facts));
    let (h, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert!(mods.has_block_entity(h));
    assert!(mods.command("machine_api slot_cap_bridge"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "slots=0=minecraft:raw_iron*2".into(),
            error: false,
        }]
    );

    let mut views = [smelter(h, None)];
    mods.tick_block_entities(&mut views);
    assert_eq!(views[0].slots, [Some(stack("minecraft:raw_iron", 2)), None]);
    assert_eq!(mods.block_entity_data(h), [1]);

    // Hot-reload the mod and verify that re-binding the block entity at the
    // same position restores its saved progress automatically.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    mods.set_item_facts(std::sync::Arc::new(Facts));
    assert!(!mods.has_block_entity(h));
    let (rebound, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_eq!(mods.block_entity_data(rebound), [1]);
    views[0].handle = rebound;
    mods.tick_block_entities(&mut views);
    mods.tick_block_entities(&mut views);
    assert_eq!(
        views[0].slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:iron_ingot", 1))
        ]
    );
    assert!(mods.command("machine_api read_slot_caps"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "slots=0=minecraft:raw_iron*1,1=minecraft:iron_ingot*1".into(),
            error: false,
        }]
    );
}

#[test]
fn entity_despawn_cleans_attachments_and_accessories_snapshot_restores() {
    let mut mods = machines();
    let h = mods.create_entity("test_machine:automaton", &[]).unwrap();
    mods.tick_entities(&[h]);
    assert!(mods.model_animation(&format!("entity:{h}")).is_some());
    mods.restore_attachments(vec![(
        format!("entity:{h}"),
        "test_machine:tag".into(),
        vec![7],
    )]);
    assert_eq!(mods.attachment_snapshot().len(), 1);
    mods.remove_entity(h);
    assert!(mods.model_animation(&format!("entity:{h}")).is_none());
    assert!(mods.attachment_snapshot().is_empty());

    let (_be_handle, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert!(mods.command("machine_api caps_and_recipes"));
    let _ = mods.drain();
    let saved_accessories = mods.accessory_snapshot();
    assert_eq!(saved_accessories.len(), 1);
    mods.clear_accessories();
    assert!(mods.command("machine_api get_ring"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "ring=none reject_stack=true reject_idx=true".into(),
            error: false,
        }]
    );
    mods.restore_accessories(saved_accessories);
    assert!(mods.command("machine_api get_ring"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "ring=test_machine:ring*1 reject_stack=true reject_idx=true".into(),
            error: false,
        }]
    );
}

#[test]
fn models_sanitize_clear_and_cleanup_on_entity_and_block_removal() {
    let mut mods = machines();
    let entity_handle = mods.create_entity("test_machine:automaton", &[]).unwrap();
    let (be_handle, _) = mods
        .create_block_entity_at(
            "test_machine:smelter",
            &[],
            Some(("minecraft:overworld".into(), -31, 70, -9)),
        )
        .unwrap();

    assert!(mods.command("machine_api set_models"));
    let _ = mods.drain();

    let entity_target = format!("entity:{entity_handle}");
    let block_target = "block:minecraft:overworld:-31,70,-9";

    assert_eq!(
        mods.model_animation(&entity_target),
        Some(("spin".into(), true, 2.0))
    );
    assert_eq!(mods.model_bones(&entity_target).len(), 1);
    assert_eq!(mods.model_bones(&entity_target)[0].bone, "head");
    assert_eq!(mods.model_draw_list(&entity_target).len(), 1);

    // NaN animation speed is sanitized to 1.0; empty bone/asset entries are filtered;
    // NaN/zero/infinite bone pose floats are sanitized to identity defaults.
    assert_eq!(
        mods.model_animation(block_target),
        Some(("smelt_spin".into(), true, 1.0))
    );
    let block_bones = mods.model_bones(block_target);
    assert_eq!(block_bones.len(), 1);
    assert_eq!(block_bones[0].bone, "rotor");
    assert_eq!(
        [
            block_bones[0].pos_x,
            block_bones[0].pos_y,
            block_bones[0].pos_z
        ],
        [0.0, 1.0, 0.0]
    );
    assert_eq!(
        [
            block_bones[0].rot_x,
            block_bones[0].rot_y,
            block_bones[0].rot_z
        ],
        [0.0, 45.0, 0.0]
    );
    assert_eq!(
        [
            block_bones[0].scale_x,
            block_bones[0].scale_y,
            block_bones[0].scale_z
        ],
        [1.0, 1.5, 1.0]
    );
    let block_draws = mods.model_draw_list(block_target);
    assert_eq!(block_draws.len(), 4);

    assert!(mods.model_animations().contains_key(&entity_target));
    assert!(mods.model_all_bones().contains_key(block_target));
    assert!(mods.model_all_draw_lists().contains_key(block_target));

    // Removing the entity clears its animations, bone poses, and draw lists.
    mods.remove_entity(entity_handle);
    assert!(mods.model_animation(&entity_target).is_none());
    assert!(mods.model_bones(&entity_target).is_empty());
    assert!(mods.model_draw_list(&entity_target).is_empty());

    // Explicit empty values clear the target's model entries.
    assert!(mods.command("machine_api clear_models"));
    let _ = mods.drain();
    assert!(mods.model_animation(block_target).is_none());
    assert!(mods.model_bones(block_target).is_empty());
    assert!(mods.model_draw_list(block_target).is_empty());

    // Re-populate block models and verify block-entity removal cleans them up.
    assert!(mods.command("machine_api set_models"));
    let _ = mods.drain();
    assert!(mods.model_animation(block_target).is_some());
    assert!(!mods.model_bones(block_target).is_empty());
    assert!(!mods.model_draw_list(block_target).is_empty());
    mods.remove_block_entity(be_handle);
    assert!(mods.model_animation(block_target).is_none());
    assert!(mods.model_bones(block_target).is_empty());
    assert!(mods.model_draw_list(block_target).is_empty());
}

#[test]
fn api_0_3_target_entity_inventory_world_view_and_server_actions() {
    let mut mods = machines();
    mods.set_snapshot(Snapshot {
        player: Some(Player {
            x: 0.5,
            y: 70.0,
            z: 0.5,
            yaw: 0.0,
            pitch: 0.0,
            health: 20.0,
            food: 20,
            dimension: "minecraft:overworld".into(),
        }),
        clock: None,
    });
    mods.set_target(
        Some(BlockHit {
            x: 0,
            y: 70,
            z: 2,
            state: "minecraft:chest[facing=north,type=single,waterlogged=false]".into(),
            face: "north".into(),
            owner: "minecraft".into(),
        }),
        Some(horizon_mods::EntityHit {
            kind: "test_machine:automaton".into(),
            health: 36.5,
            max_health: 40.0,
            name: "Automaton".into(),
        }),
    );
    mods.set_inventory(
        Some(StackView {
            slot: 0,
            id: "test_machine:wrench".into(),
            count: 1,
            damage: 3,
            max_damage: 250,
        }),
        Some(StackView {
            slot: 40,
            id: "minecraft:shield".into(),
            count: 1,
            damage: 12,
            max_damage: 336,
        }),
        vec![
            StackView {
                slot: 0,
                id: "test_machine:wrench".into(),
                count: 1,
                damage: 3,
                max_damage: 250,
            },
            StackView {
                slot: 36,
                id: "minecraft:diamond_boots".into(),
                count: 1,
                damage: 0,
                max_damage: 429,
            },
            StackView {
                slot: 39,
                id: "minecraft:diamond_helmet".into(),
                count: 1,
                damage: 0,
                max_damage: 363,
            },
            StackView {
                slot: 40,
                id: "minecraft:shield".into(),
                count: 1,
                damage: 12,
                max_damage: 336,
            },
        ],
        vec![
            StackView {
                slot: 0,
                id: "minecraft:raw_iron".into(),
                count: 8,
                damage: 0,
                max_damage: 0,
            },
            StackView {
                slot: 1,
                id: "minecraft:iron_ingot".into(),
                count: 4,
                damage: 0,
                max_damage: 0,
            },
        ],
    );

    assert!(mods.command("machine_api api_0_3_reads"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "block=minecraft:chest[facing=north,type=single,waterlogged=false]@0,70,2:north:minecraft entity=test_machine:automaton:36.5/40.0:Automaton main=0:test_machine:wrench*1(3/250) off=40:minecraft:shield*1(12/336) inv_len=4 armor=[36:minecraft:diamond_boots*1,39:minecraft:diamond_helmet*1] open=[0:minecraft:raw_iron*8,1:minecraft:iron_ingot*4]".into(),
            error: false,
        }]
    );

    // Populate world-read neighborhood and verify bounds + stale-cell eviction on move/dimension change.
    mods.set_world_view(
        "minecraft:overworld",
        vec![
            horizon_mods::WorldCell {
                x: 0,
                y: 69,
                z: 0,
                block: "minecraft:stone".into(),
                biome: "minecraft:plains".into(),
                light: 15,
            },
            horizon_mods::WorldCell {
                x: 0,
                y: 70,
                z: 0,
                block: "minecraft:air".into(),
                biome: "minecraft:plains".into(),
                light: 15,
            },
        ],
    );
    assert!(mods.command("machine_api api_0_3_world"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "at=0,70,0 below=minecraft:stone biome=minecraft:plains light=15 far_block=none far_biome=none low=none high=none".into(),
            error: false,
        }]
    );

    // Replacing the world view after player movement evicts the previous cell at (0, 69, 0).
    mods.set_world_view(
        "minecraft:overworld",
        vec![horizon_mods::WorldCell {
            x: 20,
            y: 69,
            z: 20,
            block: "minecraft:grass_block[snowy=false]".into(),
            biome: "minecraft:forest".into(),
            light: 14,
        }],
    );
    assert!(mods.command("machine_api api_0_3_world probe 0 69 0"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "probe=0,69,0 block=none biome=none light=none".into(),
            error: false,
        }]
    );

    // Server play-sound and invalid entity/dimension actions.
    assert!(
        mods.command("machine_api api_0_3_server sound_and_invalid minecraft:overworld 0 70 0")
    );
    assert_eq!(
        mods.drain(),
        [
            Output::Server(ServerAction::PlaySound {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 70,
                z: 0,
                sound: "minecraft:block.note_block.pling".into(),
                volume: 1.0,
                pitch: 1.0,
            }),
            Output::Server(ServerAction::SpawnMob {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 70,
                z: 0,
                entity: "minecraft:not_a_mob".into(),
            }),
            Output::Server(ServerAction::SetBlock {
                dimension: "test_machine:unknown_dim".into(),
                x: 0,
                y: 70,
                z: 0,
                state: "minecraft:stone".into(),
            }),
            Output::Feedback {
                text: "sound=true bad_mob_rejected=true unknown_mob_queued=true unknown_dim_queued=true".into(),
                error: false,
            },
        ]
    );
}

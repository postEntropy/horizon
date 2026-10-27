//! Loads real mods built with the SDK: the coords-hud example and a guest
//! that misbehaves on command. Needs `rustup target add wasm32-unknown-unknown`.
use horizon_mods::{
    BlockAction, BlockEntityView, BlockEvent, BlockHit, Clock, CookingKind, CookingRecipe, Corner,
    EventOutcome, ItemAction, ItemDrop, ItemEvent, ItemFacts, Mods, Output, Package, Player,
    Property, ServerAction, ServerEvent, ServerEventKind, Snapshot, Stack, StackView, Status,
    VanillaHookEvent, VanillaHookKind, manifest::componentize,
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
capabilities = ["chat", "storage"]
"#;

const MACHINE_MANIFEST: &str = r#"
id = "test_machine"
name = "Test Machine"
version = "0.1.0"
api = "0.5"
side = "both"
capabilities = ["config", "server", "keys", "target", "inventory-read", "capabilities", "ui", "net", "recipes", "world-read", "render", "entities", "models", "dimensions", "vanilla-hooks", "block-entities"]

[config.speed]
type = "int"
default = 2
min = 1
max = 10
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
        ];
        for (name, manifest) in mods {
            let core = std::fs::read(built.join(format!("{name}.wasm"))).unwrap();
            let hmod = Package::write(&manifest, &componentize(&core).unwrap()).unwrap();
            std::fs::write(dir.join(format!("{name}.hmod")), hmod).unwrap();
        }
        dir
    })
}

fn load(name: &str) -> Mods {
    Mods::load(&[mods_dir().join(name)], |_| true, |c| c == "give")
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
    fn max_stack_size(&self, id: &str) -> u32 {
        if id == "test:ghost" { 0 } else { 64 }
    }
    fn burn_time(&self, _id: &str) -> u32 {
        0
    }
    fn remainder(&self, _id: &str) -> Option<String> {
        None
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
    mods.set_item_facts(std::sync::Arc::new(Facts));
    mods
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
            ("test_machine:crate", 4, false)
        ]
    );
    assert_eq!(mods.create_block_entity("minecraft:furnace", &[]), None);
    let (h, slots) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    assert_eq!(slots, 2);
    let mut views = [smelter(h, Some(stack("minecraft:raw_iron", 2)))];
    mods.tick_block_entities(&mut views);
    assert!(views[0].changed, "lit turned on");
    assert_eq!(views[0].properties[0].value, "true");
    assert_eq!(mods.block_entity_data(h), [1]);
    views[0].changed = false;
    mods.tick_block_entities(&mut views);
    assert!(!views[0].changed, "nothing the game keeps changed");
    mods.tick_block_entities(&mut views);
    assert_eq!(
        views[0].slots,
        [
            Some(stack("minecraft:raw_iron", 1)),
            Some(stack("minecraft:iron_ingot", 1))
        ]
    );
    mods.tick_block_entities(&mut views);
    assert_eq!(mods.block_entity_data(h), [1]);

    // Saved and loaded again (a chunk unloads and comes back).
    let saved = mods.save_block_entity(h).unwrap();
    mods.remove_block_entity(h);
    assert_eq!(mods.block_entity_data(h), Vec::<i32>::new());
    let (again, _) = mods
        .create_block_entity("test_machine:smelter", &saved)
        .unwrap();
    assert_ne!(again, h);
    assert_eq!(mods.block_entity_data(again), [1]);
    assert!(mods.drain().is_empty());
}

#[test]
fn block_entities_see_only_their_own() {
    let mut mods = machines();
    let (h, _) = mods
        .create_block_entity("test_machine:smelter", &[])
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
    assert_eq!(mods.entries()[0].status, Status::Running);
}

#[test]
fn block_entities_need_their_capability_and_blocks() {
    let wasm = Package::read(&mods_dir().join("horizon_mods_test_machine.hmod"))
        .unwrap()
        .wasm
        .unwrap();
    let cases = [
        (
            MACHINE_MANIFEST.replace("\"block-entities\"", ""),
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
fn a_block_entity_that_traps_gives_its_slots_back() {
    let mut mods = machines();
    let (h, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut views = [smelter(h, Some(stack("minecraft:tnt", 3)))];
    mods.tick_block_entities(&mut views);
    assert!(matches!(mods.entries()[0].status, Status::Failed(_)));
    assert!(matches!(mods.drain()[..], [Output::Stopped { .. }]));
    // Its slots are back, with what it wrote before it stopped.
    assert_eq!(views[0].handle, h);
    assert_eq!(views[0].slots, [Some(stack("minecraft:tnt", 3)), None]);
    assert_eq!(views[0].properties[0].value, "true");
    // A stopped mod's block entities are left alone after that.
    mods.tick_block_entities(&mut views);
    assert_eq!(views[0].slots, [Some(stack("minecraft:tnt", 3)), None]);
    assert_eq!(mods.save_block_entity(h), None);
}

#[test]
fn api_0_3_and_0_4_logic_and_machine_hooks_run() {
    let mut mods = machines();
    mods.set_snapshot(player());

    // 1. Config read/write (API 0.3)
    assert!(mods.command("machine_api config"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "speed 2->5".into(),
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
        Vec::new(),
    );
    mods.set_world_block("minecraft:overworld", 0, 64, 0, "test_machine:smelter");
    mods.set_world_biome(0, 64, 0, "minecraft:plains");
    mods.set_world_light(0, 64, 0, 0xF8);
    assert!(mods.command("machine_api client_reads"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "key=true hit=test_machine:smelter[lit=false] hand=test_machine:wrench*1 blk=test_machine:smelter biome=minecraft:plains light=248".into(),
            error: false,
        }]
    );
    assert_eq!(mods.shapes().count(), 1);
    assert_eq!(mods.ghost_blocks().count(), 1);

    // 3. Capabilities (FE, fluid tanks, attachments, Curios accessories) & custom recipes (API 0.4)
    assert!(mods.command("machine_api caps_and_recipes"));
    assert_eq!(
        mods.drain(),
        [Output::Feedback {
            text: "fe=500/300/1200 fluid=1500/500 mana=42 ring=test_machine:ring recipes=1".into(),
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
    let _ = mods.drain();

    mods.menu_click("test_machine:smelter_menu", 0, 1);
    assert_eq!(mods.open_screens().next().unwrap().1.widgets[0].value, 50);
    mods.menu_button("test_machine:smelter_menu", 2);
    assert_eq!(mods.open_screens().count(), 0);
    assert_eq!(
        mods.drain(),
        [Output::ScreenClosed {
            mod_id: "test_machine".into()
        }]
    );

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
            Output::Server(ServerAction::SetBlock {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 64,
                z: 0,
                state: "test_machine:smelter[lit=true]".into(),
            }),
            Output::Server(ServerAction::SpawnDrop {
                dimension: "minecraft:overworld".into(),
                x: 0,
                y: 64,
                z: 0,
                id: "minecraft:iron_ingot".into(),
                count: 2,
            }),
        ]
    );

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

    // 1. Custom entity types, lifecycle, ticking, saving, interaction & hurt callbacks
    let entity_types: Vec<(&str, f32)> = mods
        .entity_types()
        .map(|(_, t)| (t.id.as_str(), t.max_health))
        .collect();
    assert_eq!(entity_types, [("test_machine:automaton", 40.0)]);
    let h = mods.create_entity("test_machine:automaton", &[]).unwrap();
    mods.tick_entities(&[h]);
    assert_eq!(
        mods.model_animation(&format!("entity:{h}")),
        Some(("walk".into(), true, 1.0))
    );
    let saved = mods.save_entity(h).unwrap();
    assert_eq!(saved, 1u32.to_le_bytes());
    assert!(mods.entity_interact(h, "Player", "test_machine:wrench"));
    assert!(mods.entity_hurt(h, 5.0, "player"));
    mods.remove_entity(h);

    // 2. Entities host API, animated models, draw lists, dimensions & portals
    assert_eq!(mods.dimensions().len(), 1);
    assert_eq!(mods.dimensions()[0].id, "test_machine:twilight");
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
                text: "spawned=1 near=1 dims=4".into(),
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
}

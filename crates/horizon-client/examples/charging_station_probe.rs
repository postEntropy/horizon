//! Actual Charging Gadgets WASM + a test battery provider/consumer WASM.
//! PROVIDER_HMOD CHARGER_MANIFEST CHARGER_CORE_WASM OUTPUT_JSON
use horizon_client::{item_data, items::ItemStack, save};
use horizon_mods::{
    BlockEntityView, CookingKind, CookingRecipe, ItemFacts, Mods, Package, Stack, Status,
    manifest::componentize,
};
use horizon_world::nbt::{Limits, Tag, read_network, write_network};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Facts;
impl ItemFacts for Facts {
    fn max_stack_size(&self, _: &str) -> u32 {
        64
    }
    fn burn_time(&self, id: &str) -> u32 {
        if id == "minecraft:coal" { 1600 } else { 0 }
    }
    fn remainder(&self, _: &str) -> Option<String> {
        None
    }
    fn cooking_recipe(&self, _: CookingKind, _: &str) -> Option<CookingRecipe> {
        None
    }
}
fn load(root: &Path) -> Mods {
    let mut mods = Mods::load(
        &[root.join("provider.hmod"), root.join("charger.hmod")],
        |_| true,
        |_| false,
    );
    for entry in mods.entries() {
        assert_eq!(entry.status, Status::Running);
    }
    mods.set_item_facts(Arc::new(Facts));
    mods
}
fn view(mods: &mut Mods, saved: &[u8]) -> BlockEntityView {
    let (handle, slots) = mods
        .create_block_entity_at(
            "charginggadgets:charging_station",
            saved,
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    BlockEntityView {
        handle,
        dimension: "minecraft:overworld".into(),
        y: 64,
        slots: vec![None; slots as usize],
        slot_components: vec![Vec::new(); slots as usize],
        ..Default::default()
    }
}
fn seed(energy: u32) -> Vec<u8> {
    [energy.to_le_bytes(), 0u32.to_le_bytes(), 0u32.to_le_bytes()].concat()
}
fn energy(mods: &mut Mods, handle: u32) -> u32 {
    let state = mods.save_block_entity(handle).unwrap();
    u32::from_le_bytes(state[..4].try_into().unwrap())
}
fn stored(view: &BlockEntityView) -> i64 {
    stored_at(view, 1)
}
fn stored_at(view: &BlockEntityView, slot: usize) -> i64 {
    if view.slot_components[slot].is_empty() {
        return 0;
    }
    let (tag, _) = read_network(&view.slot_components[slot], Limits::default()).unwrap();
    match tag.get("horizon:energy") {
        Some(Tag::Long(v)) => *v,
        None => 0,
        _ => panic!("invalid energy"),
    }
}
fn tick(mods: &mut Mods, view: &mut BlockEntityView) {
    mods.tick_block_entities(std::slice::from_mut(view));
    for entry in mods.entries() {
        assert_eq!(entry.status, Status::Running);
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 && args.len() != 5 {
        return Err("PROVIDER_HMOD CHARGER_MANIFEST CHARGER_CORE_WASM OUTPUT_JSON".into());
    }
    let root = std::env::temp_dir().join(format!("horizon-charge-probe-{}", std::process::id()));
    fs::create_dir(&root)?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::copy(&args[0], root.join("provider.hmod"))?;
    fs::write(
        root.join("charger.hmod"),
        Package::write(
            &fs::read_to_string(&args[1])?,
            &componentize(&fs::read(&args[2])?)?,
        )?,
    )?;
    let mut mods = load(&root);
    let mut station = view(&mut mods, &seed(3000));
    station.slots[1] = Some(Stack {
        id: "test_machine:battery".into(),
        count: 1,
    });
    station.slot_components[1] = write_network(
        &Tag::compound([("minecraft:custom_name", Tag::String("Battery A".into()))]),
        Limits::default(),
    )?;
    for expected in [625, 1250, 1875, 2500, 2500] {
        tick(&mut mods, &mut station);
        assert_eq!(stored(&station), expected);
        assert_eq!(
            energy(&mut mods, station.handle) as i64 + stored(&station),
            3000
        );
    }
    // Real item codecs and an on-disk snapshot, followed by a fresh WASM runtime.
    let (components, _) = read_network(&station.slot_components[1], Limits::default())?;
    let mut battery = ItemStack::new("test_machine:battery", 1);
    (battery.damage, battery.extra) = item_data::decode_components(Some(&components));
    let snapshot = Tag::compound([
        (
            "station",
            Tag::ByteArray(mods.save_block_entity(station.handle).unwrap()),
        ),
        ("item", save::item_to_nbt(&battery)),
    ]);
    fs::write(
        root.join("save.nbt"),
        write_network(&snapshot, Limits::default())?,
    )?;
    drop(mods);
    let (snapshot, _) = read_network(&fs::read(root.join("save.nbt"))?, Limits::default())?;
    let Tag::ByteArray(saved) = snapshot.get("station").unwrap() else {
        panic!("station save");
    };
    let battery = save::item_from_nbt(snapshot.get("item").unwrap()).unwrap();
    let components = item_data::encode_components(&battery).unwrap();
    assert_eq!(
        components.get("minecraft:custom_name"),
        Some(&Tag::String("Battery A".into()))
    );
    let mut mods = load(&root);
    let mut station = view(&mut mods, saved);
    station.slots[1] = Some(Stack {
        id: battery.item.to_string(),
        count: battery.count,
    });
    station.slot_components[1] = write_network(&components, Limits::default())?;
    tick(&mut mods, &mut station);
    assert_eq!(
        (energy(&mut mods, station.handle), stored(&station)),
        (500, 2500)
    );
    // Move the same stack/components into another mod's consumer: no shadow balance.
    let (handle, _) = mods
        .create_block_entity("test_machine:smelter", &[])
        .unwrap();
    let mut consumer = BlockEntityView {
        handle,
        dimension: "minecraft:overworld".into(),
        x: 1,
        y: 64,
        slots: vec![station.slots[1].take(), None],
        slot_components: vec![std::mem::take(&mut station.slot_components[1]), Vec::new()],
        ..Default::default()
    };
    assert!(mods.command("machine_api item_energy_extract"));
    tick(&mut mods, &mut consumer);
    assert_eq!(stored_at(&consumer, 0), 2200);
    assert_eq!(
        energy(&mut mods, station.handle) as i64 + stored_at(&consumer, 0) + 300,
        3000
    );
    // Full/unsupported/empty/stacked slots cannot drain the station.
    for stack in [
        None,
        Some(Stack {
            id: "minecraft:stick".into(),
            count: 1,
        }),
        Some(Stack {
            id: "test_machine:battery".into(),
            count: 2,
        }),
    ] {
        station.slots[1] = stack;
        station.slot_components[1].clear();
        tick(&mut mods, &mut station);
        assert_eq!(energy(&mut mods, station.handle), 500);
    }
    let mut empty = view(&mut mods, &seed(0));
    empty.slots[1] = Some(Stack {
        id: "test_machine:battery".into(),
        count: 1,
    });
    tick(&mut mods, &mut empty);
    assert_eq!(stored(&empty), 0);
    empty.slots[0] = Some(Stack {
        id: "minecraft:coal".into(),
        count: 1,
    });
    for _ in 0..2 {
        tick(&mut mods, &mut empty);
    }
    assert!(empty.slots[0].is_none());
    assert_eq!(stored(&empty), 1250);
    let saved_burn = mods.save_block_entity(empty.handle).unwrap();
    mods.remove_block_entity(empty.handle);
    let mut resumed = view(&mut mods, &saved_burn);
    resumed.slots = empty.slots;
    resumed.slot_components = empty.slot_components;
    for _ in 0..2 {
        tick(&mut mods, &mut resumed);
    }
    assert_eq!(
        stored(&resumed),
        2500,
        "four burning ticks after reload generate 4 x 625 FE"
    );
    assert_eq!(energy(&mut mods, resumed.handle), 0);
    let report = serde_json::json!({"status":"passed", "scope":"actual Charging Gadgets WASM + separate test battery provider/consumer WASM; real ItemStack codecs and scratch disk save, not full game",
        "original_behavior":{"observed":true,"fuel_slot":0,"charged_item_slot":1,"coal_burn_ticks":32,"coal_fe":20000,"fe_per_tick":625,"station_capacity":1000000},
        "checks":["charge equals station debit", "item receive limit", "partial final charge", "full item", "unsupported item", "empty slot", "stacked item refused", "no fuel/energy", "real fuel consumed at observed coal rate", "mid-burn unload/reload preserves remaining generation", "unrelated components preserved", "save/reopen", "300 FE consumed from transferred stack by another mod"],
        "remaining":["register energy support in real tool/battery ports", "full game automation and both single-player modes", "battery upgrade combinations"]});
    fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    if args.len() == 5 {
        fs::copy(root.join("charger.hmod"), &args[4])?;
    }
    println!("{report}");
    Ok(())
}

//! Real WASM shared-chest callbacks plus the game's canonical inventory storage.
//! MANIFEST CORE_WASM BLOCK OUTPUT_JSON. All packages/storage are temporary.
//! This component fixture does not substitute for the in-game hopper harness.
use horizon_client::{
    items::ItemStack,
    shared_inventory::{Inventory, SharedInventories},
};
use horizon_mods::{BlockAction, BlockEvent, Mods, Package, Status, manifest::componentize};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn bind(mods: &Mods, channels: &mut SharedInventories, handle: u32, old: &Inventory) -> Inventory {
    let (owner, channel) = mods
        .shared_inventory_channel(handle)
        .expect("WASM bound this endpoint");
    channels.bind(format!("{owner}:{channel}"), old).unwrap()
}
fn endpoint(
    mods: &mut Mods,
    channels: &mut SharedInventories,
    block: &str,
    x: i32,
    dim: &str,
    saved: &[u8],
) -> (u32, Inventory) {
    let (handle, slots) = mods
        .create_block_entity_at(block, saved, Some((dim.into(), x, 64, 0)))
        .expect("create endpoint");
    let inventory = bind(mods, channels, handle, &vec![None; slots as usize].into());
    (handle, inventory)
}
fn event(mods: &mut Mods, block: &str, x: i32, item: &str, actor: &str) {
    mods.block_event(
        BlockEvent {
            action: BlockAction::UseItemOn,
            dimension: "minecraft:overworld".into(),
            x,
            y: 64,
            z: 0,
            state: block.into(),
            actor: actor.into(),
            held_item: item.into(),
        },
        "up".into(),
    );
}
fn count(inventory: &Inventory, slot: usize) -> u32 {
    inventory.read()[slot]
        .as_ref()
        .map_or(0, |stack| stack.count)
}
fn assert_access(mods: &Mods, handle: u32, allowed: bool) {
    for slot in 0..27 {
        for face in ["up", "down", "north", "south", "east", "west"] {
            assert_eq!(mods.slot_access(handle, slot, face), (allowed, allowed));
        }
    }
}
fn extract(inventory: &Inventory, amount: u32) -> u32 {
    let mut cells = inventory.read();
    let Some(stack) = cells[0].as_mut() else {
        return 0;
    };
    let taken = stack.count.min(amount);
    stack.count -= taken;
    if stack.count == 0 {
        cells[0] = None;
    }
    taken
}
fn load(path: &Path) -> Mods {
    let mods = Mods::load(&[path.to_path_buf()], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    mods
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 && args.len() != 6 {
        return Err("expected MANIFEST CORE_WASM BLOCK OUTPUT_JSON [CARRY_DIR OUTPUT_HMOD]".into());
    }
    let root = std::env::temp_dir().join(format!("horizon-shared-chest-{}", std::process::id()));
    fs::create_dir(&root)?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let path = root.join("port.hmod");
    let manifest = fs::read_to_string(&args[0])?;
    fs::write(
        &path,
        Package::write(&manifest, &componentize(&fs::read(&args[1])?)?)?,
    )?;
    if args.len() == 6 {
        fn collect(
            root: &Path,
            path: &Path,
            files: &mut Vec<(String, Vec<u8>)>,
        ) -> std::io::Result<()> {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    collect(root, &entry.path(), files)?;
                } else if entry.file_type()?.is_file() {
                    let name = entry
                        .path()
                        .strip_prefix(root)
                        .expect("under carry root")
                        .to_string_lossy()
                        .replace('\\', "/");
                    files.push((name, fs::read(entry.path())?));
                }
            }
            Ok(())
        }
        let carry = Path::new(&args[4]);
        let mut files = Vec::new();
        for dir in ["assets", "data"] {
            if carry.join(dir).is_dir() {
                collect(carry, &carry.join(dir), &mut files)?;
            }
        }
        let component = componentize(&fs::read(&args[1])?)?;
        let refs: Vec<_> = files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        fs::write(
            &args[5],
            Package::write_files(&manifest, Some(&component), &refs)?,
        )?;
    }
    let block = &args[2];
    let mut mods = load(&path);
    let mut channels = SharedInventories::default();
    let (a, first) = endpoint(
        &mut mods,
        &mut channels,
        block,
        0,
        "minecraft:overworld",
        &[],
    );
    let (b, mut second) = endpoint(
        &mut mods,
        &mut channels,
        block,
        10,
        "minecraft:overworld",
        &[],
    );
    first.read()[0] = Some(ItemStack::new("minecraft:diamond", 18));
    assert_eq!(count(&second, 0), 18);
    assert_eq!(extract(&second, 5), 5);
    assert_eq!(count(&first, 0), 13);
    assert_eq!(extract(&first, 64), 13);
    assert_eq!(extract(&second, 64), 0);
    // Sequential operations in the same server tick, using the opposite order.
    second.read()[0] = Some(ItemStack::new("minecraft:diamond", 1));
    assert_eq!(extract(&second, 1), 1);
    assert_eq!(extract(&first, 1), 0);
    first.read()[0] = Some(ItemStack::new("minecraft:diamond", 23));
    let (_, other_dimension) = endpoint(
        &mut mods,
        &mut channels,
        block,
        0,
        "minecraft:the_nether",
        &[],
    );
    assert_eq!(count(&other_dimension, 0), 23);
    // Exercise the port's real configuration callbacks, not just fabricated saves.
    if block.starts_with("dimstorage:") {
        event(&mut mods, block, 10, "", "Alice");
        mods.menu_button("screen:dimstorage:chest", 102);
    } else {
        event(&mut mods, block, 10, "minecraft:red_dye", "Alice");
    }
    second = bind(&mods, &mut channels, b, &second);
    if block.starts_with("dimstorage:") {
        assert_access(&mods, b, true);
        mods.menu_button("screen:dimstorage:chest", 104);
        assert_access(&mods, b, false);
        let locked = mods.save_block_entity(b).expect("save locked endpoint");
        let (restored, _) = endpoint(
            &mut mods,
            &mut channels,
            block,
            20,
            "minecraft:overworld",
            &locked,
        );
        assert_access(&mods, restored, false);
        mods.remove_block_entity(restored);
        mods.menu_button("screen:dimstorage:chest", 104);
        assert_access(&mods, b, true);
        assert_access(&mods, a, true);
    }
    assert_eq!(count(&second, 0), 0, "new frequency starts empty");
    assert_eq!(
        count(&first, 0),
        23,
        "changing frequency does not move items"
    );
    second.read()[0] = Some(ItemStack::new("minecraft:iron_ingot", 8));
    event(&mut mods, block, 10, "minecraft:diamond", "Alice");
    second = bind(&mods, &mut channels, b, &second);
    assert_eq!(count(&second, 0), 0, "private frequency is distinct");
    second.read()[0] = Some(ItemStack::new("minecraft:gold_ingot", 3));
    let saved_a = mods.save_block_entity(a).expect("save public");
    let saved_b = mods.save_block_entity(b).expect("save private");
    let old_private = mods.shared_inventory_channel(b).unwrap();
    event(&mut mods, block, 10, "minecraft:diamond", "Bob");
    assert_eq!(
        mods.shared_inventory_channel(b).unwrap(),
        old_private,
        "another player cannot retune a private endpoint"
    );
    mods.remove_block_entity(a);
    mods.remove_block_entity(b);
    drop((mods, first, second, other_dimension));
    // Save with zero endpoints; recreate the WASM instance as on reopen.
    let bytes = horizon_world::nbt::write_network(&channels.snapshot(), Default::default())?;
    let (tag, _) = horizon_world::nbt::read_network(&bytes, Default::default())?;
    let mut channels = SharedInventories::restore(&tag)?;
    let mut mods = load(&path);
    // Reverse endpoint order on load.
    let (_, private) = endpoint(
        &mut mods,
        &mut channels,
        block,
        10,
        "minecraft:overworld",
        &saved_b,
    );
    let (_, public) = endpoint(
        &mut mods,
        &mut channels,
        block,
        0,
        "minecraft:overworld",
        &saved_a,
    );
    assert_eq!(count(&private, 0), 3);
    assert_eq!(count(&public, 0), 23);
    assert_eq!(mods.entries()[0].status, Status::Running);
    let report = serde_json::json!({
        "block": block, "status":"passed", "component":"real WASM", "storage":"game canonical inventory",
        "checks":["partial extraction", "full extraction", "same-tick endpoint operations in reversed order", "cross-dimension channel", "frequency change", "private owner isolation", "endpoint unload", "save without endpoints", "reopen WASM and restore in reversed order"],
        "automation_lock_checks": if block.starts_with("dimstorage:") { "all 27 slots, all faces: default, lock, locked load, unlock, other endpoint isolation" } else { "not applicable" },
        "scope":"game hopper/persistence validation is recorded separately; this fixture does not measure game throughput",
        "remaining_validation":["explicit capability-setter transfers across aliases"]
    });
    fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}

//! Isolated diagnostic of a port's empty block-entity tick cost, including host overhead.
//! Run with: CARRY_DIR CORE_WASM BLOCK COUNT TICKS OUTPUT_JSON
//! No game, assets or player data are opened. This is not a gameplay/parity benchmark.
use horizon_mods::{BlockEntityView, Mods, Package, Stack, Status, manifest::componentize};
use std::{path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err("expected CARRY_DIR CORE_WASM BLOCK COUNT TICKS OUTPUT_JSON".into());
    }
    let count: usize = args[3].parse()?;
    let ticks: usize = args[4].parse()?;
    if count == 0 || ticks == 0 {
        return Err("COUNT and TICKS must be positive".into());
    }
    let manifest = std::fs::read_to_string(PathBuf::from(&args[0]).join("horizon-mod.toml"))?;
    let core = std::fs::read(&args[1])?;
    let component = componentize(&core)?;
    let package = Package::write(&manifest, &component)?;
    let temp = std::env::temp_dir().join(format!("horizon-port-bench-{}", std::process::id()));
    std::fs::create_dir(&temp)?;
    // Remove the scratch package on errors too. Runtime storage is scoped to this folder.
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(temp.clone());
    let path = temp.join("port.hmod");
    std::fs::write(&path, package)?;
    let mut mods = Mods::load(&[path], |_| true, |_| false);
    let running =
        |mods: &Mods| mods.entries().len() == 1 && mods.entries()[0].status == Status::Running;
    if !running(&mods) {
        return Err(format!("load failed: {:?}", mods.entries()[0].status).into());
    }
    let mut views = Vec::with_capacity(count);
    for index in 0..count {
        let (handle, slots) = mods
            .create_block_entity(&args[2], &[])
            .ok_or("block entity creation failed")?;
        views.push(BlockEntityView {
            handle,
            dimension: "minecraft:overworld".into(),
            x: index.try_into()?,
            y: 64,
            slots: vec![None; slots as usize],
            slot_components: vec![Vec::new(); slots as usize],
            ..Default::default()
        });
    }
    let mut samples = Vec::with_capacity(ticks);
    for tick in 0..(ticks + 10) {
        let start = Instant::now();
        mods.tick_block_entities(&mut views);
        let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
        if !running(&mods) {
            return Err(format!("tick failed: {:?}", mods.entries()[0].status).into());
        }
        if tick >= 10 {
            samples.push(elapsed);
        }
    }
    samples.sort_by(f64::total_cmp);
    let idle = serde_json::json!({
        "p50_us": samples[ticks / 2],
        "p95_us": samples[((ticks - 1) * 95) / 100],
        "max_us": samples[ticks - 1],
    });
    // Reproduce extraction from a shared chest: remove a stack after its first sync.
    views[0].slots[0] = Some(Stack {
        id: "minecraft:diamond".into(),
        count: 1,
    });
    mods.tick_block_entities(&mut views);
    views[0].slots[0] = None;
    mods.tick_block_entities(&mut views);
    let report = serde_json::json!({
        "block": args[2], "count": count, "ticks": ticks,
        "scenario": "empty inventories; 10 warmup ticks; host + WASM; no ItemFacts",
        "idle": idle,
        "slot_after_extraction": views[0].slots[0].as_ref().map(|s| (&s.id, s.count)),
        "status": format!("{:?}", mods.entries()[0].status),
    });
    std::fs::write(&args[5], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}

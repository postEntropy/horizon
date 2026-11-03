//! Actual Farmer's Delight WASM conservation fixture. MANIFEST CORE_WASM OUT_JSON.
//! Uses synthetic recipes plus observed original storage/serving behavior.
use horizon_mods::{
    BlockEntityView, CookingKind, CookingRecipe, IngredientSlot, ItemFacts, Mods, Package,
    RecipeEntry, Stack, Status, manifest::componentize,
};
use std::{fs, path::PathBuf, sync::Arc};
struct Facts;
impl ItemFacts for Facts {
    fn max_stack_size(&self, id: &str) -> u32 {
        match id {
            "farmersdelight:beef_stew" | "farmersdelight:apple_cider" => 16,
            _ => 64,
        }
    }
    fn burn_time(&self, _: &str) -> u32 {
        0
    }
    fn remainder(&self, _: &str) -> Option<String> {
        None
    }
    fn cooking_recipe(&self, _: CookingKind, _: &str) -> Option<CookingRecipe> {
        None
    }
    fn block_in_tag(&self, id: &str, tag: &str) -> bool {
        match tag {
            "farmersdelight:heat_sources" => {
                matches!(id, "minecraft:magma_block" | "minecraft:campfire")
            }
            "farmersdelight:heat_conductors" => id == "minecraft:hopper",
            _ => false,
        }
    }
}
fn stack(id: &str, count: u32) -> Option<Stack> {
    Some(Stack {
        id: id.into(),
        count,
    })
}
fn create(mods: &mut Mods, saved: &[u8]) -> BlockEntityView {
    let (handle, slots) = mods
        .create_block_entity_at(
            "farmersdelight:cooking_pot",
            saved,
            Some(("minecraft:overworld".into(), 0, 64, 0)),
        )
        .unwrap();
    assert_eq!(mods.player_slot_access(handle, 6), (false, false));
    assert_eq!(mods.player_slot_access(handle, 8), (false, true));
    assert_eq!(mods.slot_limit(handle, 6), Some(64));
    for face in ["up", "down", "north", "south", "east", "west"] {
        assert_eq!(mods.slot_access(handle, 6, face), (false, false));
    }
    let mut view = BlockEntityView {
        handle,
        dimension: "minecraft:overworld".into(),
        y: 64,
        slots: vec![None; slots as usize],
        slot_components: vec![Vec::new(); slots as usize],
        ..Default::default()
    };
    for (slot, id) in [
        (0, "minecraft:beef"),
        (1, "minecraft:carrot"),
        (2, "minecraft:potato"),
        (7, "minecraft:bowl"),
    ] {
        view.slots[slot] = stack(id, 2);
    }
    view
}
fn tick(mods: &mut Mods, view: &mut BlockEntityView) {
    mods.tick_block_entities(std::slice::from_mut(view));
    assert_eq!(mods.entries()[0].status, Status::Running);
}
fn recipe(ticks: u32, result: &str, count: u32) -> RecipeEntry {
    RecipeEntry {
        id: "farmersdelight:fixture".into(),
        kind: "farmersdelight:cooking".into(),
        inputs: [
            vec!["minecraft:beef", "minecraft:carrot"],
            vec!["minecraft:beef"],
            vec!["minecraft:potato"],
        ]
        .into_iter()
        .map(|items| IngredientSlot {
            items: items.into_iter().map(str::to_owned).collect(),
            count: 1,
        })
        .collect(),
        outputs: vec![format!("{result}*{count}")],
        ticks,
        energy: 0,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("MANIFEST CORE_WASM OUT_JSON".into());
    }
    let root = std::env::temp_dir().join(format!("horizon-farmer-{}", std::process::id()));
    fs::create_dir(&root)?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let path = root.join("port.hmod");
    fs::write(
        &path,
        Package::write(
            &fs::read_to_string(&args[0])?,
            &componentize(&fs::read(&args[1])?)?,
        )?,
    )?;
    let mut mods = Mods::load(&[path], |_| true, |_| false);
    mods.set_item_facts(Arc::new(Facts));
    mods.add_resource_recipes([recipe(200, "farmersdelight:beef_stew", 1)]);
    let saved = [199u32.to_le_bytes(), 200u32.to_le_bytes()].concat();
    for scenario in [
        "full_meal",
        "incompatible_meal",
        "blocked_output",
        "full_output",
        "store",
        "missing",
        "extra",
        "cold",
        "unlit",
        "success",
        "conductor",
    ] {
        mods.set_world_block(
            "minecraft:overworld",
            0,
            63,
            0,
            if scenario == "cold" {
                "minecraft:stone"
            } else if scenario == "unlit" {
                "minecraft:campfire[lit=false]"
            } else if scenario == "conductor" {
                "minecraft:hopper"
            } else {
                "minecraft:magma_block"
            },
        );
        mods.set_world_block("minecraft:overworld", 0, 62, 0, "minecraft:magma_block");
        let mut view = create(&mut mods, &saved);
        match scenario {
            "full_meal" => {
                view.slots[6] = stack("farmersdelight:beef_stew", 64);
                view.slots[7] = None;
            }
            "incompatible_meal" => view.slots[6] = stack("minecraft:diamond", 1),
            "blocked_output" => view.slots[8] = stack("minecraft:diamond", 64),
            "full_output" => view.slots[8] = stack("farmersdelight:beef_stew", 16),
            "store" => view.slots[7] = None,
            "missing" => view.slots[2] = None,
            "extra" => view.slots[3] = stack("minecraft:diamond", 1),
            _ => {}
        }
        let before = view.slots.clone();
        tick(&mut mods, &mut view);
        if matches!(scenario, "success" | "conductor") {
            assert_eq!(view.slots[8], stack("farmersdelight:beef_stew", 1));
            assert!(view.slots[6].is_none());
            for slot in [0, 1, 2, 7] {
                assert_eq!(view.slots[slot].as_ref().unwrap().count, 1);
            }
        } else if matches!(scenario, "store" | "blocked_output" | "full_output") {
            assert_eq!(view.slots[6], stack("farmersdelight:beef_stew", 1));
            for slot in 0..3 {
                assert_eq!(view.slots[slot].as_ref().unwrap().count, 1);
            }
            assert_eq!(view.slots[7], before[7]);
            assert_eq!(view.slots[8], before[8]);
        } else {
            assert_eq!(view.slots, before, "{scenario}");
        }
        mods.remove_block_entity(view.handle);
    }
    mods.set_world_block("minecraft:overworld", 0, 63, 0, "minecraft:magma_block");
    let mut view = create(&mut mods, &[]);
    for _ in 0..100 {
        tick(&mut mods, &mut view);
    }
    let saved = mods.save_block_entity(view.handle).unwrap();
    mods.remove_block_entity(view.handle);
    let mut restored = create(&mut mods, &saved);
    restored.slots = view.slots;
    restored.slot_components = view.slot_components;
    for _ in 0..100 {
        tick(&mut mods, &mut restored);
    }
    assert_eq!(restored.slots[8], stack("farmersdelight:beef_stew", 1));
    mods.remove_block_entity(restored.handle);
    // A replacement published by the host must invalidate the guest cache.
    mods.register_recipe(recipe(100, "farmersdelight:dumplings", 2));
    let mut view = create(&mut mods, &[]);
    view.slots[7] = None; // This result has no serving container.
    for _ in 0..99 {
        tick(&mut mods, &mut view);
    }
    assert!(view.slots[8].is_none());
    tick(&mut mods, &mut view);
    assert_eq!(view.slots[8], stack("farmersdelight:dumplings", 2));
    assert!(view.slots[6].is_none());
    for slot in 0..3 {
        assert_eq!(view.slots[slot].as_ref().unwrap().count, 1);
    }
    mods.remove_block_entity(view.handle);
    // Resource publications use the same cache invalidation path.
    mods.add_resource_recipes([recipe(1, "farmersdelight:apple_cider", 1)]);
    let mut view = create(&mut mods, &[]);
    for slot in 0..3 {
        view.slots[slot].as_mut().unwrap().count = 1;
    }
    tick(&mut mods, &mut view);
    assert_eq!(view.slots[6], stack("farmersdelight:apple_cider", 1));
    assert!(view.slots[8].is_none(), "bowl must not replace bottle");
    view.slots[7] = stack("minecraft:glass_bottle", 2);
    tick(&mut mods, &mut view);
    assert_eq!(view.slots[8], stack("farmersdelight:apple_cider", 1));
    assert_eq!(view.slots[7], stack("minecraft:glass_bottle", 1));
    assert!(view.slots[6].is_none());
    mods.remove_block_entity(view.handle);
    // Serving remains possible with no heat or matching ingredients. Amounts
    // match the original menu fixture after cooking initialized its metadata.
    mods.set_world_block("minecraft:overworld", 0, 63, 0, "minecraft:stone");
    for (meal, output, expected_meal, expected_container) in [
        ("farmersdelight:beef_stew", None, 16, 16),
        (
            "farmersdelight:beef_stew",
            stack("farmersdelight:beef_stew", 15),
            31,
            31,
        ),
    ] {
        let mut view = create(&mut mods, &[]);
        view.slots.fill(None);
        view.slots[6] = stack(meal, 32);
        view.slots[7] = stack("minecraft:bowl", 32);
        view.slots[8] = output;
        tick(&mut mods, &mut view);
        assert_eq!(view.slots[6], stack(meal, expected_meal));
        assert_eq!(view.slots[7], stack("minecraft:bowl", expected_container));
        assert_eq!(view.slots[8], stack(meal, 16));
        mods.remove_block_entity(view.handle);
    }
    let report = serde_json::json!({"status":"passed","scope":"actual WASM; synthetic resource recipes and fixture item facts; storage/serving semantics observed from original black-box fixture","checks":["full/incompatible reservoir preserves ingredients","blocked/full served output stores meals without debiting container","64 meal capacity overrides served-item limit of 16","missing/extra ingredient refuses recipe","cold and unlit source do not cook","heated and conductor cook","overlapping ingredient alternatives","exact ingredient/container debit","mid-progress save/load","host and resource publication invalidate recipe cache","loaded duration and output count","container-free result","bottle required for cider","serving without heat or ingredients","serving batch respects real output limit and available space","player and hopper reservoir policies configured"],"remaining":["all 28 recipes with installed carry tags","recipe-specific container metadata and ingredient remainders","real game in both modes including menu policies"]});
    fs::write(&args[2], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

//! The Horizon side of a code mod's machine harness (tools/mod_machine_harness.py,
//! docs/research/code-mods-plan.md): runs a ported mod's block entity through
//! horizon-mods on Horizon's own item, fuel and cooking tables, and writes its
//! slots, block state and saved fields after every tick.
//!
//! cargo run -p horizon-client --example mod_machine_probe -- PREPARED MOD.hmod SCENARIOS.json OUT.json
//!
//! SCENARIOS.json is also accepted by the NeoForge oracle:
//! `[{"name", "block", "ticks", "dimension", "x", "y", "z",
//! "slots": {"0": ["minecraft:raw_iron", 8]}, "energy": {"stored", "capacity"},
//! "fluids": [{"tank", "fluid", "amount", "capacity"}],
//! "reload_at": [100],
//! "properties": {"lit": ["false", ["false", "true"]]}, "fields": ["CookTime"],
//! "events": [{"tick": 40, "slot": 1, "stack": ["minecraft:coal", 1]}],
//! "expect_final": {"slots": {"1": ["minecraft:iron_ingot", 1]}}}].
//! Recipe files come from `PREPARED`; it must include the mod's carried data.
//! `expect_final` checks selected Horizon outputs after the last tick.
use horizon_client::{
    Result,
    assets::Assets,
    blocks::BlockFacts,
    containers::Providers,
    crafting::Recipes,
    items::{ItemStack, Items},
};
use horizon_mods::{
    BlockEntityView, CookingKind, CookingRecipe, EnergyView, IngredientSlot, ItemFacts, Mods,
    Property, RecipeEntry, Stack, Status,
};
use serde_json::{Map, Value, json};
use std::{fs, path::Path, sync::Arc};

struct Facts {
    items: Items,
    recipes: Recipes,
    providers: Providers,
}
impl ItemFacts for Facts {
    fn max_stack_size(&self, id: &str) -> u32 {
        self.items.get(id).map_or(0, |i| i.max_stack)
    }
    fn burn_time(&self, id: &str) -> u32 {
        let stack = ItemStack::new(id, 1);
        self.providers.burn_time(&self.items, &stack, false).max(0) as u32
    }
    fn remainder(&self, id: &str) -> Option<String> {
        self.items.get(id)?.crafting_remainder.clone()
    }
    fn cooking_recipe(&self, kind: CookingKind, input: &str) -> Option<CookingRecipe> {
        let kind = match kind {
            CookingKind::Smelting => "minecraft:smelting",
            CookingKind::Blasting => "minecraft:blasting",
            CookingKind::Smoking => "minecraft:smoking",
            CookingKind::Campfire => "minecraft:campfire_cooking",
        };
        let c = self.recipes.cook(kind, &ItemStack::new(input, 1))?;
        Some(CookingRecipe {
            output: Stack {
                id: c.result.item.to_string(),
                count: c.result.count,
            },
            cooking_time: c.time,
            experience: c.experience,
        })
    }
}

fn stack(v: &Value) -> Option<Stack> {
    Some(Stack {
        id: v[0].as_str()?.to_owned(),
        count: v[1].as_u64()? as u32,
    })
}

fn run(mods: &mut Mods, scenario: &Value) -> Result<Value> {
    let block = scenario["block"].as_str().ok_or("scenario without block")?;
    let dimension = scenario["dimension"]
        .as_str()
        .unwrap_or("minecraft:overworld");
    let x = scenario["x"].as_i64().unwrap_or(0) as i32;
    let y = scenario["y"].as_i64().unwrap_or(64) as i32;
    let z = scenario["z"].as_i64().unwrap_or(0) as i32;
    let (mut handle, slots) = mods
        .create_block_entity_at(block, &[], Some((dimension.to_owned(), x, y, z)))
        .ok_or_else(|| format!("no running mod has a block entity for {block}"))?;
    let capability_key = format!("{dimension}:{x},{y},{z}");
    if let Some(energy) = scenario.get("energy") {
        mods.set_energy_buffer(
            &capability_key,
            EnergyView {
                stored: energy["stored"].as_u64().unwrap_or(0),
                capacity: energy["capacity"].as_u64().unwrap_or(1_000_000),
                max_receive: energy["max_receive"].as_u64().unwrap_or(u64::MAX),
                max_extract: energy["max_extract"].as_u64().unwrap_or(u64::MAX),
            },
        );
    }
    for tank in scenario["fluids"].as_array().into_iter().flatten() {
        let index = tank["tank"].as_u64().unwrap_or(0) as u32;
        let capacity = tank["capacity"].as_u64().unwrap_or(16_000);
        let fluid = tank["fluid"].as_str().unwrap_or_default();
        let amount = tank["amount"].as_u64().unwrap_or(0);
        mods.declare_fluid_tank(&capability_key, index, capacity);
        if amount > 0 {
            let accepted = mods.fill_fluid_tank(&capability_key, index, fluid, amount);
            if accepted != amount {
                return Err(
                    format!("scenario fluid tank {index} accepted {accepted}/{amount}").into(),
                );
            }
        }
    }
    let mut view = BlockEntityView {
        handle,
        dimension: dimension.to_owned(),
        x,
        y,
        z,
        slots: vec![None; slots as usize],
        ..Default::default()
    };
    for (slot, s) in scenario["slots"].as_object().into_iter().flatten() {
        view.slots[slot.parse::<usize>()?] = stack(s);
    }
    for (name, p) in scenario["properties"].as_object().into_iter().flatten() {
        view.properties.push(Property {
            name: name.clone(),
            value: p[0].as_str().unwrap_or_default().to_owned(),
            values: p[1]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| Some(v.as_str()?.to_owned()))
                .collect(),
        });
    }
    let fields: Vec<&str> = scenario["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut trace = Vec::new();
    for tick in 0..scenario["ticks"].as_u64().unwrap_or(0) {
        if scenario["reload_at"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|reload_tick| reload_tick.as_u64() == Some(tick))
        {
            let saved = mods.save_block_entity(handle).unwrap_or_default();
            mods.remove_block_entity(handle);
            mods.clear_machine_buffers(&capability_key);
            let (reloaded, reloaded_slots) = mods
                .create_block_entity_at(block, &saved, Some((dimension.to_owned(), x, y, z)))
                .ok_or_else(|| format!("could not reload machine {block}"))?;
            handle = reloaded;
            view.handle = reloaded;
            view.slots.resize(reloaded_slots as usize, None);
        }
        for e in scenario["events"].as_array().into_iter().flatten() {
            if e["tick"].as_u64() == Some(tick) {
                view.slots[e["slot"].as_u64().unwrap_or(0) as usize] = stack(&e["stack"]);
            }
        }
        let mut views = [view];
        mods.tick_block_entities(&mut views);
        [view] = views;
        let saved = mods.save_block_entity(handle).unwrap_or_default();
        let saved_len = saved.len();
        let saved_progress = saved
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes);
        let saved_value: Value = serde_json::from_slice(&saved).unwrap_or(Value::Null);
        let slots: Map<String, Value> = view
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| Some((i.to_string(), json!([s.as_ref()?.id, s.as_ref()?.count]))))
            .collect();
        let properties: Map<String, Value> = view
            .properties
            .iter()
            .map(|p| (p.name.clone(), json!(p.value)))
            .collect();
        let values: Map<String, Value> = fields
            .iter()
            .map(|f| (f.to_string(), saved_value[*f].clone()))
            .collect();
        let energy = mods.energy_buffer(&capability_key).map(|view| {
            json!({
                "stored": view.stored,
                "capacity": view.capacity,
            })
        });
        trace.push(json!({
            "slots": slots,
            "properties": properties,
            "fields": values,
            "energy": energy,
            "saved_progress": saved_progress,
            "saved_bytes": saved_len,
        }));
    }
    if let Some(expected) = scenario.get("expect_final") {
        let actual = trace.last().ok_or("scenario has no ticks to check")?;
        for (key, value) in expected
            .as_object()
            .ok_or("expect_final must be a JSON object")?
        {
            if actual.get(key) != Some(value) {
                return Err(format!(
                    "scenario {}: final {key} was {}, expected {value}",
                    scenario["name"].as_str().unwrap_or("unnamed"),
                    actual.get(key).unwrap_or(&Value::Null)
                )
                .into());
            }
        }
    }
    mods.remove_block_entity(handle);
    Ok(json!({"name": scenario["name"], "trace": trace}))
}

fn register_resource_recipes(mods: &mut Mods, recipes: &Recipes) {
    mods.add_resource_recipes(recipes.machine.iter().map(|recipe| {
        RecipeEntry {
            id: recipe.id.clone(),
            kind: recipe.kind.clone(),
            inputs: recipe
                .inputs
                .iter()
                .map(|input| match input {
                    horizon_client::crafting::MachineInput::Item { ingredient, count } => {
                        IngredientSlot {
                            items: ingredient.1.clone(),
                            count: *count,
                        }
                    }
                    horizon_client::crafting::MachineInput::Resource {
                        kind,
                        tank,
                        accepted,
                        amount,
                    } => {
                        let kind = match kind {
                            horizon_client::crafting::MachineResourceKind::Fluid => "fluid",
                            horizon_client::crafting::MachineResourceKind::Chemical => "chemical",
                        };
                        let tank = *tank + if kind == "chemical" { 1_000 } else { 0 };
                        IngredientSlot {
                            items: accepted
                                .iter()
                                .map(|id| format!("@{kind}/{tank}/{id}"))
                                .collect(),
                            count: *amount,
                        }
                    }
                })
                .collect(),
            outputs: recipe
                .outputs
                .iter()
                .map(|output| match output {
                    horizon_client::crafting::MachineOutput::Item(stack) => {
                        format!("{}*{}", stack.item, stack.count)
                    }
                    horizon_client::crafting::MachineOutput::Resource {
                        kind,
                        tank,
                        id,
                        amount,
                    } => {
                        let kind = match kind {
                            horizon_client::crafting::MachineResourceKind::Fluid => "fluid",
                            horizon_client::crafting::MachineResourceKind::Chemical => "chemical",
                        };
                        let tank = *tank + if kind == "chemical" { 1_000 } else { 0 };
                        format!("@{kind}/{tank}/{id}*{amount}")
                    }
                })
                .collect(),
            ticks: recipe.ticks,
            energy: recipe.energy,
        }
    }));
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, hmod, scenarios, out] = args.as_slice() else {
        return Err("usage: mod_machine_probe PREPARED MOD.hmod SCENARIOS.json OUT.json".into());
    };
    let root = Path::new(root);
    let assets = Assets::load(root)?;
    let facts = BlockFacts::load(root, assets.registry)?;
    let items = Items::load(root, assets.registry, &facts)?;
    let recipes = Recipes::load(root, &items)?;
    let providers = Providers::load(root);
    let mut mods = Mods::load(&[hmod.into()], |_| true, |_| false);
    if let Status::Failed(why) = &mods.entries()[0].status {
        return Err(format!("{hmod}: {why}").into());
    }
    register_resource_recipes(&mut mods, &recipes);
    mods.set_item_facts(Arc::new(Facts {
        items,
        recipes,
        providers,
    }));
    let scenarios: Value = serde_json::from_slice(&fs::read(scenarios)?)?;
    let mut results = Vec::new();
    for scenario in scenarios.as_array().into_iter().flatten() {
        results.push(run(&mut mods, scenario)?);
    }
    fs::write(out, serde_json::to_vec_pretty(&results)?)?;
    Ok(())
}

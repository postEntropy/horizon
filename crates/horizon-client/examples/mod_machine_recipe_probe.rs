//! Load carried mod content and print machine recipes as Horizon resolves them.
//!
//! cargo run -p horizon-client --example mod_machine_recipe_probe -- PREPARED WORK OUT.json [MOD...]

use horizon_client::{
    Result,
    assets::Assets,
    crafting::{MachineInput, MachineOutput, Recipes},
    items::Items,
    mod_content::{self, ContentMod},
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [prepared, work, out, wanted @ ..] = args.as_slice() else {
        return Err("usage: mod_machine_recipe_probe PREPARED WORK OUT.json [MOD...]".into());
    };
    let (prepared, work) = (Path::new(prepared), Path::new(work));
    let mut mods = Vec::new();
    for entry in fs::read_dir(work.join("mods"))?.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        if !wanted.is_empty() && !wanted.contains(&id) {
            continue;
        }
        let dir = entry.path().join("carry");
        if !dir.is_dir() {
            continue;
        }
        let package = horizon_mods::Package::read(&dir)?;
        let content = horizon_mods::Content::read(&dir, &package.manifest)?;
        mods.push(ContentMod {
            id: package.manifest.id.clone(),
            content,
        });
    }
    mods.sort_by(|a, b| a.id.cmp(&b.id));
    let root = work.join("machine-recipe-root");
    mod_content::build(prepared, &mods, &root)?;
    let assets = Assets::load(&root)?;
    let items = Items::load(
        &root,
        assets.registry,
        &horizon_client::blocks::BlockFacts::load(&root, assets.registry)?,
    )?;
    let recipes = Recipes::load(&root, &items)?;
    let rows: Vec<Value> = recipes.machine.iter().filter(|recipe| recipe.kind == "farmersdelight:cooking" || recipe.kind.starts_with("ae2:inscriber:") || recipe.kind.starts_with("create:") || recipe.kind.starts_with("thermal:") || recipe.kind.starts_with("avaritia:")).map(|recipe| {
        let inputs: Vec<Value> = recipe.inputs.iter().map(|input| match input {
            MachineInput::Item { ingredient, count } => json!({"item_candidates": ingredient.1, "count": count}),
            MachineInput::Resource { kind, tank, accepted, amount } => json!({"resource_kind": format!("{kind:?}"), "tank": tank, "accepted": accepted, "amount": amount}),
        }).collect();
        let outputs: Vec<Value> = recipe.outputs.iter().map(|output| match output {
            MachineOutput::Item(stack) => json!({"item": stack.item.to_string(), "count": stack.count, "chance": 1.0}),
            MachineOutput::ItemWithChance { stack, chance } => json!({"item": stack.item.to_string(), "count": stack.count, "chance": chance}),
            MachineOutput::Resource { kind, tank, id, amount } => json!({"resource_kind": format!("{kind:?}"), "tank": tank, "id": id, "amount": amount}),
        }).collect();
        json!({"id": recipe.id, "kind": recipe.kind, "inputs": inputs, "outputs": outputs, "ticks": recipe.ticks, "energy": recipe.energy})
    }).collect();
    fs::write(out, serde_json::to_vec_pretty(&rows)?)?;
    println!("{}: {} selected machine recipes loaded", out, rows.len());
    Ok(())
}

//! Build and validate the content-only Stone Chest package.
//!
//! The mod's behavior comes from carried registries and Horizon's vanilla
//! chest host implementation; no WebAssembly component is needed.

use horizon_mods::{Content, Package};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const VARIANTS: [&str; 10] = [
    "andesite",
    "calcite",
    "cobbled_deepslate",
    "cobblestone",
    "deepslate",
    "diorite",
    "granite",
    "prismarine",
    "stone",
    "tuff",
];

fn collect(
    root: &Path,
    dir: &Path,
    include_binary: bool,
    out: &mut Vec<(String, Vec<u8>)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(root, &path, include_binary, out)?;
        } else if entry.file_type()?.is_file()
            && (include_binary
                || path
                    .extension()
                    .is_some_and(|extension| extension == "json"))
        {
            let name = path
                .strip_prefix(root)
                .expect("content path is under the port folder")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((name, fs::read(path)?));
        }
    }
    Ok(())
}

fn content_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, Box<dyn Error>> {
    let mut files = Vec::new();
    for prefix in ["assets", "data"] {
        let dir = root.join(prefix);
        if dir.exists() {
            collect(root, &dir, prefix == "assets", &mut files)?;
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn build(output: &Path) -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_text = fs::read_to_string(root.join("horizon-mod.toml"))?;
    let files = content_files(&root)?;
    let refs: Vec<_> = files
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    let bytes = Package::write_files(&manifest_text, None, &refs)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, bytes)?;

    let package = Package::read(output)?;
    let content = Content::read(output, &package.manifest)?;
    let chest_count = content
        .blocks
        .iter()
        .filter(|block| block.id.starts_with("stonechest:chest_"))
        .count();
    let part_count = content
        .items
        .iter()
        .filter(|item| item.id.starts_with("stonechest:part_"))
        .count();
    let texture_count = files
        .iter()
        .filter(|(name, _)| {
            name.starts_with("assets/stonechest/textures/entity/chest/") && name.ends_with(".png")
        })
        .count();
    if chest_count != VARIANTS.len() || part_count != VARIANTS.len() || texture_count != 30 {
        return Err(format!(
            "package has {chest_count} chest blocks, {part_count} parts and {texture_count} chest textures; expected ten, ten and thirty"
        )
        .into());
    }
    println!(
        "Built {}: 10 chest blocks, 10 part items, {} content files",
        output.display(),
        content.files.len()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/horizon-mod/stonechest.hmod"));
    build(&output)
}

#[cfg(test)]
mod tests {
    use super::VARIANTS;
    use serde_json::Value;
    use std::{fs, path::PathBuf};

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn json(path: PathBuf) -> Value {
        serde_json::from_slice(&fs::read(path).expect("carried resource exists"))
            .expect("carried resource is valid JSON")
    }

    #[test]
    fn every_imported_chest_variant_has_registration_recipe_and_loot() {
        for variant in VARIANTS {
            let id = format!("stonechest:chest_{variant}");
            let block = json(root().join(format!(
                "data/stonechest/horizon/block/chest_{variant}.json"
            )));
            assert_eq!(block["template"], "minecraft:chest", "{id}");
            assert_eq!(block["item"], true, "{id} must register its quest item");

            let recipe = json(root().join(format!("data/stonechest/recipes/chest_{variant}.json")));
            assert_eq!(recipe["result"]["item"], id, "{id}");

            let loot = json(root().join(format!(
                "data/stonechest/loot_tables/blocks/chest_{variant}.json"
            )));
            assert_eq!(loot["pools"][0]["entries"][0]["name"], id, "{id}");
            assert_eq!(
                loot["pools"][0]["entries"][0]["functions"][0]["function"], "minecraft:copy_name",
                "{id} preserves a custom chest name when dropped"
            );
        }
    }

    #[test]
    fn every_part_has_a_vanilla_item_template_and_crafting_recipe() {
        for variant in VARIANTS {
            let id = format!("stonechest:part_{variant}");
            let item =
                json(root().join(format!("data/stonechest/horizon/item/part_{variant}.json")));
            assert_eq!(item["template"], "minecraft:stick", "{id}");

            let recipe = json(root().join(format!("data/stonechest/recipes/part_{variant}.json")));
            assert_eq!(recipe["result"]["item"], id, "{id}");
        }
    }

    #[test]
    fn capacity_is_vanilla_chest_capacity_not_a_mod_config() {
        let manifest = fs::read_to_string(root().join("horizon-mod.toml")).unwrap();
        assert!(!manifest.contains("chest_capacity"));
        let carry = json(root().join("carry-report.json"));
        assert_eq!(carry["blocks"].as_object().unwrap().len(), VARIANTS.len());
        assert_eq!(carry["items"], VARIANTS.len() as u64);
    }
}

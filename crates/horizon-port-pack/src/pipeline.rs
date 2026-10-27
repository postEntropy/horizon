//! The steps on a work folder, as the command line and the launcher run
//! them. Each one reads what the earlier steps wrote and says what it did
//! through `log`, one line at a time.
use crate::{carry, fetch, import, inventory, modrinth, oracle, report};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// Where a step's lines go: the terminal, or the launcher's log.
pub type Log<'a> = &'a dyn Fn(String);

pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Step 1: the pack at `pack` to `<work>/pack.json`.
pub fn import(pack: &Path, work: &Path, log: Log) -> Result<import::Pack, String> {
    let source = pack;
    let mut pack = import::import(source)?;
    fs::create_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))?;
    let overrides = import::extract_overrides(source, &work.join("pack-overrides"))?;
    let override_datapacks = import::index_override_datapacks(work, &mut pack)?;
    write_json(&work.join("pack.json"), &pack)?;
    let mut kinds = BTreeMap::<&str, usize>::new();
    for f in &pack.files {
        *kinds.entry(f.kind()).or_default() += 1;
    }
    log(format!(
        "{} ({}): Minecraft {}, {} {}; files {kinds:?}; overrides {:?}",
        pack.name, pack.format, pack.minecraft, pack.loader, pack.loader_version, pack.overrides
    ));
    log(format!("{overrides} pack configuration files staged"));
    if override_datapacks > 0 {
        log(format!(
            "{override_datapacks} override data packs indexed for carry-over"
        ));
    }
    Ok(pack)
}

/// Step 2: `<work>/inventory.json` and `inventory.csv`. Online, Modrinth is
/// asked about the files it has not answered for yet; when it cannot be
/// reached, the cached answers and the reviewed table classify alone.
pub fn inventory(work: &Path, offline: bool, log: Log) -> Result<Vec<inventory::Record>, String> {
    let pack: import::Pack = read_json(&work.join("pack.json"))?;
    let cache_path = work.join("modrinth.json");
    let mut cache = modrinth::Cache::load(&cache_path);
    if !offline {
        let hashes = inventory::hashes(&pack, work);
        match cache.fetch(&hashes) {
            Ok(()) => write_json(&cache_path, &cache)?,
            Err(e) => log(format!("Modrinth not reached, classifying offline: {e}")),
        }
    }
    let records = inventory::build(&pack, &cache, work);
    write_json(&work.join("inventory.json"), &records)?;
    fs::write(work.join("inventory.csv"), inventory::csv(&records))
        .map_err(|e| format!("inventory.csv: {e}"))?;
    let jars = records.iter().filter(|r| r.jar.is_some()).count();
    log(format!("{} files, {jars} jars read", records.len()));
    for (title, field) in [
        (
            "class",
            (|r: &inventory::Record| r.class.as_str()) as fn(&inventory::Record) -> &str,
        ),
        ("tier", |r| r.tier.as_str()),
        ("confidence", |r| r.confidence.as_str()),
        ("source", |r| r.source.as_str()),
    ] {
        log(format!("{title}: {:?}", inventory::counts(&records, field)));
    }
    let layers = records.iter().map(|r| r.layer).max().map_or(0, |m| m + 1);
    let per_layer: Vec<usize> = (0..layers)
        .map(|l| records.iter().filter(|r| r.layer == l).count())
        .collect();
    log(format!("layers: {per_layer:?}"));
    let missing: usize = records.iter().map(|r| r.missing.len()).sum();
    if missing > 0 {
        log(format!(
            "{missing} required dependencies are not in the pack (see inventory.json)"
        ));
    }
    Ok(records)
}

/// The vanilla blocks a prepared resource root has (`facts.json`'s defaults).
fn vanilla_blocks(resources: &Path) -> Result<BTreeSet<String>, String> {
    let facts: serde_json::Value = read_json(&resources.join("facts.json"))?;
    let defaults = facts["defaults"]
        .as_object()
        .ok_or("facts.json: no defaults")?;
    Ok(defaults.keys().cloned().collect())
}

/// Vanilla's property domains, by block id and property name.
fn vanilla_state_schemas(resources: &Path) -> Result<carry::StateSchemas, String> {
    let facts: Value = read_json(&resources.join("facts.json"))?;
    let states = facts["states"].as_array().ok_or("facts.json: no states")?;
    let mut schemas = carry::StateSchemas::new();
    for row in states {
        let Some(name) = row.get(0).and_then(Value::as_str) else {
            continue;
        };
        let Some(properties) = row
            .get(4)
            .and_then(|state| state.get("properties"))
            .and_then(Value::as_object)
        else {
            continue;
        };
        let schema = schemas.entry(name.to_owned()).or_default();
        for (property, value) in properties {
            if let Some(value) = value.as_str() {
                schema
                    .entry(property.clone())
                    .or_default()
                    .insert(value.to_owned());
            }
        }
    }
    Ok(schemas)
}

/// Step 4: supported-tier mods with a jar (or the ones in `only`) carried over
/// into `<work>/mods/<id>/carry/`. Returns how many were.
pub fn carry(
    work: &Path,
    resources: Option<&Path>,
    only: &[String],
    log: Log,
) -> Result<usize, String> {
    let records: Vec<inventory::Record> = read_json(&work.join("inventory.json"))?;
    let facts_path = work.join("oracle/facts.json");
    let facts = if facts_path.exists() {
        let f = oracle::Facts::read(&facts_path)?;
        log(format!(
            "oracle facts: {} blocks, {} items",
            f.blocks.len(),
            f.items.len()
        ));
        Some(f)
    } else {
        None
    };
    // Without the prepared tables, templates are not limited to what the game has.
    let vanilla = resources.and_then(|r| {
        vanilla_blocks(r)
            .inspect_err(|e| log(format!("templates not limited to the game's blocks: {e}")))
            .ok()
    });
    let state_schemas = resources.and_then(|r| {
        vanilla_state_schemas(r)
            .inspect_err(|e| log(format!("template state domains unavailable: {e}")))
            .ok()
    });
    let allowed = |b: &str| vanilla.as_ref().is_none_or(|v| v.contains(b));
    let conventions = crate::worldgen::loader_biome_tags(work);
    let chosen = |r: &inventory::Record| {
        if only.is_empty() {
            portable(r)
        } else {
            only.contains(&r.key)
        }
    };
    // The mods ported along (carried now or before), for the conditions of
    // the mods' worldgen modifiers.
    let ported: BTreeSet<&str> = records
        .iter()
        .filter(|r| {
            r.jar.is_some() && (chosen(r) || work.join("mods").join(&r.key).join("carry").is_dir())
        })
        .filter_map(|r| Some(r.info.as_ref()?.mod_id.as_str()))
        .chain(["minecraft"])
        .collect();
    let loaded = |m: &str| ported.contains(m);
    // Every mod of the pack, ported or not: a namespace one of them has as
    // its id is never another mod's.
    let in_pack: BTreeSet<&str> = records
        .iter()
        .flat_map(|r| {
            [
                Some(r.key.as_str()),
                r.info.as_ref().map(|i| i.mod_id.as_str()),
            ]
        })
        .flatten()
        .collect();
    // A namespace two carried mods both claim (a library's they both use, such as YUNG's API's
    // `yungsapi`, whose own jar isn't read) is neither's: either would refuse the other in the
    // game. Its files stay outside both.
    let claims = records.iter().filter(|r| chosen(r)).filter_map(|r| {
        let (jar, info) = (r.jar.as_ref()?, r.info.as_ref()?);
        let pack_mod = |m: &str| in_pack.contains(m);
        let owned = if r.path.split('/').next() == Some("datapacks") {
            carry::datapack_namespaces_claimed(jar, &info.mod_id, &pack_mod)
        } else {
            carry::namespaces_claimed(jar, &info.mod_id, &pack_mod)
        }
        .ok()?;
        Some((r.key.as_str(), owned))
    });
    let shared = shared_namespaces(claims, log);
    let in_pack = |m: &str| in_pack.contains(m) || shared.contains(m);
    let pack = carry::Pack {
        loaded: &loaded,
        mods: &in_pack,
    };
    let mut done = 0;
    for r in &records {
        let chosen = chosen(r);
        let (Some(jar), Some(info)) = (&r.jar, &r.info) else {
            if chosen && !only.is_empty() {
                log(format!(
                    "{}: no jar (put it in {}/jars/)",
                    r.key,
                    work.display()
                ));
            }
            continue;
        };
        if !chosen {
            continue;
        }
        // The oracle's facts for the mods it ran; a mod it did not load (a
        // library missing) keeps the guesses from its blockstates.
        let prefix = format!("{}:", info.mod_id);
        let seen = |f: &&oracle::Facts| {
            f.blocks
                .keys()
                .chain(f.items.keys())
                .any(|k| k.starts_with(&prefix))
        };
        let out = work.join("mods").join(&r.key);
        let facts = facts.as_ref().filter(seen);
        let result = match (
            r.path.split('/').next() == Some("datapacks"),
            state_schemas.as_ref(),
        ) {
            (true, Some(schemas)) => carry::carry_datapack_with_state_schemas(
                jar,
                info,
                &out,
                facts,
                &allowed,
                &pack,
                &conventions,
                schemas,
            ),
            (false, Some(schemas)) => carry::carry_with_state_schemas(
                jar,
                info,
                &out,
                facts,
                &allowed,
                &pack,
                &conventions,
                schemas,
            ),
            (true, None) => {
                carry::carry_datapack(jar, info, &out, facts, &allowed, &pack, &conventions)
            }
            (false, None) => carry::carry(jar, info, &out, facts, &allowed, &pack, &conventions),
        };
        match result {
            Ok(report) => {
                done += 1;
                log(format!(
                    "{}: {} files, {} recipes upgraded ({} kept), {} loot tables upgraded ({} added), {} item definitions added, {} blocks ({} left out, {} with differences), {} items ({} left out), {} convention tags, {} outside",
                    r.key,
                    report.copied,
                    report.recipes_upgraded,
                    report.recipes_kept.len(),
                    report.loot_tables_upgraded,
                    report.loot_tables_added,
                    report.item_definitions_added,
                    report.blocks.len(),
                    report.blocks_unknown.len(),
                    report.differences.len(),
                    report.items,
                    report.items_unknown.len(),
                    report.convention_tags,
                    report.outside.len()
                ));
            }
            Err(e) => log(format!("{}: {e}", r.key)),
        }
    }
    let merged_datapacks = merge_datapack_namespaces(work, &records, log)?;
    if merged_datapacks > 0 {
        log(format!(
            "merged {merged_datapacks} data pack namespace(s) into their existing content owner"
        ));
    }
    let recipes_excluded = validate_carried_recipes(work, resources, log)?;
    if recipes_excluded > 0 {
        log(format!(
            "{recipes_excluded} recipes excluded: their result item is not registered by vanilla or a carried mod"
        ));
    }
    log(format!(
        "{done} mods carried over into {}",
        work.join("mods").display()
    ));
    Ok(done)
}

/// Item ids available to the content root built from the carries in `<work>`.
/// Only API declarations add mod items; custom Java-registered items that have
/// no carried declaration are deliberately absent from this set.
fn carried_item_ids(
    work: &Path,
    resources: Option<&Path>,
    log: Log,
) -> Result<BTreeSet<String>, String> {
    let mut known = BTreeSet::new();
    if let Some(resources) = resources {
        let items: Value = read_json(&resources.join("items.json"))?;
        let items = items.as_object().ok_or_else(|| {
            format!(
                "{}: no item registry",
                resources.join("items.json").display()
            )
        })?;
        known.extend(items.keys().cloned());
    }
    let mods = work.join("mods");
    let entries = match fs::read_dir(&mods) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(known),
        Err(error) => return Err(format!("{}: {error}", mods.display())),
    };
    let mut folders: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    let mut owners: Vec<(String, Vec<String>)> = Vec::new();
    for folder in folders {
        let carry = folder.join("carry");
        if !carry.join(horizon_mods::manifest::MANIFEST).is_file() {
            continue;
        }
        let package = match horizon_mods::Package::read(&carry) {
            Ok(package) => package,
            Err(error) => {
                log(format!(
                    "{}: item validation skips content that cannot load: {error}",
                    folder.file_name().unwrap_or_default().to_string_lossy()
                ));
                continue;
            }
        };
        let content = match horizon_mods::Content::read(&carry, &package.manifest) {
            Ok(content) => content,
            Err(error) => {
                log(format!(
                    "{}: item validation skips content that cannot load: {error}",
                    folder.file_name().unwrap_or_default().to_string_lossy()
                ));
                continue;
            }
        };
        let own = (package.manifest.id.as_str(), content.namespaces.as_slice());
        if let Some(error) = owners
            .iter()
            .find_map(|(id, namespaces)| horizon_mods::content::clash((id, namespaces), own))
        {
            log(format!(
                "{}: item validation skips content that cannot load: {error}",
                package.manifest.id
            ));
            continue;
        }
        owners.push((package.manifest.id.clone(), content.namespaces.clone()));
        known.extend(content.items.into_iter().map(|item| item.id));
        known.extend(
            content
                .blocks
                .into_iter()
                .filter(|block| block.item)
                .map(|block| block.id),
        );
    }
    Ok(known)
}

/// Sorted JSON files below a declaration or recipe directory, without
/// following symlinks in a user-owned mod folder.
fn json_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("{}: {error}", dir.display())),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                visit(&path, files)?;
            } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                files.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn recipe_files(data: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(data) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", data.display())),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let namespace = entry.path();
        let metadata = fs::symlink_metadata(&namespace)
            .map_err(|error| format!("{}: {error}", namespace.display()))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            files.extend(json_files(&namespace.join("recipe"))?);
        }
    }
    files.sort();
    Ok(files)
}

/// Excludes recipes whose output is absent from the vanilla item registry and
/// every carried mod's declarations. This is pack-wide so cross-mod recipes
/// work when their output mod is carried, while optional integrations with a
/// missing output mod are reported instead of loading an empty result.
fn validate_carried_recipes(
    work: &Path,
    resources: Option<&Path>,
    log: Log,
) -> Result<usize, String> {
    let known = carried_item_ids(work, resources, log)?;
    let mods = work.join("mods");
    let entries = match fs::read_dir(&mods) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("{}: {error}", mods.display())),
    };
    let mut excluded = 0;
    for entry in entries.flatten() {
        let folder = entry.path();
        let carry = folder.join("carry");
        let report_path = folder.join("carry-report.json");
        if !report_path.is_file() || !carry.join("horizon-mod.toml").is_file() {
            continue;
        }
        let mut report: Value = read_json(&report_path)?;
        let mut mod_excluded = Vec::new();
        for recipe_path in recipe_files(&carry.join("data"))? {
            let recipe: Value = match fs::read(&recipe_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            {
                Some(recipe) => recipe,
                None => continue,
            };
            let result = recipe["result"]
                .as_str()
                .or_else(|| recipe["result"]["id"].as_str())
                .or_else(|| recipe["result"]["item"].as_str());
            let Some(result) = result else {
                continue;
            };
            let id = if result.contains(':') {
                result.to_owned()
            } else {
                format!("minecraft:{result}")
            };
            if known.contains(&id) {
                continue;
            }
            if id.starts_with("minecraft:") && resources.is_none() {
                continue;
            }
            let relative = recipe_path
                .strip_prefix(&carry)
                .map_err(|error| error.to_string())?;
            let outside = folder.join("outside").join(relative);
            if let Some(parent) = outside.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            if outside.exists() {
                fs::remove_file(&outside)
                    .map_err(|error| format!("{}: {error}", outside.display()))?;
            }
            fs::rename(&recipe_path, &outside)
                .map_err(|error| format!("{}: {error}", recipe_path.display()))?;
            mod_excluded.push(relative.to_string_lossy().into_owned());
        }
        if mod_excluded.is_empty() {
            continue;
        }
        mod_excluded.sort();
        let copied = report["copied"].as_u64().unwrap_or(0);
        report["copied"] = Value::from(copied.saturating_sub(mod_excluded.len() as u64));
        let listed = report["recipes_unregistered"]
            .as_array_mut()
            .ok_or_else(|| {
                format!(
                    "{}: recipes_unregistered is not an array",
                    report_path.display()
                )
            })?;
        for path in &mod_excluded {
            if !listed.iter().any(|value| value.as_str() == Some(path)) {
                listed.push(Value::String(path.clone()));
            }
        }
        listed.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        write_json(&report_path, &report)?;
        excluded += mod_excluded.len();
        log(format!(
            "{}: {} recipes use items absent from the vanilla and carried registries",
            folder.file_name().unwrap_or_default().to_string_lossy(),
            mod_excluded.len()
        ));
    }
    Ok(excluded)
}

fn merge_datapack_namespaces(
    work: &Path,
    records: &[inventory::Record],
    log: Log,
) -> Result<usize, String> {
    let mods_root = work.join("mods");
    let Ok(dir) = fs::read_dir(&mods_root) else {
        return Ok(0);
    };
    let mut entries: Vec<_> = dir
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{}: {e}", mods_root.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    let datapack_keys: BTreeSet<&str> = records
        .iter()
        .filter(|record| record.path.split('/').next() == Some("datapacks"))
        .map(|record| record.key.as_str())
        .collect();
    let mut owners = BTreeMap::<String, PathBuf>::new();
    for entry in &entries {
        let key = entry.file_name().to_string_lossy().into_owned();
        if datapack_keys.contains(key.as_str()) {
            continue;
        }
        let carry = entry.path().join("carry");
        let Ok(text) = fs::read_to_string(carry.join(horizon_mods::manifest::MANIFEST)) else {
            continue;
        };
        let Ok(manifest) = horizon_mods::Manifest::parse(&text) else {
            continue;
        };
        for namespace in std::iter::once(manifest.id).chain(manifest.namespaces) {
            owners.entry(namespace).or_insert_with(|| carry.clone());
        }
    }

    let mut datapacks: Vec<_> = records
        .iter()
        .filter(|record| record.path.split('/').next() == Some("datapacks"))
        .collect();
    // DataPackLoader uses file-name order when no explicit level order exists.
    datapacks.sort_by(|a, b| a.path.cmp(&b.path));
    let mut merged = 0;
    for record in datapacks {
        let carry = mods_root.join(&record.key).join("carry");
        let manifest_path = carry.join(horizon_mods::manifest::MANIFEST);
        let Ok(text) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(mut manifest) = horizon_mods::Manifest::parse(&text) else {
            continue;
        };
        let owned: Vec<String> = std::iter::once(manifest.id.clone())
            .chain(manifest.namespaces.iter().cloned())
            .collect();
        let conflicts: BTreeMap<String, PathBuf> = owned
            .iter()
            .filter_map(|namespace| {
                owners
                    .get(namespace)
                    .filter(|owner| *owner != &carry)
                    .map(|owner| (namespace.clone(), owner.clone()))
            })
            .collect();
        if conflicts.is_empty() {
            for namespace in owned {
                owners.entry(namespace).or_insert_with(|| carry.clone());
            }
            continue;
        }

        for (namespace, owner) in &conflicts {
            merge_namespace_tree(&carry, owner, namespace)?;
            merged += 1;
            log(format!(
                "{}: merged namespace {namespace} into {}",
                record.key,
                owner.display()
            ));
        }

        let id_conflicts = conflicts.contains_key(&manifest.id);
        manifest.namespaces.retain(|ns| !conflicts.contains_key(ns));
        if id_conflicts {
            manifest.id = record.key.clone();
        }
        let mut value: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
        value["id"] = toml::Value::String(manifest.id.clone());
        value["namespaces"] = toml::Value::Array(
            manifest
                .namespaces
                .iter()
                .cloned()
                .map(toml::Value::String)
                .collect(),
        );
        let updated = toml::to_string_pretty(&value).map_err(|e| e.to_string())?;
        horizon_mods::Manifest::parse(&updated)?;
        fs::write(&manifest_path, updated)
            .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
        for namespace in std::iter::once(manifest.id).chain(manifest.namespaces) {
            owners.entry(namespace).or_insert_with(|| carry.clone());
        }
    }
    Ok(merged)
}

fn merge_namespace_tree(source: &Path, destination: &Path, namespace: &str) -> Result<(), String> {
    fn copy_tree(source: &Path, destination: &Path, source_root: &Path) -> Result<(), String> {
        let mut entries: Vec<_> = fs::read_dir(source)
            .map_err(|e| format!("{}: {e}", source.display()))?
            .flatten()
            .collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let source_path = entry.path();
            let metadata = fs::symlink_metadata(&source_path)
                .map_err(|e| format!("{}: {e}", source_path.display()))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            let target = destination.join(entry.file_name());
            if metadata.is_dir() {
                fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
                copy_tree(&source_path, &target, source_root)?;
            } else if metadata.is_file() {
                let relative = source_path
                    .strip_prefix(source_root)
                    .map_err(|e| e.to_string())?;
                if target.exists() && relative.starts_with("tags") {
                    merge_tag_file(&source_path, &target)?;
                } else {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("{}: {e}", parent.display()))?;
                    }
                    fs::copy(&source_path, &target).map_err(|e| {
                        format!("{} -> {}: {e}", source_path.display(), target.display())
                    })?;
                }
            }
        }
        Ok(())
    }

    for top in ["assets", "data"] {
        let from = source.join(top).join(namespace);
        if !from.is_dir() {
            continue;
        }
        let to = destination.join(top).join(namespace);
        fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
        copy_tree(&from, &to, &from)?;
    }
    Ok(())
}

fn merge_tag_file(source: &Path, destination: &Path) -> Result<(), String> {
    let incoming: serde_json::Value = serde_json::from_slice(
        &fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?,
    )
    .map_err(|e| format!("{}: {e}", source.display()))?;
    if incoming["replace"] == true {
        fs::copy(source, destination)
            .map_err(|e| format!("{} -> {}: {e}", source.display(), destination.display()))?;
        return Ok(());
    }
    let previous: serde_json::Value = serde_json::from_slice(
        &fs::read(destination).map_err(|e| format!("{}: {e}", destination.display()))?,
    )
    .map_err(|e| format!("{}: {e}", destination.display()))?;
    let mut values = previous["values"].as_array().cloned().unwrap_or_default();
    for value in incoming["values"].as_array().into_iter().flatten() {
        if !values.contains(value) {
            values.push(value.clone());
        }
    }
    let mut merged = incoming;
    merged["values"] = serde_json::Value::Array(values);
    if let Some(object) = merged.as_object_mut() {
        object.remove("replace");
    }
    fs::write(
        destination,
        serde_json::to_vec_pretty(&merged).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("{}: {e}", destination.display()))
}

/// Which carried-over mods (`mods/<key>/carry` folders) the game loads together, in the order
/// given: as the game does, the first of two owning one namespace stays on. Returns the ones on,
/// and a line for each one left off.
pub fn loadable(mods: &[std::path::PathBuf]) -> (Vec<std::path::PathBuf>, Vec<String>) {
    let mut on: Vec<(std::path::PathBuf, String, Vec<String>)> = Vec::new();
    let mut off = Vec::new();
    for path in mods {
        let manifest = fs::read_to_string(path.join(horizon_mods::manifest::MANIFEST))
            .map_err(|e| e.to_string())
            .and_then(|t| horizon_mods::Manifest::parse(&t));
        let m = match manifest {
            Ok(m) => m,
            Err(e) => {
                off.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let owned: Vec<String> = std::iter::once(m.id.clone())
            .chain(m.namespaces.iter().cloned())
            .collect();
        let clash = on
            .iter()
            .find_map(|(_, id, ns)| horizon_mods::content::clash((id, ns), (&m.id, &owned)));
        match clash {
            Some(why) => off.push(format!("{} not loaded: {why}", m.id)),
            None => on.push((path.clone(), m.id, owned)),
        }
    }
    (on.into_iter().map(|(p, _, _)| p).collect(), off)
}

/// The namespaces more than one mod claims, from each mod's (key, claimed namespaces).
fn shared_namespaces<'a>(
    claims: impl Iterator<Item = (&'a str, Vec<String>)>,
    log: Log,
) -> BTreeSet<String> {
    let mut by_namespace: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (key, owned) in claims {
        for ns in owned {
            by_namespace.entry(ns).or_default().push(key);
        }
    }
    by_namespace
        .into_iter()
        .filter(|(_, mods)| mods.len() > 1)
        .map(|(ns, mods)| {
            log(format!(
                "namespace {ns} is claimed by {}: left to none of them",
                mods.join(", ")
            ));
            ns
        })
        .collect()
}

/// The pack's title as the report heads it.
pub fn title(pack: &import::Pack) -> String {
    format!(
        "{} ({} {}, Minecraft {})",
        pack.name, pack.loader, pack.loader_version, pack.minecraft
    )
}

/// Step 7: `<work>/coverage.json` and `coverage.md`.
pub fn report(work: &Path, log: Log) -> Result<Vec<report::Row>, String> {
    let pack: import::Pack = read_json(&work.join("pack.json"))?;
    let records: Vec<inventory::Record> = read_json(&work.join("inventory.json"))?;
    let rows = report::build(&records, work);
    write_json(&work.join("coverage.json"), &rows)?;
    let title = title(&pack);
    fs::write(work.join("coverage.md"), report::markdown(&title, &rows))
        .map_err(|e| format!("coverage.md: {e}"))?;
    log(format!(
        "{title}: {} files: {:?}",
        rows.len(),
        report::counts(&rows)
    ));
    Ok(rows)
}

/// What a player's machine can carry or port today: content mods, integrations,
/// and server-only mods in tiers Horizon's mod API supports (`data` 0.2,
/// `logic` 0.3, `machines` 0.4, `deep` 0.5). Integrations may remain partial
/// until their dependencies are ported; keeping them in the pipeline lets
/// their own data be carried and their jars be fetched in the same pass.
pub fn portable(r: &inventory::Record) -> bool {
    matches!(r.class.as_str(), "content" | "integration" | "server-only")
        && matches!(r.tier.as_str(), "data" | "logic" | "machines" | "deep")
}

/// Unknown mods with jars are fetched for a metadata and file-census pass.
/// That pass can establish whether they add content before carry-over starts.
pub fn inspectable(r: &inventory::Record) -> bool {
    portable(r) || r.class == "unknown"
}

/// Resource, shader and data packs are carried into the profile as files, not
/// as `.hmod` content. Fetch them alongside content mods so the profile can
/// install them in the appropriate game location.
pub fn fetchable(r: &inventory::Record) -> bool {
    inspectable(r)
        || matches!(
            r.path.split('/').next(),
            Some("resourcepacks" | "shaderpacks" | "datapacks")
        )
}

/// Everything after the import, for the portable mods: their jars (`fetch`,
/// from `from` when the player has them), the inventory again with the jars
/// read, the carry-over and the report. `resources` is the prepared
/// version the ports will play on.
pub fn port(
    work: &Path,
    resources: Option<&Path>,
    from: Option<&Path>,
    log: Log,
) -> Result<(fetch::Fetched, Vec<report::Row>), String> {
    let records = match read_json::<Vec<inventory::Record>>(&work.join("inventory.json")) {
        Ok(r) => r,
        Err(_) => inventory(work, false, log)?,
    };
    let fetched = fetch::fetch(work, &records, from, &fetchable, log)?;
    // The jars tell classes and tiers better than metadata alone. Unknown
    // candidates are included so their manifests and file census can classify.
    let records = inventory(work, false, log)?;
    stage_profile_pack_files(work, &records, log)?;
    let only: Vec<String> = records
        .iter()
        .filter(|r| portable(r) && r.jar.is_some())
        .map(|r| r.key.clone())
        .collect();
    if only.is_empty() {
        log("no portable mod has its jar: nothing to carry over".into());
    } else {
        carry(work, resources, &only, log)?;
    }
    Ok((fetched, report(work, log)?))
}

fn stage_profile_pack_files(
    work: &Path,
    records: &[inventory::Record],
    log: Log,
) -> Result<(), String> {
    for record in records.iter().filter(|r| {
        matches!(
            r.path.split('/').next(),
            Some("resourcepacks" | "shaderpacks" | "datapacks")
        )
    }) {
        let Some(source) = record.jar.as_ref().filter(|path| path.is_file()) else {
            continue;
        };
        let Some(name) = record.path.rsplit('/').next() else {
            continue;
        };
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
        {
            continue;
        }
        let kind = record.path.split('/').next().unwrap_or_default();
        let destination = work.join("pack-overrides").join(kind).join(name);
        if source == &destination {
            continue;
        }
        let parent = destination.parent().ok_or("asset path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        fs::copy(source, &destination)
            .map_err(|e| format!("{} -> {}: {e}", source.display(), destination.display()))?;
        log(format!("{}: staged for the profile", record.path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_namespace_two_mods_claim_is_neither_s() {
        let claims = [
            ("betterdungeons", vec!["yungsapi".to_owned()]),
            ("betterfortresses", vec!["yungsapi".to_owned()]),
            (
                "towns_and_towers",
                vec!["kaisyn".to_owned(), "towns_and_towers".to_owned()],
            ),
        ];
        let shared = shared_namespaces(claims.into_iter(), &|_| {});
        assert_eq!(shared.into_iter().collect::<Vec<_>>(), ["yungsapi"]);
    }

    #[test]
    fn the_first_of_two_mods_owning_a_namespace_loads() {
        let root = std::env::temp_dir().join(format!("port-pack-loadable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let carry = |key: &str, namespaces: &str| {
            let dir = root.join(key).join("carry");
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("horizon-mod.toml"),
                format!(
                    "id = \"{key}\"\nname = \"{key}\"\nversion = \"1\"\napi = \"0.2\"\n\
                     capabilities = [\"registries\"]\nnamespaces = [{namespaces}]\n"
                ),
            )
            .unwrap();
            dir
        };
        let mods = [
            carry("betterdungeons", "\"yungsapi\""),
            carry("betterfortresses", "\"yungsapi\""),
            carry("ruby", ""),
        ];
        let (on, off) = loadable(&mods);
        assert_eq!(on, [mods[0].clone(), mods[2].clone()]);
        assert_eq!(off.len(), 1);
        assert!(off[0].starts_with("betterfortresses not loaded"), "{off:?}");
        fs::remove_dir_all(&root).unwrap();
    }
}

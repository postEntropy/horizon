//! Modpacks: import a Java modpack (a Modrinth `.mrpack`, CurseForge zip, instance or Packwiz folder),
//! show what Horizon runs of it, port that on this computer from the player's own downloads, and
//! play it as a profile of its own (docs/modpacks.md).
//!
//! Each pack lives in `ports/<slug>/` in the game's home: the work folder `horizon-port-pack`
//! writes (pack.json, inventory, the jars, the carried-over mods, the coverage report) and
//! `home/`, the profile's own saves and options. Nothing is uploaded or shared: mod jars are
//! often All Rights Reserved, so the ports stay on the player's machine.
use crate::backend;
use horizon_port_pack::{
    fetch::{Fetched, How},
    import::Pack,
    pipeline::{self, Log},
    report::{Row, Status},
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where imported packs live.
pub fn ports_dir() -> PathBuf {
    backend::game_home().join("ports")
}

/// A pack's folder name: its name in lowercase words joined by dashes.
pub fn slug(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if words.is_empty() {
        "pack".into()
    } else {
        words.join("-")
    }
}

/// An imported pack, as its work folder has it.
pub struct Imported {
    pub slug: String,
    pub work: PathBuf,
    pub pack: Pack,
    pub rows: Vec<Row>,
    pub fetched: Fetched,
    /// The carried-over mods the profile loads.
    pub mods: Vec<PathBuf>,
    /// Mods left off, with why: a ported one an earlier one owns a namespace of, or one whose
    /// jar only CurseForge serves when no API key or local copy is available.
    pub left_off: Vec<String>,
    /// Mods classified as content and supported by a Horizon mod API tier.
    pub portable: usize,
    /// Unknown records whose jars can be fetched and inspected before carrying over.
    pub inspectable: usize,
    /// Resource, shader and data packs fetched into this profile.
    pub assets: usize,
}

impl Imported {
    pub fn title(&self) -> &str {
        if self.pack.name.is_empty() {
            &self.slug
        } else {
            &self.pack.name
        }
    }
    pub fn count(&self, status: Status) -> usize {
        self.rows.iter().filter(|r| r.status == status).count()
    }
    /// "Minecraft 1.21.1 · neoforge 21.1.259 · 496 mods".
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.pack.minecraft.is_empty() {
            parts.push(format!("Minecraft {}", self.pack.minecraft));
        }
        if !self.pack.loader.is_empty() {
            parts.push(format!("{} {}", self.pack.loader, self.pack.loader_version));
        }
        parts.push(format!("{} files", self.pack.files.len()));
        parts.join("  ·  ")
    }
    /// What getting the jars did, in one line, or None before it ran.
    pub fn jars(&self) -> Option<String> {
        if self.fetched.files.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for (how, what) in [
            (How::Downloaded, "downloaded"),
            (How::Local, "copied from your folder"),
            (How::Present, "already here"),
            (
                How::NeedsKey,
                "CurseForge API key needed (set HORIZON_CURSEFORGE_API_KEY or choose your \
                 installed instance)",
            ),
            (How::NotFound, "not found"),
            (How::Failed, "failed"),
        ] {
            let n = self.fetched.count(how);
            if n > 0 {
                parts.push(format!("{n} {what}"));
            }
        }
        Some(format!("Jars: {}.", parts.join(", ")))
    }
}

/// Reads the pack in `work`.
pub fn load(work: &Path) -> Result<Imported, String> {
    let pack: Pack = pipeline::read_json(&work.join("pack.json"))?;
    let rows: Vec<Row> = pipeline::read_json(&work.join("coverage.json")).unwrap_or_default();
    let records: Vec<horizon_port_pack::inventory::Record> =
        pipeline::read_json(&work.join("inventory.json")).unwrap_or_default();
    let (mods, mut left_off) = pipeline::loadable(&carried(work));
    let fetched = Fetched::load(work);
    left_off.extend(
        fetched
            .files
            .iter()
            .filter(|o| o.how == How::NeedsKey)
            .map(|o| format!("{}: HORIZON_CURSEFORGE_API_KEY is not set", o.key)),
    );
    Ok(Imported {
        slug: work
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        work: work.to_path_buf(),
        pack,
        rows,
        fetched,
        mods,
        left_off,
        portable: records.iter().filter(|r| pipeline::portable(r)).count(),
        inspectable: records
            .iter()
            .filter(|r| r.class == "unknown" && r.jar.is_none())
            .count(),
        assets: records
            .iter()
            .filter(|r| matches!(r.path.split('/').next(), Some("resourcepacks" | "shaderpacks" | "datapacks")))
            .count(),
    })
}

/// Every imported pack, by name.
pub fn list() -> Vec<Imported> {
    let mut packs: Vec<Imported> = fs::read_dir(ports_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| load(&e.path()).ok())
        .collect();
    packs.sort_by_key(|p| p.title().to_ascii_lowercase());
    packs
}

/// The carried-over mods in a work folder (`mods/<id>/carry/` with its manifest).
pub fn carried(work: &Path) -> Vec<PathBuf> {
    let mut mods: Vec<PathBuf> = fs::read_dir(work.join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path().join("carry"))
        .filter(|c| c.join("horizon-mod.toml").is_file())
        .collect();
    // A pack-file archive can have the same data namespace as its owning mod.
    // Keep that mod first so a namespace clash never turns the data pack into
    // the winner and prevents the mod's content from loading.
    mods.sort_by_key(|carry| {
        (
            carry
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name.to_string_lossy().starts_with("datapack_")),
            carry.clone(),
        )
    });
    mods
}

/// Imports the pack at `path` into `ports/<slug>/` and writes its coverage report from
/// metadata (Modrinth answers by file hash; nothing is downloaded yet). Importing the same pack
/// again keeps the jars and ports it already has.
pub fn import(path: &Path, log: Log) -> Result<Imported, String> {
    let pack = horizon_port_pack::import::import(path)?;
    let name = if pack.name.is_empty() || pack.name == "modlist" {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        pack.name.clone()
    };
    let work = ports_dir().join(slug(&name));
    log(format!("Importing {name} into {}", work.display()));
    pipeline::import(path, &work, log)?;
    let applied = merge_pack_overrides(&work)?;
    enable_new_resource_packs(&profile_home(&work), &applied)?;
    if !applied.is_empty() {
        log(format!("{} pack files copied to its profile", applied.len()));
    }
    pipeline::inventory(&work, false, log)?;
    pipeline::report(&work, log)?;
    load(&work)
}

/// Gets jars for portable mods and unclassified candidates, then carries over the content that
/// matches Horizon's templates and writes the report again.
pub fn port(
    work: &Path,
    resources: Option<&Path>,
    from: Option<&Path>,
    log: Log,
) -> Result<Imported, String> {
    pipeline::port(work, resources, from, log)?;
    let applied = merge_pack_overrides(work)?;
    enable_new_resource_packs(&profile_home(work), &applied)?;
    load(work)
}

/// What the mod browser does with a modpack: imports it, then ports what this computer can
/// (from the pack's links, Modrinth, or CurseForge when its API key is set), so it plays straight
/// away. A port that fails leaves the pack imported; the error says so.
pub fn import_and_port(
    path: &Path,
    resources: Option<&Path>,
    log: Log,
) -> Result<Imported, String> {
    let pack = import(path, log)?;
    if pack.portable == 0 && pack.inspectable == 0 && pack.assets == 0 {
        return Ok(pack);
    }
    port(&pack.work, resources, None, log)
        .map_err(|e| format!("{} is imported, but porting it failed: {e}", pack.title()))
}

/// One line on what an import and port gave: the mods carried over and the ones that weren't.
pub fn outcome(pack: &Imported) -> String {
    let not = pack.count(Status::NotYet);
    format!(
        "{} is imported{}: {} of its {} files play in Horizon ({} ported, {} partly), {} don't \
         yet, {} aren't needed{}. Its list says why.",
        pack.title(),
        if pack.mods.is_empty() {
            ""
        } else {
            " and ported"
        },
        pack.count(Status::Ported) + pack.count(Status::Partial),
        pack.rows.len(),
        pack.count(Status::Ported),
        pack.count(Status::Partial),
        not,
        pack.count(Status::NotNeeded),
        match pack.left_off.len() {
            0 => String::new(),
            n => format!("; {n} are left off (listed under the pack)"),
        },
    )
}

/// Removes an imported pack: its jars, ports and the profile's saves.
pub fn remove(work: &Path) -> Result<(), String> {
    if work.parent() != Some(ports_dir().as_path()) {
        return Err(format!("{} is not an imported pack", work.display()));
    }
    fs::remove_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))
}

/// The profile's own game home: saves, options.txt, logs.
pub fn profile_home(work: &Path) -> PathBuf {
    work.join("home")
}

/// Adds pack configs/scripts to the isolated profile without replacing files
/// the player already customized on an earlier import.
fn merge_pack_overrides(work: &Path) -> Result<Vec<PathBuf>, String> {
    let source = work.join("pack-overrides");
    if !source.is_dir() {
        return Ok(Vec::new());
    }
    let home = profile_home(work);
    fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    let mut copied = merge_tree(&source, &source, &home)?;
    copied.sort();
    Ok(copied)
}

fn merge_tree(root: &Path, source: &Path, destination: &Path) -> Result<Vec<PathBuf>, String> {
    let mut copied = Vec::new();
    for entry in fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let target = destination.join(relative);
        if metadata.is_dir() {
            fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            copied.extend(merge_tree(root, &path, destination)?);
        } else if metadata.is_file() && !target.exists() {
            let parent = target.parent().ok_or_else(|| format!("{}: no parent", target.display()))?;
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", target.display()))?;
            copied.push(relative.to_path_buf());
        }
    }
    Ok(copied)
}

fn enable_new_resource_packs(home: &Path, files: &[PathBuf]) -> Result<(), String> {
    let ids: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let mut parts = path.components();
            (parts.next()?.as_os_str() == "resourcepacks")
                .then(|| parts.next().map(|name| format!("file/{}", name.as_os_str().to_string_lossy())))
                .flatten()
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let options_path = home.join("options.txt");
    let existing = match fs::read_to_string(&options_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("{}: {error}", options_path.display())),
    };
    let mut lines: Vec<String> = existing.lines().map(str::to_owned).collect();
    let index = lines.iter().position(|line| line.starts_with("resourcePacks:"));
    let mut selected = if let Some(index) = index {
        let value = lines[index]
            .split_once(':')
            .map(|(_, value)| value.trim())
            .ok_or_else(|| format!("{}: malformed resourcePacks setting", options_path.display()))?;
        serde_json::from_str::<Vec<String>>(value)
            .map_err(|e| format!("{}: resourcePacks: {e}", options_path.display()))?
    } else {
        vec!["vanilla".into()]
    };
    for id in ids {
        if !selected.contains(&id) {
            selected.push(id);
        }
    }
    let setting = format!("resourcePacks:{}", serde_json::to_string(&selected).map_err(|e| e.to_string())?);
    if let Some(index) = index {
        lines[index] = setting;
    } else {
        lines.push(setting);
    }
    let mut text = lines.join("\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    fs::write(&options_path, text).map_err(|e| format!("{}: {e}", options_path.display()))
}

/// Turns the game's command into this pack's profile: its own home (the player's options.txt
/// copied in the first time, so keys and video settings carry over) and its carried-over mods.
pub fn apply_profile(cmd: &mut Command, work: &Path) -> Result<(), String> {
    // Only mods that load together: the game would refuse the later of two sharing a namespace.
    let (mods, _) = pipeline::loadable(&carried(work));
    let home = profile_home(work);
    let has_pack_files = ["resourcepacks", "shaderpacks", "datapacks", "config", "defaultconfigs", "kubejs"]
        .iter()
        .any(|folder| home.join(folder).is_dir());
    if mods.is_empty() && !has_pack_files {
        return Err("Nothing is ported yet: port the pack first.".into());
    }
    fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    let options = home.join("options.txt");
    let own = backend::game_home().join("options.txt");
    if !options.exists() && own.is_file() {
        let _ = fs::copy(own, &options);
    }
    cmd.current_dir(&home).env("HORIZON_HOME", &home);
    for m in mods {
        cmd.arg("--dev-mod").arg(m);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("All the Mods 10"), "all-the-mods-10");
        assert_eq!(slug("Create: Above & Beyond!"), "create-above-beyond");
        assert_eq!(slug("  "), "pack");
    }

    #[test]
    fn curseforge_files_without_a_key_are_left_off_with_setup_hint() {
        use horizon_port_pack::fetch::Outcome;
        let work = std::env::temp_dir().join(format!("horizon-launcher-cf-{}", std::process::id()));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).unwrap();
        pipeline::write_json(&work.join("pack.json"), &Pack::default()).unwrap();
        let fetched = Fetched {
            files: vec![Outcome {
                key: "allthemodium".into(),
                file: "mods/allthemodium.jar".into(),
                how: How::NeedsKey,
                detail: String::new(),
            }],
        };
        pipeline::write_json(&work.join("fetch.json"), &fetched).unwrap();
        let pack = load(&work).unwrap();
        assert_eq!(
            pack.left_off,
            ["allthemodium: HORIZON_CURSEFORGE_API_KEY is not set"]
        );
        assert!(pack
            .jars()
            .unwrap()
            .contains("CurseForge API key needed"));
        fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn a_profile_has_its_own_home_and_mods() {
        let root =
            std::env::temp_dir().join(format!("horizon-launcher-pack-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let work = root.join("ports/small");
        let mut cmd = Command::new("true");
        assert!(apply_profile(&mut cmd, &work).is_err());
        for (id, namespaces) in [("a", ""), ("b", ""), ("d", "\"a\"")] {
            let carry = work.join("mods").join(id).join("carry");
            fs::create_dir_all(&carry).unwrap();
            fs::write(
                carry.join("horizon-mod.toml"),
                format!(
                    "id = \"{id}\"\nname = \"{id}\"\nversion = \"1\"\napi = \"0.2\"\n\
                     capabilities = [\"registries\"]\nnamespaces = [{namespaces}]\n"
                ),
            )
            .unwrap();
        }
        // Left out: no manifest (c), and one owning a namespace of an earlier mod (d).
        fs::create_dir_all(work.join("mods/c/carry")).unwrap();
        apply_profile(&mut cmd, &work).unwrap();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args.len(), 4);
        assert_eq!(
            Path::new(&args[1]),
            work.join("mods").join("a").join("carry")
        );
        assert_eq!(
            Path::new(&args[3]),
            work.join("mods").join("b").join("carry")
        );
        assert_eq!(cmd.get_current_dir(), Some(profile_home(&work).as_path()));
        assert!(profile_home(&work).is_dir());
        fs::remove_dir_all(&root).unwrap();
    }
}

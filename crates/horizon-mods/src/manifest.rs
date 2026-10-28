//! `horizon-mod.toml`, a mod's manifest, and the `.hmod` file around it.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fmt,
    io::Read,
    path::{Path, PathBuf},
};

/// The manifest's file name, beside `mod.wasm`.
pub const MANIFEST: &str = "horizon-mod.toml";
/// The component's file name in a `.hmod` or a mod folder.
pub const WASM: &str = "mod.wasm";
/// The API (WIT package `horizon:mods`) versions this build runs: a mod
/// built for any 0.1.x SDK.
pub const API: &str = "0.1";
/// The manifest `api` lines this build loads. 0.2 is 0.1 plus content:
/// `assets/` and `data/` in the `.hmod`, with blocks and items declared in
/// `data/<id>/horizon/` (docs/modpacks.md). 0.3 adds block/item/server
/// callbacks, typed commands, keybindings, crosshair target, inventory read
/// and `[config]` settings. 0.4 adds block entities (`block-entities`),
/// capabilities (FE/fluid/item/attachments/accessories), menus (`ui`),
/// networking (`net`), custom recipes (`recipes`) and 3D render overlays.
/// 0.5 adds custom entities (`entities`), animated models and draw lists
/// (`models`), dimensions (`dimensions`) and named vanilla hooks
/// (`vanilla-hooks`) (docs/mods.md).
pub const APIS: [&str; 5] = [API, "0.2", "0.3", "0.4", "0.5"];
/// Namespaces no mod may claim: vanilla's, the convention tags' (mods add
/// to those, docs/mods.md) and the game's own.
const RESERVED_NAMESPACES: [&str; 5] = ["minecraft", "c", "forge", "horizon", "neoforge"];
/// The biggest `mod.wasm` a mod may ship.
const MAX_WASM: u64 = 32 << 20;

/// A typed value in `[config.<key>]`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// One entry in `[config.<key>]` in `horizon-mod.toml` (API 0.3).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConfigSchema {
    #[serde(rename = "type")]
    pub kind: String,
    pub default: ConfigValue,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `[a-z0-9_]`, 1 to 64 characters: the mod's folder, log and settings key.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    /// The API line the mod was built for: `0.1`, `0.2` for content, `0.3`
    /// for logic, `0.4` for machines and block entities.
    pub api: String,
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Namespaces besides `id` that the mod's `assets/` and `data/` use, for
    /// a content mod whose files sit under other names than its id (Towns
    /// and Towers ships `kaisyn` and `towns_and_towers`). No two content mods
    /// that are on may share one (`content::clash`).
    #[serde(default)]
    pub namespaces: Vec<String>,
    /// Namespaces this mod may add resource files to without owning them.
    /// These files overlay the owning mod's namespace; ownership and clash
    /// checks still use only `id` and `namespaces`.
    #[serde(default)]
    pub contributes: Vec<String>,
    /// Typed settings schema (`[config.<key>]`, API 0.3).
    #[serde(default)]
    pub config: BTreeMap<String, ConfigSchema>,
    /// Optional origin note when ported by `horizon-port-pack`.
    #[serde(default, rename = "ported-from")]
    pub ported_from: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Runs in the player's game only (HUD, keys, chat commands).
    #[default]
    Client,
    /// Runs where the world is simulated (rules, blocks).
    Server,
    /// Both, and the server tells clients which version it expects.
    Both,
}

/// What a mod may do beyond writing its log. Each is a WIT interface the
/// component imports; the player sees them before turning the mod on.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    GameRead,
    Chat,
    Hud,
    Storage,
    /// New blocks and items from the mod's `data/` (API 0.2): no WIT
    /// interface, the game reads them before the world loads.
    Registries,
    /// Typed settings declared in `[config.<key>]` (API 0.3).
    Config,
    /// Server-side block, item, drop, sound and rule actions (API 0.3).
    Server,
    /// Custom keybindings in Controls (API 0.3).
    Keys,
    /// Crosshair block and entity target (API 0.3).
    Target,
    /// Read-only view of the player's inventory and open container (API 0.3).
    InventoryRead,
    /// Block entities for the mod's own blocks (API 0.4): the `machine`
    /// interface and the `block-entities` export.
    BlockEntities,
    /// Per-face item insertion and extraction rules for mod machine slots (API 0.4).
    ItemTransfer,
    /// Read and write opaque Minecraft data components on machine slots (API 0.4).
    ItemData,
    /// Energy (FE), fluid tanks, item slots, data attachments and accessories (API 0.4).
    Capabilities,
    /// Declarative container menus and screens (API 0.4).
    Ui,
    /// Bounded mod packets between client and server (API 0.4).
    Net,
    /// Recipe graph queries and custom machine recipe types (API 0.4).
    Recipes,
    /// Read-only block, biome and light queries around the player (API 0.3).
    WorldRead,
    /// Declarative 3D wireframe boxes and ghost-block previews (API 0.4).
    Render,
    /// Custom entity types, reusable vanilla AI goals and entity steering (API 0.5).
    Entities,
    /// Animated models (GeckoLib / Bedrock bones) and declarative draw lists (API 0.5).
    Models,
    /// Custom dimensions, portals and cross-dimension travel (API 0.5).
    Dimensions,
    /// Named hooks into vanilla behaviour (loot, damage, XP, crops, explosions) (API 0.5).
    VanillaHooks,
}
impl Capability {
    pub const ALL: [Capability; 23] = [
        Capability::GameRead,
        Capability::Chat,
        Capability::Hud,
        Capability::Storage,
        Capability::Registries,
        Capability::Config,
        Capability::Server,
        Capability::Keys,
        Capability::Target,
        Capability::InventoryRead,
        Capability::BlockEntities,
        Capability::ItemTransfer,
        Capability::ItemData,
        Capability::Capabilities,
        Capability::Ui,
        Capability::Net,
        Capability::Recipes,
        Capability::WorldRead,
        Capability::Render,
        Capability::Entities,
        Capability::Models,
        Capability::Dimensions,
        Capability::VanillaHooks,
    ];
    /// Minimum manifest `api` version that grants this capability.
    pub fn min_api(self) -> &'static str {
        match self {
            Capability::GameRead | Capability::Chat | Capability::Hud | Capability::Storage => {
                "0.1"
            }
            Capability::Registries => "0.2",
            Capability::Config
            | Capability::Server
            | Capability::Keys
            | Capability::Target
            | Capability::InventoryRead
            | Capability::WorldRead => "0.3",
            Capability::BlockEntities
            | Capability::ItemTransfer
            | Capability::ItemData
            | Capability::Capabilities
            | Capability::Ui
            | Capability::Net
            | Capability::Recipes
            | Capability::Render => "0.4",
            Capability::Entities
            | Capability::Models
            | Capability::Dimensions
            | Capability::VanillaHooks => "0.5",
        }
    }
    /// The WIT interface it grants, in `horizon:mods`.
    pub fn interface(self) -> &'static str {
        match self {
            Capability::GameRead => "game",
            Capability::Chat => "chat",
            Capability::Hud => "hud",
            Capability::Storage => "storage",
            Capability::Registries => "registries",
            Capability::Config => "config",
            Capability::Server => "server",
            Capability::Keys => "keys",
            Capability::Target => "target",
            Capability::InventoryRead => "inventory",
            Capability::BlockEntities => "machine",
            Capability::ItemTransfer => "item-transfer",
            Capability::ItemData => "item-data",
            Capability::Capabilities => "capabilities",
            Capability::Ui => "ui",
            Capability::Net => "net",
            Capability::Recipes => "recipes",
            Capability::WorldRead => "world-read",
            Capability::Render => "render",
            Capability::Entities => "entities",
            Capability::Models => "models",
            Capability::Dimensions => "dimensions",
            Capability::VanillaHooks => "vanilla-hooks",
        }
    }
    /// What the player reads before turning the mod on: "May <this>".
    pub fn describe(self) -> &'static str {
        match self {
            Capability::GameRead => "see your position, health and the time",
            Capability::Chat => "show you chat messages",
            Capability::Hud => "draw text on your screen",
            Capability::Storage => "save its own settings to disk",
            Capability::Registries => "add blocks and items to your worlds",
            Capability::Config => "expose configurable settings in the Mods screen",
            Capability::Server => "change blocks, drops, sounds and rules in your worlds",
            Capability::Keys => "add keybindings to Controls",
            Capability::Target => "see the block or entity you are looking at",
            Capability::InventoryRead => "see the items in your inventory and open container",
            Capability::BlockEntities => "run its blocks' machines in your worlds",
            Capability::ItemTransfer => "control which sides allow item insertion and extraction",
            Capability::ItemData => "preserve item data components in machine inventories",
            Capability::Capabilities => {
                "exchange energy, fluids, items, attachments and accessories"
            }
            Capability::Ui => "open custom machine and container screens",
            Capability::Net => "send mod packets between client and server",
            Capability::Recipes => "read recipes and register custom machine recipe types",
            Capability::WorldRead => "read blocks, biomes and light around you",
            Capability::Render => "draw 3D boxes and ghost-block previews in the world",
            Capability::Entities => "spawn and control custom entities and mobs",
            Capability::Models => "animate custom 3D models and block-entity attachments",
            Capability::Dimensions => "register custom dimensions and portals",
            Capability::VanillaHooks => "hook into loot drops, combat damage, XP and plant growth",
        }
    }
    /// The capability a component import needs, `Ok(None)` for one every
    /// mod has; `Err` for an import Horizon does not offer.
    pub fn of_import(name: &str) -> Result<Option<Capability>, String> {
        let unknown = || format!("imports {name}, which Horizon does not offer");
        let rest = name.strip_prefix("horizon:mods/").ok_or_else(unknown)?;
        let (interface, version) = rest.split_once('@').ok_or_else(unknown)?;
        if !version.starts_with("0.1.") {
            return Err(format!("needs API {version}; this Horizon runs {API}"));
        }
        if interface == "log" {
            return Ok(None);
        }
        Capability::ALL
            .into_iter()
            .filter(|c| *c != Capability::Registries)
            .find(|c| c.interface() == interface)
            .map(Some)
            .ok_or_else(unknown)
    }
}
impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Capability::GameRead => "game-read",
            Capability::Chat => "chat",
            Capability::Hud => "hud",
            Capability::Storage => "storage",
            Capability::Registries => "registries",
            Capability::Config => "config",
            Capability::Server => "server",
            Capability::Keys => "keys",
            Capability::Target => "target",
            Capability::InventoryRead => "inventory-read",
            Capability::BlockEntities => "block-entities",
            Capability::ItemTransfer => "item-transfer",
            Capability::ItemData => "item-data",
            Capability::Capabilities => "capabilities",
            Capability::Ui => "ui",
            Capability::Net => "net",
            Capability::Recipes => "recipes",
            Capability::WorldRead => "world-read",
            Capability::Render => "render",
            Capability::Entities => "entities",
            Capability::Models => "models",
            Capability::Dimensions => "dimensions",
            Capability::VanillaHooks => "vanilla-hooks",
        };
        f.write_str(name)
    }
}

fn api_rank(api: &str) -> u8 {
    match api {
        "0.1" => 1,
        "0.2" => 2,
        "0.3" => 3,
        "0.4" => 4,
        "0.5" => 5,
        _ => 0,
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Manifest = toml::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        let id_ok = (1..=64).contains(&m.id.len())
            && m.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            return Err(format!(
                "{MANIFEST}: id {:?} is not [a-z0-9_]{{1,64}}",
                m.id
            ));
        }
        if !APIS.contains(&m.api.as_str()) {
            return Err(format!(
                "built for API {}; this Horizon runs {}",
                m.api,
                APIS.join(", ")
            ));
        }
        let rank = api_rank(&m.api);
        for &cap in &m.capabilities {
            let need = cap.min_api();
            if rank < api_rank(need) {
                return Err(format!("{MANIFEST}: capability {cap} needs api = {need:?}"));
            }
        }
        if !m.config.is_empty() && rank < api_rank("0.3") {
            return Err(format!("{MANIFEST}: [config] needs api = \"0.3\""));
        }
        for (key, schema) in &m.config {
            let key_ok = (1..=64).contains(&key.len())
                && key
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
            if !key_ok {
                return Err(format!(
                    "{MANIFEST}: config key {key:?} is not [a-z0-9_.-]{{1,64}}"
                ));
            }
            match (schema.kind.as_str(), &schema.default) {
                ("bool", ConfigValue::Bool(_)) => {}
                ("int", ConfigValue::Int(v)) => {
                    if let Some(min) = schema.min
                        && (*v as f64) < min
                    {
                        return Err(format!("{MANIFEST}: config {key} default is below min"));
                    }
                    if let Some(max) = schema.max
                        && (*v as f64) > max
                    {
                        return Err(format!("{MANIFEST}: config {key} default is above max"));
                    }
                }
                ("float", ConfigValue::Float(v)) => {
                    if let Some(min) = schema.min
                        && *v < min
                    {
                        return Err(format!("{MANIFEST}: config {key} default is below min"));
                    }
                    if let Some(max) = schema.max
                        && *v > max
                    {
                        return Err(format!("{MANIFEST}: config {key} default is above max"));
                    }
                }
                ("string", ConfigValue::String(s)) => {
                    if !schema.choices.is_empty() && !schema.choices.contains(s) {
                        return Err(format!(
                            "{MANIFEST}: config {key} default is not in choices"
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "{MANIFEST}: config {key} type {:?} does not match its default",
                        schema.kind
                    ));
                }
            }
        }
        if !m.namespaces.is_empty() && !m.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{MANIFEST}: namespaces are for content, which needs capability registries"
            ));
        }
        if !m.contributes.is_empty() && !m.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{MANIFEST}: contributes is for content, which needs capability registries"
            ));
        }
        for (i, ns) in m.namespaces.iter().enumerate() {
            // Vanilla's namespace characters, at most as long as an id.
            let chars_ok = (1..=64).contains(&ns.len())
                && ns
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
            if !chars_ok {
                return Err(format!(
                    "{MANIFEST}: namespace {ns:?} is not [a-z0-9_.-]{{1,64}}"
                ));
            }
            if RESERVED_NAMESPACES.contains(&ns.as_str()) {
                return Err(format!(
                    "{MANIFEST}: namespace {ns} is not a mod's to claim (only to add to its tags)"
                ));
            }
            if *ns == m.id || m.namespaces[..i].contains(ns) {
                return Err(format!("{MANIFEST}: namespace {ns} is listed twice"));
            }
        }
        for (i, ns) in m.contributes.iter().enumerate() {
            let chars_ok = (1..=64).contains(&ns.len())
                && ns
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
            if !chars_ok || RESERVED_NAMESPACES.contains(&ns.as_str()) {
                return Err(format!(
                    "{MANIFEST}: contributed namespace {ns:?} is invalid"
                ));
            }
            if m.owned_namespaces().any(|owned| owned == ns) || m.contributes[..i].contains(ns) {
                return Err(format!(
                    "{MANIFEST}: contributed namespace {ns} is already listed"
                ));
            }
        }
        Ok(m)
    }
    /// The namespaces the mod's content owns: its id, then `namespaces`.
    pub fn owned_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.namespaces.iter().map(String::as_str))
    }
    /// Checks that the component's imports need only declared capabilities.
    pub fn check_imports<'a>(
        &self,
        imports: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), String> {
        for name in imports {
            if let Some(c) = Capability::of_import(name)?
                && !self.capabilities.contains(&c)
            {
                return Err(format!("uses capability {c}, missing from {MANIFEST}"));
            }
        }
        Ok(())
    }
}

/// A mod as found on disk: its manifest and component bytes.
pub struct Package {
    pub manifest: Manifest,
    /// None for a content-only mod (capability `registries`, no code).
    pub wasm: Option<Vec<u8>>,
}
impl Package {
    /// A `.hmod` (a zip holding `horizon-mod.toml`, `mod.wasm` and, for
    /// content, `assets/` and `data/`) or a folder holding the same.
    pub fn read(path: &Path) -> Result<Self, String> {
        let (toml, wasm) = if path.is_dir() {
            let toml = std::fs::read_to_string(path.join(MANIFEST));
            let wasm = path.join(WASM);
            let wasm = match wasm.exists() {
                true => Some(std::fs::read(wasm).map_err(|e| format!("{WASM}: {e}"))?),
                false => None,
            };
            (toml.map_err(|e| format!("{MANIFEST}: {e}"))?, wasm)
        } else {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let mut entry = |name: &str| -> Result<Option<Vec<u8>>, String> {
                let f = match zip.by_name(name) {
                    Ok(f) => f,
                    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
                    Err(e) => return Err(format!("{name}: {e}")),
                };
                let mut out = Vec::new();
                f.take(MAX_WASM + 1)
                    .read_to_end(&mut out)
                    .map_err(|e| e.to_string())?;
                Ok(Some(out))
            };
            let toml = entry(MANIFEST)?.ok_or_else(|| format!("{MANIFEST}: missing"))?;
            let toml = String::from_utf8(toml).map_err(|_| format!("{MANIFEST}: not UTF-8"))?;
            (toml, entry(WASM)?)
        };
        if wasm.as_ref().is_some_and(|w| w.len() as u64 > MAX_WASM) {
            return Err(format!("{WASM} is over {} MiB", MAX_WASM >> 20));
        }
        let manifest = Manifest::parse(&toml)?;
        if wasm.is_none() && !manifest.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{WASM}: missing (only a content mod, with capability registries, may have no code)"
            ));
        }
        Ok(Package { manifest, wasm })
    }
    /// The `.hmod` bytes for `manifest_text` and a component.
    pub fn write(manifest_text: &str, wasm: &[u8]) -> Result<Vec<u8>, String> {
        Self::write_files(manifest_text, Some(wasm), &[])
    }
    /// The `.hmod` bytes for `manifest_text`, a component if the mod has
    /// code, and content `files` (`assets/...`, `data/...`) by path.
    pub fn write_files(
        manifest_text: &str,
        wasm: Option<&[u8]>,
        files: &[(&str, &[u8])],
    ) -> Result<Vec<u8>, String> {
        use std::io::Write;
        Manifest::parse(manifest_text)?;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        let err = |e: zip::result::ZipError| e.to_string();
        let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
            zip.start_file(name, options).map_err(err)?;
            zip.write_all(bytes).map_err(|e| e.to_string())
        };
        put(MANIFEST, manifest_text.as_bytes())?;
        if let Some(wasm) = wasm {
            put(WASM, wasm)?;
        }
        for (name, bytes) in files {
            crate::content::check_path(name)?;
            put(name, bytes)?;
        }
        Ok(zip.finish().map_err(err)?.into_inner())
    }
}

/// The `.hmod` files and mod folders in `dir`, by name.
pub fn find(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "hmod") || p.join(MANIFEST).is_file())
        .collect();
    found.sort();
    found
}

/// Turns a `wasm32-unknown-unknown` cdylib built with the SDK into the
/// component the game loads.
#[cfg(feature = "pack")]
pub fn componentize(core: &[u8]) -> Result<Vec<u8>, String> {
    wit_component::ComponentEncoder::default()
        .module(core)
        .and_then(|e| e.validate(true).encode())
        .map_err(|e| format!("{e:#}"))
}

/// The names a component imports: `horizon:mods/game@0.1.0`, ...
#[cfg(feature = "pack")]
pub fn component_imports(component: &[u8]) -> Result<Vec<String>, String> {
    use wit_parser::{WorldItem, WorldKey};
    let wit_component::DecodedWasm::Component(resolve, world) =
        wit_component::decode(component).map_err(|e| format!("{e:#}"))?
    else {
        return Err("a WIT package, not a component".into());
    };
    let names = resolve.worlds[world]
        .imports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), _) | (_, WorldItem::Interface { id, .. }) => resolve
                .id_of(*id)
                .unwrap_or_else(|| resolve.name_world_key(key)),
            _ => resolve.name_world_key(key),
        });
    Ok(names.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
id = "coords_hud"
name = "Coordinates HUD"
version = "0.1.0"
api = "0.1"
capabilities = ["game-read", "hud"]
"#;

    #[test]
    fn parses_and_checks_imports() {
        let m = Manifest::parse(EXAMPLE).unwrap();
        assert_eq!(m.side, Side::Client);
        assert_eq!(m.capabilities, [Capability::GameRead, Capability::Hud]);
        let ok = [
            "horizon:mods/log@0.1.0",
            "horizon:mods/game@0.1.0",
            "horizon:mods/hud@0.1.2",
        ];
        assert_eq!(m.check_imports(ok), Ok(()));
        let chat = m.check_imports(["horizon:mods/chat@0.1.0"]);
        assert_eq!(
            chat,
            Err("uses capability chat, missing from horizon-mod.toml".into())
        );
        assert!(m.check_imports(["wasi:filesystem/types@0.2.0"]).is_err());
        assert!(m.check_imports(["horizon:mods/game@0.2.0"]).is_err());
        let storage = m.check_imports(["horizon:mods/storage@0.1.0"]);
        assert_eq!(
            storage,
            Err("uses capability storage, missing from horizon-mod.toml".into())
        );
        let with_storage = EXAMPLE.replace("\"hud\"", "\"hud\", \"storage\"");
        let m = Manifest::parse(&with_storage).unwrap();
        assert_eq!(m.check_imports(["horizon:mods/storage@0.1.0"]), Ok(()));
    }

    #[test]
    fn refuses_bad_ids_and_apis() {
        let bad_id = EXAMPLE.replace("coords_hud", "Coords HUD");
        assert!(Manifest::parse(&bad_id).unwrap_err().contains("is not"));
        let bad_api = EXAMPLE.replace("api = \"0.1\"", "api = \"0.6\"");
        assert!(Manifest::parse(&bad_api).unwrap_err().contains("API 0.6"));
        let unknown = format!("{EXAMPLE}network = true\n");
        assert!(Manifest::parse(&unknown).is_err());
    }

    #[test]
    fn api_0_5_capabilities() {
        let m05 = r#"
id = "twilightforest"
name = "The Twilight Forest"
version = "1.0.0"
api = "0.5"
side = "both"
capabilities = ["registries", "entities", "models", "dimensions", "vanilla-hooks"]
"#;
        let m = Manifest::parse(m05).unwrap();
        assert_eq!(
            m.check_imports([
                "horizon:mods/entities@0.1.0",
                "horizon:mods/models@0.1.0",
                "horizon:mods/dimensions@0.1.0",
                "horizon:mods/vanilla-hooks@0.1.0",
            ]),
            Ok(())
        );
        let too_old = m05.replace("api = \"0.5\"", "api = \"0.4\"");
        assert!(
            Manifest::parse(&too_old)
                .unwrap_err()
                .contains("needs api = \"0.5\"")
        );
    }

    #[test]
    fn api_0_3_and_0_4_capabilities_and_config() {
        let m03 = r#"
id = "waystones"
name = "Waystones"
version = "1.0.0"
api = "0.3"
side = "both"
capabilities = ["config", "server", "keys", "target", "inventory-read"]
ported-from = "waystones-21.1.jar"

[config.xp_cost]
type = "int"
default = 3
min = 0
max = 30
description = "Levels per teleport"
"#;
        let m = Manifest::parse(m03).unwrap();
        assert_eq!(m.ported_from.as_deref(), Some("waystones-21.1.jar"));
        assert_eq!(m.config["xp_cost"].default, ConfigValue::Int(3));
        assert_eq!(
            m.check_imports([
                "horizon:mods/config@0.1.0",
                "horizon:mods/server@0.1.0",
                "horizon:mods/keys@0.1.0",
                "horizon:mods/target@0.1.0",
                "horizon:mods/inventory@0.1.0",
            ]),
            Ok(())
        );
        let too_old = m03.replace("api = \"0.3\"", "api = \"0.2\"");
        assert!(Manifest::parse(&too_old).unwrap_err().contains("needs api"));

        let m04 = r#"
id = "mekanism"
name = "Mekanism"
version = "1.0.0"
api = "0.4"
side = "both"
capabilities = ["registries", "block-entities", "capabilities", "ui", "net", "recipes", "world-read", "render"]
"#;
        let m = Manifest::parse(m04).unwrap();
        assert_eq!(
            m.check_imports([
                "horizon:mods/machine@0.1.0",
                "horizon:mods/capabilities@0.1.0",
                "horizon:mods/ui@0.1.0",
                "horizon:mods/net@0.1.0",
                "horizon:mods/recipes@0.1.0",
                "horizon:mods/world-read@0.1.0",
                "horizon:mods/render@0.1.0",
            ]),
            Ok(())
        );
    }

    #[test]
    fn checks_extra_namespaces() {
        let content = "id = \"t_and_t\"\nname = \"T\"\nversion = \"1\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n";
        let with = |list: &str| Manifest::parse(&format!("{content}namespaces = [{list}]\n"));
        let m = with("\"kaisyn\", \"towns_and_towers\"").unwrap();
        let owned: Vec<&str> = m.owned_namespaces().collect();
        assert_eq!(owned, ["t_and_t", "kaisyn", "towns_and_towers"]);
        for reserved in ["minecraft", "c", "forge", "horizon", "neoforge"] {
            let e = with(&format!("\"{reserved}\"")).unwrap_err();
            assert!(e.contains("not a mod's to claim"), "{e}");
        }
        assert!(with("\"t_and_t\"").unwrap_err().contains("twice"));
        assert!(with("\"a\", \"a\"").unwrap_err().contains("twice"));
        assert!(with("\"Kaisyn\"").unwrap_err().contains("is not"));
        assert!(with("\"\"").unwrap_err().contains("is not"));
        let code = format!("{EXAMPLE}namespaces = [\"kaisyn\"]\n");
        assert!(Manifest::parse(&code).unwrap_err().contains("registries"));
    }

    #[test]
    fn block_entities_need_api_0_4() {
        let base = "id = \"ironfurnaces\"\nname = \"I\"\nversion = \"1\"\nside = \"both\"\n";
        let caps = "capabilities = [\"registries\", \"block-entities\"]\n";
        let m = Manifest::parse(&format!("{base}api = \"0.4\"\n{caps}")).unwrap();
        assert_eq!(m.check_imports(["horizon:mods/machine@0.1.0"]), Ok(()));
        let old = Manifest::parse(&format!("{base}api = \"0.2\"\n{caps}")).unwrap_err();
        assert!(old.contains("needs api = \"0.4\""), "{old}");
        let without = "capabilities = [\"registries\"]\n";
        let m = Manifest::parse(&format!("{base}api = \"0.4\"\n{without}")).unwrap();
        let e = m.check_imports(["horizon:mods/machine@0.1.0"]).unwrap_err();
        assert!(e.contains("block-entities"), "{e}");
    }

    #[test]
    fn hmod_round_trips() {
        let dir = std::env::temp_dir().join(format!("horizon-mods-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("coords_hud.hmod");
        std::fs::write(&path, Package::write(EXAMPLE, b"\0asm").unwrap()).unwrap();
        let p = Package::read(&path).unwrap();
        assert_eq!(p.manifest.id, "coords_hud");
        assert_eq!(p.wasm.as_deref(), Some(&b"\0asm"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

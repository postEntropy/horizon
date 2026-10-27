//! Step 1, import: a Modrinth `.mrpack`, a CurseForge zip, an instance
//! folder (Prism / MultiMC, Packwiz, or a plain `mods/` folder), or a Crash Assistant
//! `modlist.json` (the list packs such as ATM10 publish) to `pack.json`.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    /// `mrpack`, `curseforge`, `packwiz`, `modlist` or `instance`.
    pub format: String,
    pub minecraft: String,
    /// `neoforge`, `forge`, `fabric` or `quilt`.
    pub loader: String,
    pub loader_version: String,
    pub files: Vec<PackFile>,
    /// Override files by top folder (`config`, `kubejs`, ...).
    pub overrides: BTreeMap<String, usize>,
    /// A CurseForge zip's files by project and file id only: how many its
    /// manifest lists, when the files came from the pack's own mod list.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub curseforge_files: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Where Crash Assistant keeps its list in an instance (and in the
/// overrides of packs that ship it, such as All the Mods 10).
pub const MODLIST: &str = "config/crash_assistant/modlist.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PackFile {
    /// Path in the instance (`mods/create-1.21.1-6.0.4.jar`).
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash_format: Option<String>,
    #[serde(default)]
    pub allow_external_download: bool,
    /// `required`, `optional` or `unsupported`; empty when not stated.
    pub client: String,
    pub server: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downloads: Vec<String>,
    /// CurseForge project and file ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curseforge: Option<(u64, u64)>,
    /// The mod id, name and version a mod list states (`modlist.json`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The jar on disk (instances only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar: Option<PathBuf>,
}

impl PackFile {
    /// `mods`, `resourcepacks`, `shaderpacks`, `datapacks`...
    pub fn kind(&self) -> &str {
        self.path.split('/').next().unwrap_or("")
    }
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

const LOADERS: [(&str, &str); 4] = [
    ("neoforge", "neoforge"),
    ("forge", "forge"),
    ("fabric-loader", "fabric"),
    ("quilt-loader", "quilt"),
];

pub fn import(path: &Path) -> Result<Pack, String> {
    if path.is_dir() {
        if path.join("pack.toml").is_file() {
            return packwiz(path);
        }
        return instance(path);
    }
    if path.extension().is_some_and(|e| e == "json") {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let list: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        return modlist(&list);
    }
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(index) = read_json(&mut zip, "modrinth.index.json")? {
        return Ok(mrpack(&index, &names(&zip)));
    }
    if let Some(manifest) = read_json(&mut zip, "manifest.json")? {
        let mut pack = curseforge(&manifest, &names(&zip));
        // The manifest names files only by CurseForge ids, which need a key
        // to resolve; the pack's own mod list names them and gives their
        // SHA-1s, so Modrinth can answer for most.
        let root = manifest["overrides"].as_str().unwrap_or("overrides");
        if let Some(list) = read_json(&mut zip, &format!("{root}/{MODLIST}"))? {
            pack.curseforge_files = pack.files.len();
            pack.files = modlist(&list)?.files;
        }
        return Ok(pack);
    }
    Err(format!(
        "{}: neither modrinth.index.json nor manifest.json",
        path.display()
    ))
}

/// Copies pack configuration and scripts to a staging directory for the
/// launcher profile. Content files (mods, packs and datapacks) are handled by
/// inventory/carry instead. Existing profile files are preserved by the caller.
pub fn extract_overrides(source: &Path, destination: &Path) -> Result<usize, String> {
    fs::create_dir_all(destination).map_err(|e| format!("{}: {e}", destination.display()))?;
    if source.extension().is_some_and(|ext| ext == "json") {
        return Ok(0);
    }
    if source.is_dir() {
        let root = source
            .canonicalize()
            .map_err(|e| format!("{}: {e}", source.display()))?;
        let mut count = 0;
        if root.join("pack.toml").is_file() {
            let manifest: toml::Value = toml::from_str(
                &fs::read_to_string(root.join("pack.toml")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("pack.toml: {e}"))?;
            let index_rel = manifest["index"]["file"].as_str().unwrap_or("index.toml");
            let index_path = safe_pack_path(&root, index_rel)?;
            let index: toml::Value =
                toml::from_str(&fs::read_to_string(&index_path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("index.toml: {e}"))?;
            for entry in index["files"].as_array().into_iter().flatten() {
                if entry["metafile"].as_bool() == Some(true) {
                    continue;
                }
                let Some(relative) = entry["file"].as_str() else {
                    continue;
                };
                if tracked_path(relative) || relative.ends_with(".pw.toml") {
                    continue;
                }
                let input = safe_pack_path(&root, relative)?;
                count += copy_one(&input, destination, relative)?;
            }
        } else {
            let game = [".minecraft", "minecraft", "."]
                .iter()
                .map(|p| root.join(p))
                .find(|p| p.join("mods").is_dir())
                .unwrap_or_else(|| root.clone());
            for folder in ["config", "defaultconfigs", "kubejs", "datapacks"] {
                count += copy_tree(&game.join(folder), destination, &root)?;
            }
        }
        return Ok(count);
    }

    let file = fs::File::open(source).map_err(|e| format!("{}: {e}", source.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", source.display()))?;
    let manifest = read_json(&mut zip, "manifest.json")?;
    let roots = if let Some(manifest) = manifest {
        let root = manifest["overrides"].as_str().unwrap_or("overrides");
        vec![format!("{root}/")]
    } else {
        vec!["overrides/".into(), "client-overrides/".into()]
    };
    let mut count = 0;
    // Process base overrides first, then client overrides so the latter win.
    for root in roots {
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let Some(relative) = entry.name().strip_prefix(&root).map(str::to_owned) else {
                continue;
            };
            if entry.is_dir() || relative.is_empty() {
                continue;
            }
            validate_relative(&relative)?;
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                continue;
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|e| format!("{}: {e}", entry.name()))?;
            write_override(destination, &relative, &bytes)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Turns override data packs into inventory files so they use the same
/// fetch, carry-over, and coverage path as data packs listed in a manifest.
/// Folder packs are zipped in the staging area; their ZIP is also what new
/// worlds receive from the profile.
pub fn index_override_datapacks(work: &Path, pack: &mut Pack) -> Result<usize, String> {
    let root = work.join("pack-overrides/datapacks");
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("{}: {error}", root.display())),
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut indexed = 0;
    for entry in entries {
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let archive = if metadata.is_dir() && path.join("pack.mcmeta").is_file() {
            let stem = path.file_name().unwrap_or_default().to_string_lossy();
            let mut target = root.join(format!("{stem}.override.zip"));
            let mut suffix = 2;
            while target.exists() {
                target = root.join(format!("{stem}.override-{suffix}.zip"));
                suffix += 1;
            }
            zip_datapack_folder(&path, &target)?;
            fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            target
        } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "zip") {
            path
        } else {
            continue;
        };
        let relative = archive
            .strip_prefix(&root)
            .map_err(|e| format!("{}: {e}", archive.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        let pack_path = format!("datapacks/{relative}");
        if pack.files.iter().any(|file| file.path == pack_path) {
            continue;
        }
        let bytes = fs::read(&archive).map_err(|e| format!("{}: {e}", archive.display()))?;
        let name = archive
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        pack.files.push(PackFile {
            path: pack_path,
            sha1: Some(sha1_hex(&bytes)),
            name: Some(name),
            jar: Some(archive),
            ..PackFile::default()
        });
        indexed += 1;
    }
    Ok(indexed)
}

fn zip_datapack_folder(source: &Path, destination: &Path) -> Result<(), String> {
    fn add_files(
        root: &Path,
        dir: &Path,
        zip: &mut zip::ZipWriter<fs::File>,
    ) -> Result<(), String> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
            .collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                add_files(root, &path, zip)?;
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|e| format!("{}: {e}", path.display()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                validate_relative(&relative)?;
                zip.start_file(&relative, zip::write::SimpleFileOptions::default())
                    .map_err(|e| e.to_string())?;
                let mut file =
                    fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                std::io::copy(&mut file, zip).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    let file =
        fs::File::create(destination).map_err(|e| format!("{}: {e}", destination.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    add_files(source, source, &mut zip)?;
    zip.finish()
        .map_err(|e| format!("{}: {e}", destination.display()))?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path, root: &Path) -> Result<usize, String> {
    if !source.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !canonical.starts_with(root) {
            continue;
        }
        if canonical.is_dir() {
            count += copy_tree(&canonical, destination, root)?;
        } else if canonical.is_file() {
            let rel = canonical
                .strip_prefix(root)
                .map_err(|e| format!("{}: {e}", canonical.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            count += copy_one(&canonical, destination, &rel)?;
        }
    }
    Ok(count)
}

fn copy_one(source: &Path, destination: &Path, relative: &str) -> Result<usize, String> {
    validate_relative(relative)?;
    let bytes = fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
    write_override(destination, relative, &bytes)?;
    Ok(1)
}

fn write_override(destination: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    validate_relative(relative)?;
    let target = destination.join(relative);
    let parent = target
        .parent()
        .ok_or_else(|| format!("invalid override path: {relative}"))?;
    fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))
}

/// Imports a Packwiz project directory. Index paths and `.pw.toml` filenames
/// are kept relative to the pack root, and external files retain their
/// declared URL and hash for the ordinary fetch/inventory pipeline.
fn packwiz(root: &Path) -> Result<Pack, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))?;
    let pack_path = safe_pack_path(&root, "pack.toml")?;
    let pack_text = fs::read_to_string(&pack_path).map_err(|e| format!("pack.toml: {e}"))?;
    let manifest: toml::Value =
        toml::from_str(&pack_text).map_err(|e| format!("pack.toml: {e}"))?;
    let index_rel = manifest["index"]["file"].as_str().unwrap_or("index.toml");
    let index_path = safe_pack_path(&root, index_rel)?;
    let index_bytes =
        fs::read(&index_path).map_err(|e| format!("{}: {e}", index_path.display()))?;
    let index_text = std::str::from_utf8(&index_bytes).map_err(|e| format!("index.toml: {e}"))?;
    let index: toml::Value = toml::from_str(index_text).map_err(|e| format!("index.toml: {e}"))?;
    let index_format = manifest["index"]["hash-format"]
        .as_str()
        .or_else(|| index["hash-format"].as_str())
        .unwrap_or("sha256");
    if let Some(expected) = manifest["index"]["hash"].as_str() {
        verify_hash("packwiz index", index_format, expected, &index_bytes)?;
    }
    let versions = &manifest["versions"];
    let (loader, loader_version) = ["neoforge", "forge", "fabric", "quilt"]
        .into_iter()
        .find_map(|name| {
            versions[name]
                .as_str()
                .map(|v| (name.to_owned(), v.to_owned()))
        })
        .unwrap_or_default();
    let mut pack = Pack {
        name: manifest["name"]
            .as_str()
            .unwrap_or("Packwiz pack")
            .to_owned(),
        format: "packwiz".into(),
        minecraft: versions["minecraft"].as_str().unwrap_or("").to_owned(),
        loader,
        loader_version,
        ..Pack::default()
    };
    for entry in index["files"].as_array().into_iter().flatten() {
        let indexed = entry["file"]
            .as_str()
            .ok_or("index.toml: file entry has no path")?;
        let hash_format = entry["hash-format"].as_str().unwrap_or(index_format);
        let expected_hash = entry["hash"].as_str().unwrap_or("").to_owned();
        if expected_hash.is_empty() {
            return Err(format!("index.toml: {indexed} has no hash"));
        }
        let indexed_path = safe_pack_path(&root, indexed)?;
        let metafile = entry["metafile"].as_bool() == Some(true);
        if metafile {
            let metadata_bytes =
                fs::read(&indexed_path).map_err(|e| format!("{}: {e}", indexed_path.display()))?;
            verify_hash(indexed, hash_format, &expected_hash, &metadata_bytes)?;
            let text = String::from_utf8(metadata_bytes)
                .map_err(|e| format!("{}: {e}", indexed_path.display()))?;
            let metadata: toml::Value =
                toml::from_str(&text).map_err(|e| format!("{}: {e}", indexed_path.display()))?;
            let filename = metadata["filename"]
                .as_str()
                .ok_or_else(|| format!("{}: missing filename", indexed))?;
            let parent = Path::new(indexed).parent().unwrap_or(Path::new(""));
            let destination = parent.join(filename).to_string_lossy().replace('\\', "/");
            validate_relative(&destination)?;
            let top = destination.split('/').next().unwrap_or("").to_owned();
            let url = metadata["download"]["url"]
                .as_str()
                .ok_or_else(|| format!("{}: missing download.url", indexed))?;
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(format!("{}: unsupported download URL", indexed));
            }
            let hash_format = metadata["download"]["hash-format"]
                .as_str()
                .ok_or_else(|| format!("{}: missing download.hash-format", indexed))?;
            let hash = metadata["download"]["hash"]
                .as_str()
                .ok_or_else(|| format!("{}: missing download.hash", indexed))?;
            let record = PackFile {
                path: destination,
                sha1: (hash_format == "sha1").then(|| hash.to_owned()),
                hash: Some(hash.to_owned()),
                hash_format: Some(hash_format.to_owned()),
                allow_external_download: true,
                client: match metadata["side"].as_str().unwrap_or("both") {
                    "server" => "unsupported".into(),
                    _ => "required".into(),
                },
                server: match metadata["side"].as_str().unwrap_or("both") {
                    "client" => "unsupported".into(),
                    _ => "required".into(),
                },
                downloads: vec![url.to_owned()],
                name: metadata["name"].as_str().map(str::to_owned),
                mod_id: metadata["update"]["modrinth"]["mod-id"]
                    .as_str()
                    .map(str::to_owned),
                version: metadata["update"]["modrinth"]["version"]
                    .as_str()
                    .map(str::to_owned),
                ..PackFile::default()
            };
            if tracked_path(&record.path) {
                pack.files.push(record);
            } else {
                *pack.overrides.entry(top).or_default() += 1;
            }
        } else {
            let bytes =
                fs::read(&indexed_path).map_err(|e| format!("{}: {e}", indexed_path.display()))?;
            verify_hash(indexed, hash_format, &expected_hash, &bytes)?;
            let relative = indexed.replace('\\', "/");
            if relative.ends_with(".pw.toml") {
                continue;
            }
            let top = relative.split('/').next().unwrap_or("").to_owned();
            let is_jar = relative.starts_with("mods/") && relative.ends_with(".jar");
            let record = PackFile {
                path: relative,
                sha1: (hash_format == "sha1").then(|| expected_hash.clone()),
                hash: Some(expected_hash),
                hash_format: Some(hash_format.to_owned()),
                jar: is_jar.then_some(indexed_path),
                ..PackFile::default()
            };
            if tracked_path(&record.path) {
                pack.files.push(record);
            } else {
                *pack.overrides.entry(top).or_default() += 1;
            }
        }
    }
    Ok(pack)
}

fn tracked_path(path: &str) -> bool {
    matches!(
        path.split('/').next().unwrap_or(""),
        "mods" | "resourcepacks" | "shaderpacks" | "datapacks"
    )
}

pub(crate) fn validate_relative(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("unsafe pack path: {}", path.display()));
    }
    Ok(())
}

fn safe_pack_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative(relative)?;
    let path = root.join(relative);
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !canonical.starts_with(root) {
        return Err(format!("pack path escapes its root: {relative}"));
    }
    Ok(canonical)
}

pub(crate) fn verify_hash(
    label: &str,
    format: &str,
    expected: &str,
    bytes: &[u8],
) -> Result<(), String> {
    use sha2::Digest;
    let actual = match format {
        "sha1" => sha1_hex(bytes),
        "sha256" => sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        "sha512" => sha2::Sha512::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        "md5" => md5::Md5::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        "murmur2" => murmur2(bytes).to_string(),
        other => return Err(format!("{label}: unsupported hash format {other}")),
    };
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("{label}: {format} {actual}, pack says {expected}"))
    }
}

fn murmur2(data: &[u8]) -> u32 {
    let (m, r) = (0x5bd1e995u32, 24u32);
    // Packwiz uses CurseForge's variant: seed 1 and whitespace bytes removed.
    let filtered: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .collect();
    let mut h = (filtered.len() as u32) ^ 1;
    let mut chunks = filtered.chunks_exact(4);
    for chunk in &mut chunks {
        let mut k = u32::from_le_bytes(chunk.try_into().unwrap());
        k = k.wrapping_mul(m);
        k ^= k >> r;
        k = k.wrapping_mul(m);
        h = h.wrapping_mul(m) ^ k;
    }
    let tail = chunks.remainder();
    if tail.len() >= 3 {
        h ^= (tail[2] as u32) << 16;
    }
    if tail.len() >= 2 {
        h ^= (tail[1] as u32) << 8;
    }
    if !tail.is_empty() {
        h = (h ^ tail[0] as u32).wrapping_mul(m);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(m);
    h ^ (h >> 15)
}

fn names(zip: &zip::ZipArchive<fs::File>) -> Vec<String> {
    zip.file_names().map(str::to_owned).collect()
}

fn read_json(zip: &mut zip::ZipArchive<fs::File>, name: &str) -> Result<Option<Value>, String> {
    let Ok(mut entry) = zip.by_name(name) else {
        return Ok(None);
    };
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map(Some)
        .map_err(|e| format!("{name}: {e}"))
}

fn str_of(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_owned()
}

/// Override files by top folder, below any of `roots`.
fn overrides(names: &[String], roots: &[&str]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for name in names.iter().filter(|n| !n.ends_with('/')) {
        for root in roots {
            if let Some(rest) = name.strip_prefix(root).and_then(|r| r.strip_prefix('/')) {
                let top = rest.split('/').next().unwrap_or(rest);
                *out.entry(top.to_owned()).or_default() += 1;
            }
        }
    }
    out
}

pub fn mrpack(index: &Value, names: &[String]) -> Pack {
    let deps = &index["dependencies"];
    let (loader, loader_version) = LOADERS
        .iter()
        .find_map(|(key, name)| Some((name.to_string(), deps.get(*key)?.as_str()?.to_owned())))
        .unwrap_or_default();
    let files = index["files"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            let sha1 = f["hashes"]["sha1"].as_str().map(str::to_owned);
            let (hash_format, hash) = if let Some(hash) = &sha1 {
                (Some("sha1".to_owned()), Some(hash.clone()))
            } else if let Some(hash) = f["hashes"]["sha512"].as_str() {
                (Some("sha512".to_owned()), Some(hash.to_owned()))
            } else {
                (None, None)
            };
            PackFile {
                path: str_of(f, "path"),
                sha1,
                hash,
                hash_format,
                client: str_of(&f["env"], "client"),
                server: str_of(&f["env"], "server"),
                downloads: f["downloads"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|d| d.as_str().map(str::to_owned))
                    .collect(),
                ..PackFile::default()
            }
        })
        .collect();
    Pack {
        name: str_of(index, "name"),
        format: "mrpack".into(),
        minecraft: str_of(deps, "minecraft"),
        loader,
        loader_version,
        files,
        overrides: overrides(
            names,
            &["overrides", "client-overrides", "server-overrides"],
        ),
        ..Pack::default()
    }
}

pub fn curseforge(manifest: &Value, names: &[String]) -> Pack {
    let primary = manifest["minecraft"]["modLoaders"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["primary"].as_bool().unwrap_or(true))
        .map(|l| str_of(l, "id"))
        .unwrap_or_default();
    let (loader, loader_version) = primary.split_once('-').unwrap_or((&primary, ""));
    let files = manifest["files"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            let (p, id) = (f["projectID"].as_u64(), f["fileID"].as_u64());
            PackFile {
                path: format!("mods/curseforge-{}-{}", p.unwrap_or(0), id.unwrap_or(0)),
                // CurseForge states no side; its API needs a key, so names
                // come from the user's installed instance (import that).
                curseforge: p.zip(id),
                ..PackFile::default()
            }
        })
        .collect();
    let root = manifest["overrides"].as_str().unwrap_or("overrides");
    Pack {
        name: str_of(manifest, "name"),
        format: "curseforge".into(),
        minecraft: str_of(&manifest["minecraft"], "version"),
        loader: loader.to_owned(),
        loader_version: loader_version.to_owned(),
        files,
        overrides: overrides(names, &[root]),
        ..Pack::default()
    }
}

/// Crash Assistant's `config/crash_assistant/modlist.json`: jar name → mod
/// id, name, version and Modrinth SHA-1, with the loader as an entry of its
/// own. NeoForge `21.1.x` runs Minecraft 1.21.1.
pub fn modlist(list: &Value) -> Result<Pack, String> {
    let entries = list.as_object().ok_or("modlist.json: not an object")?;
    let mut pack = Pack {
        name: "modlist".into(),
        format: "modlist".into(),
        ..Pack::default()
    };
    for (key, e) in entries {
        let jar = e["jarName"].as_str().unwrap_or(key);
        if jar.ends_with("(modloader)") {
            let version = str_of(e, "version");
            let (loader, v) = version.split_once('-').unwrap_or(("", &version));
            pack.loader = loader.to_owned();
            pack.loader_version = v.to_owned();
            if loader == "neoforge" {
                let mut parts = v.split('.');
                if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
                    pack.minecraft = if b == "0" {
                        format!("1.{a}")
                    } else {
                        format!("1.{a}.{b}")
                    };
                }
            }
            continue;
        }
        let folder = if jar.ends_with(".zip") {
            "datapacks"
        } else {
            "mods"
        };
        let text = |k: &str| e[k].as_str().filter(|v| !v.is_empty()).map(str::to_owned);
        pack.files.push(PackFile {
            path: format!("{folder}/{jar}"),
            sha1: text("modrinthHash"),
            mod_id: text("modId"),
            name: text("name"),
            version: text("version"),
            ..PackFile::default()
        });
    }
    pack.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(pack)
}

/// A Prism / MultiMC instance (`mmc-pack.json`), a game folder, or a
/// folder of jars: the jars on disk, hashed.
fn instance(dir: &Path) -> Result<Pack, String> {
    let mut pack = Pack {
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        format: "instance".into(),
        ..Pack::default()
    };
    if let Ok(text) = fs::read_to_string(dir.join("mmc-pack.json")) {
        let mmc: Value = serde_json::from_str(&text).map_err(|e| format!("mmc-pack.json: {e}"))?;
        for c in mmc["components"].as_array().into_iter().flatten() {
            let version = str_of(c, "version");
            match c["uid"].as_str().unwrap_or("") {
                "net.minecraft" => pack.minecraft = version,
                "net.neoforged" => {
                    (pack.loader, pack.loader_version) = ("neoforge".into(), version)
                }
                "net.minecraftforge" => {
                    (pack.loader, pack.loader_version) = ("forge".into(), version)
                }
                "net.fabricmc.fabric-loader" => {
                    (pack.loader, pack.loader_version) = ("fabric".into(), version)
                }
                "org.quiltmc.quilt-loader" => {
                    (pack.loader, pack.loader_version) = ("quilt".into(), version)
                }
                _ => {}
            }
        }
    }
    let game = [".minecraft", "minecraft", "."]
        .iter()
        .map(|g| dir.join(g))
        .find(|g| g.join("mods").is_dir())
        .or_else(|| dir.read_dir().is_ok().then(|| dir.to_path_buf()))
        .ok_or_else(|| format!("{}: no mods folder", dir.display()))?;
    let mods = if game.join("mods").is_dir() {
        game.join("mods")
    } else {
        game.clone()
    };
    let mut jars: Vec<PathBuf> = fs::read_dir(&mods)
        .map_err(|e| format!("{}: {e}", mods.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jar"))
        .collect();
    jars.sort();
    for jar in jars {
        let bytes = fs::read(&jar).map_err(|e| format!("{}: {e}", jar.display()))?;
        pack.files.push(PackFile {
            path: format!(
                "mods/{}",
                jar.file_name().unwrap_or_default().to_string_lossy()
            ),
            sha1: Some(sha1_hex(&bytes)),
            jar: Some(jar),
            ..PackFile::default()
        });
    }
    for top in ["config", "kubejs", "defaultconfigs", "datapacks"] {
        let n = count_files(&game.join(top));
        if n > 0 {
            pack.overrides.insert(top.into(), n);
        }
    }
    Ok(pack)
}

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() { count_files(&p) } else { 1 }
        })
        .sum()
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::Digest;
    sha1::Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_an_mrpack_index() {
        let index = json!({"name": "Small", "dependencies": {"minecraft": "1.21.1", "neoforge": "21.1.233"},
            "files": [{"path": "mods/create.jar", "hashes": {"sha1": "ab"}, "env": {"client": "required", "server": "required"},
                "downloads": ["https://cdn.modrinth.com/create.jar"]},
                {"path": "resourcepacks/look.zip", "hashes": {"sha1": "cd"}}]});
        let names = [
            "overrides/config/a.toml",
            "overrides/config/b/c.toml",
            "client-overrides/kubejs/x.js",
            "overrides/",
        ]
        .map(String::from);
        let p = mrpack(&index, &names);
        assert_eq!(
            (
                p.minecraft.as_str(),
                p.loader.as_str(),
                p.loader_version.as_str()
            ),
            ("1.21.1", "neoforge", "21.1.233")
        );
        assert_eq!(p.files.len(), 2);
        assert_eq!(
            (p.files[0].kind(), p.files[0].file_name()),
            ("mods", "create.jar")
        );
        assert_eq!(p.files[0].sha1.as_deref(), Some("ab"));
        assert_eq!(p.files[1].kind(), "resourcepacks");
        assert_eq!(
            p.overrides,
            BTreeMap::from([("config".into(), 2), ("kubejs".into(), 1)])
        );
    }

    #[test]
    fn reads_a_crash_assistant_mod_list() {
        let list = serde_json::json!({
            "neoforge-21.1.259 (modloader)": {"jarName": "neoforge-21.1.259 (modloader)",
                "modId": "neoforge", "version": "neoforge-21.1.259"},
            "mcw-doors-1.1.2.jar": {"jarName": "mcw-doors-1.1.2.jar", "modId": "mcwdoors",
                "modrinthHash": "0123456789abcdef0123456789abcdef01234567"},
            "sawmill.zip": {"jarName": "sawmill.zip"}
        });
        let pack = modlist(&list).unwrap();
        assert_eq!(
            (
                pack.minecraft.as_str(),
                pack.loader.as_str(),
                pack.loader_version.as_str()
            ),
            ("1.21.1", "neoforge", "21.1.259")
        );
        let paths: Vec<&str> = pack.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["datapacks/sawmill.zip", "mods/mcw-doors-1.1.2.jar"]);
        assert!(pack.files[1].sha1.is_some());
        assert_eq!(pack.files[1].mod_id.as_deref(), Some("mcwdoors"));
    }

    #[test]
    fn reads_a_curseforge_manifest() {
        let manifest = json!({"name": "CF", "overrides": "overrides",
            "minecraft": {"version": "1.21.1", "modLoaders": [{"id": "neoforge-21.1.259", "primary": true}]},
            "files": [{"projectID": 238222, "fileID": 5101366, "required": true}]});
        let p = curseforge(&manifest, &["overrides/kubejs/server_scripts/a.js".into()]);
        assert_eq!(
            (p.loader.as_str(), p.loader_version.as_str()),
            ("neoforge", "21.1.259")
        );
        assert_eq!(p.files[0].curseforge, Some((238222, 5101366)));
        assert_eq!(p.overrides["kubejs"], 1);
    }

    #[test]
    fn a_curseforge_zip_takes_names_from_its_mod_list() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("horizon-cf-zip-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pack.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        let manifest = json!({"name": "Pack", "overrides": "overrides",
            "minecraft": {"version": "1.21.1", "modLoaders": [{"id": "neoforge-21.1.259"}]},
            "files": [{"projectID": 1, "fileID": 2}, {"projectID": 3, "fileID": 4}]});
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file(format!("overrides/{MODLIST}"), opts)
            .unwrap();
        let list = json!({"mcw-doors-1.1.2.jar": {"jarName": "mcw-doors-1.1.2.jar",
            "modId": "mcwdoors", "modrinthHash": "0123456789abcdef0123456789abcdef01234567"}});
        // Crash Assistant writes a byte order mark.
        zip.write_all(format!("\u{feff}{list}").as_bytes()).unwrap();
        zip.finish().unwrap();
        let pack = import(&path).unwrap();
        assert_eq!(
            (pack.format.as_str(), pack.name.as_str()),
            ("curseforge", "Pack")
        );
        assert_eq!(pack.curseforge_files, 2);
        assert_eq!(pack.files.len(), 1);
        assert_eq!(pack.files[0].path, "mods/mcw-doors-1.1.2.jar");
        assert_eq!(pack.overrides["config"], 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}

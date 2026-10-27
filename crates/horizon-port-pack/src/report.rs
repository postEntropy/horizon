//! Step 7, the coverage report: per mod, ported, partial, not yet or not
//! needed, with the reason, and the totals by class, tier and licence
//! (`coverage.json`, `coverage.md`). It reads what the other steps wrote:
//! the inventory, each mod's `carry-report.json` and its harness result
//! (`tools/mod_port_harness.py`).
use crate::inventory::Record;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Ported,
    Partial,
    NotYet,
    NotNeeded,
}
impl Status {
    pub const ALL: [Status; 4] = [
        Status::Ported,
        Status::Partial,
        Status::NotYet,
        Status::NotNeeded,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Status::Ported => "ported",
            Status::Partial => "partial",
            Status::NotYet => "not yet",
            Status::NotNeeded => "not needed",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Row {
    pub key: String,
    pub name: String,
    pub class: String,
    pub tier: String,
    pub license: String,
    /// `permissive`, `copyleft`, `all-rights-reserved`, `restricted`
    /// (non-commercial or no-derivatives), `custom` or `unknown`.
    pub license_kind: String,
    pub status: Status,
    pub reason: String,
}

/// The licence family of an SPDX id or a `mods.toml` licence text.
pub fn license_kind(license: &str) -> &'static str {
    let l = license.to_ascii_lowercase();
    if l.is_empty() || l.starts_with("unknown") {
        "unknown"
    } else if l.contains("-nc") || l.contains("-nd") || l.contains("noncommercial") {
        "restricted"
    } else if l.contains("all-rights-reserved") || l.contains("all rights reserved") || l == "arr" {
        "all-rights-reserved"
    } else if ["gpl", "mpl", "eupl", "general public"]
        .iter()
        .any(|k| l.contains(k))
    {
        "copyleft"
    } else if [
        "mit",
        "apache",
        "bsd",
        "cc0",
        "cc-by",
        "unlicense",
        "isc",
        "zlib",
    ]
    .iter()
    .any(|k| l.contains(k))
    {
        "permissive"
    } else {
        "custom"
    }
}

/// What still blocks a mod of this tier that has not been carried over.
fn not_yet(tier: &str) -> &'static str {
    match tier {
        "data" => "data tier: carry-over only, no agent; its jar is not here yet (fetch gets it)",
        "logic" => {
            "mod API 0.3 ready: behaviour callbacks (use, place, break, ticks) and server events; jar not carried yet"
        }
        "machines" => {
            "mod API 0.4 ready: block entities, menus, capabilities and mod networking; jar not carried yet"
        }
        "deep" => {
            "mod API 0.5 ready: entities, custom renderers and models, dimensions, named vanilla hooks; jar not carried yet"
        }
        "client" => {
            "client interfaces ready (keys, target, inventory-read, ui, render); not carried yet"
        }
        _ => "not classified",
    }
}

fn not_needed(class: &str) -> &'static str {
    match class {
        "library" => "library: its API maps to Horizon's mod API and SDK",
        "performance" => "performance mod: Horizon's engine is the replacement",
        "other" => "pack tooling or an engine feature, folded into the pipeline or the engine",
        "client-ui" => "built into Horizon",
        _ => "nothing to port",
    }
}

/// Pack files are installed directly in the profile, not carried as mod
/// manifests. Report their actual runtime support instead of treating their
/// archive as a missing data-tier mod jar.
fn pack_file_status(record: &Record, work: &Path) -> Option<(Status, String)> {
    let kind = record.path.split('/').next()?;
    if !matches!(kind, "resourcepacks" | "shaderpacks" | "datapacks") {
        return None;
    }
    let installed = crate::import::validate_relative(&record.path).is_ok()
        && ["pack-overrides", "home"]
            .iter()
            .any(|root| work.join(root).join(&record.path).is_file());
    let reason = match (kind, installed) {
        ("resourcepacks", true) => "resource pack installed and enabled in the profile",
        ("resourcepacks", false) => "resource pack not fetched or installed yet",
        ("shaderpacks", true) => {
            "shader pack installed; Horizon does not apply shader packs in the renderer yet"
        }
        ("shaderpacks", false) => "shader pack not fetched or installed yet",
        ("datapacks", true) => {
            "data pack installed for new worlds; functions, tags and predicates load, but its other data is not applied yet"
        }
        ("datapacks", false) => "data pack not fetched or installed yet",
        _ => unreachable!(),
    };
    let status = match (kind, installed) {
        ("resourcepacks", true) => Status::Ported,
        ("datapacks" | "shaderpacks", true) => Status::Partial,
        (_, false) => Status::NotYet,
        _ => unreachable!(),
    };
    Some((status, reason.to_owned()))
}

/// A carried-over mod: ported when its content is complete (data tier with no
/// missing blocks/processors, or code tier with `carry/mod.wasm`) and no
/// harness check failed; else partial, with what is missing.
fn carried(work: &Path, key: &str, tier: &str) -> Option<(Status, String)> {
    let dir = work.join("mods").join(key);
    let has_wasm = dir.join("carry/mod.wasm").is_file();
    let harness: Option<Value> = std::fs::read(dir.join("harness/result.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let mut harness_gaps = Vec::new();
    let mut checked = 0;
    if let Some(h) = &harness {
        for (name, c) in h["checks"].as_object().into_iter().flatten() {
            let (p, f) = (
                c["passed"].as_u64().unwrap_or(0),
                c["failed"].as_u64().unwrap_or(0),
            );
            checked += p + f;
            if f > 0 {
                harness_gaps.push(format!("{name} {p}/{}", p + f));
            }
        }
    }
    let carry: Value = match std::fs::read(dir.join("carry-report.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
    {
        Some(c) => c,
        None if has_wasm && dir.join("carry/horizon-mod.toml").is_file() => {
            return if harness_gaps.is_empty() {
                let suffix = if harness.is_some() {
                    "; harness green"
                } else {
                    ""
                };
                Some((Status::Ported, format!("ported-wasm (mod.wasm{suffix})")))
            } else {
                Some((
                    Status::Partial,
                    format!("mod.wasm; {}", harness_gaps.join("; ")),
                ))
            };
        }
        None => return None,
    };
    let count = |k: &str| carry[k].as_object().map_or(0, |o| o.len());
    let (blocks, items) = (count("blocks"), carry["items"].as_u64().unwrap_or(0));
    let (left_blocks, left_items) = (count("blocks_unknown"), count("items_unknown"));
    let worldgen = carry["worldgen"].as_u64().unwrap_or(0);
    let sum_map = |k: &str| -> u64 {
        carry["worldgen_types"][k]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_u64())
            .sum()
    };
    let wg_unsup = sum_map("unsupported");
    let wg_dropped = sum_map("processors_dropped");
    let wg_lost = sum_map("rules_lost");

    if has_wasm {
        let mut summary = if blocks == 0 && items == 0 && worldgen > 0 {
            format!("ported-wasm ({worldgen} worldgen files, mod.wasm)")
        } else {
            format!("ported-wasm ({blocks} blocks, {items} items, mod.wasm)")
        };
        if harness_gaps.is_empty() {
            if harness.is_some() {
                summary.push_str("; harness green");
            }
            return Some((Status::Ported, summary));
        }
        return Some((
            Status::Partial,
            format!("{summary}; {}", harness_gaps.join("; ")),
        ));
    }

    // A class of its own is code the facts cannot show; anything else the
    // template misses (shapes, sounds) the harness also reports.
    let mut differences = 0;
    let mut own_code = 0;
    for list in carry["differences"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, v)| v)
    {
        let notes: Vec<&str> = list
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if notes.iter().any(|n| n.starts_with("class ")) {
            own_code += 1;
        }
        if notes.iter().any(|n| !n.starts_with("class ")) {
            differences += 1;
        }
    }
    let mut gaps = Vec::new();
    if left_blocks > 0 {
        gaps.push(format!(
            "{left_blocks} blocks left out (no vanilla template)"
        ));
    }
    if left_items > 0 {
        gaps.push(format!("{left_items} items left out"));
    }
    if differences > 0 {
        gaps.push(format!(
            "{differences} blocks or items differ from their templates"
        ));
    }
    if wg_unsup > 0 {
        gaps.push(format!("{wg_unsup} unsupported worldgen structures"));
    }
    if wg_dropped > 0 {
        gaps.push(format!("{wg_dropped} worldgen processors dropped"));
    }
    if wg_lost > 0 {
        gaps.push(format!("{wg_lost} worldgen rules lost"));
    }
    gaps.extend(harness_gaps);
    if tier != "data" {
        gaps.push("needs mod.wasm".into());
    }

    if blocks == 0 && items == 0 && checked == 0 {
        if worldgen > 0 {
            if gaps.is_empty() {
                return Some((Status::Ported, format!("{worldgen} worldgen files carried")));
            }
            return Some((
                Status::Partial,
                format!("{worldgen} worldgen files carried; {}", gaps.join("; ")),
            ));
        }
        let what = if tier != "data" {
            "assets/data carried; needs mod.wasm".into()
        } else {
            "nothing registered to compare".into()
        };
        return Some((Status::Partial, what));
    }
    let mut summary = format!("{blocks} blocks, {items} items");
    if own_code > 0 {
        let _ = write!(
            summary,
            " ({own_code} with code of their own, not compared)"
        );
    }
    if gaps.is_empty() {
        let suffix = if harness.is_some() {
            "harness green"
        } else {
            "data carried"
        };
        Some((Status::Ported, format!("{summary}; {suffix}")))
    } else {
        Some((Status::Partial, format!("{summary}; {}", gaps.join("; "))))
    }
}

pub fn build(records: &[Record], work: &Path) -> Vec<Row> {
    let mut rows: Vec<Row> = records
        .iter()
        .map(|r| {
            let (status, reason) = if let Some(status) = pack_file_status(r, work) {
                status
            } else if r.class == "unknown" {
                let why = if r.jar.is_some() {
                    "unclassified after reading its jar metadata and file census; needs review"
                } else {
                    "unclassified: the port step will inspect its jar metadata and file census"
                };
                (Status::NotYet, why.to_owned())
            } else if r.tier == "none" || matches!(r.class.as_str(), "library" | "performance") {
                (Status::NotNeeded, not_needed(&r.class).to_owned())
            } else if let Some(c) = carried(work, &r.key, &r.tier) {
                c
            } else {
                let mut why = not_yet(&r.tier).to_owned();
                if r.tier == "data" && r.jar.is_some() {
                    why = "data tier: carry-over not run yet".into();
                }
                if r.class == "integration" {
                    why = format!("after the mods it joins are ported; {why}");
                }
                (Status::NotYet, why)
            };
            Row {
                key: r.key.clone(),
                name: r.name.clone(),
                class: r.class.clone(),
                tier: r.tier.clone(),
                license: r.license.clone(),
                license_kind: license_kind(&r.license).to_owned(),
                status,
                reason,
            }
        })
        .collect();
    // A mod waits on its dependencies that are not ported.
    let status: BTreeMap<String, Status> = rows.iter().map(|r| (r.key.clone(), r.status)).collect();
    for (row, r) in rows.iter_mut().zip(records) {
        if row.status == Status::NotNeeded {
            continue;
        }
        let waiting: Vec<&str> = r
            .depends
            .iter()
            .filter(|d| status.get(*d) == Some(&Status::NotYet))
            .map(String::as_str)
            .collect();
        if !waiting.is_empty() {
            let _ = write!(row.reason, "; waits on {}", waiting.join(", "));
        }
    }
    rows
}

/// Rows counted by `field` and status.
fn table(rows: &[Row], title: &str, field: fn(&Row) -> &str) -> String {
    let mut counts: BTreeMap<&str, [usize; 4]> = BTreeMap::new();
    for r in rows {
        let i = Status::ALL.iter().position(|s| *s == r.status).unwrap_or(0);
        counts.entry(field(r)).or_default()[i] += 1;
    }
    let mut out = format!(
        "| {title} | ported | partial | not yet | not needed | all |\n|---|---:|---:|---:|---:|---:|\n"
    );
    let mut totals = [0usize; 4];
    for (k, c) in &counts {
        let all: usize = c.iter().sum();
        let _ = writeln!(
            out,
            "| {k} | {} | {} | {} | {} | {all} |",
            c[0], c[1], c[2], c[3]
        );
        for (t, n) in totals.iter_mut().zip(c) {
            *t += n;
        }
    }
    let all: usize = totals.iter().sum();
    let _ = writeln!(
        out,
        "| **all** | **{}** | **{}** | **{}** | **{}** | **{all}** |",
        totals[0], totals[1], totals[2], totals[3]
    );
    out
}

pub fn markdown(pack: &str, rows: &[Row]) -> String {
    let mut out = format!("# Coverage: {pack}\n\n");
    out += &table(rows, "class", |r| &r.class);
    out += "\n";
    out += &table(rows, "tier", |r| &r.tier);
    out += "\n";
    out += &table(rows, "licence", |r| &r.license_kind);
    out += "\nPorts stay local. Only permissive or copyleft ports could ever be published, and only after Ludovic asks the authors.\n\n";
    out += "| mod | class | tier | licence | status | why |\n|---|---|---|---|---|---|\n";
    let mut sorted: Vec<&Row> = rows.iter().collect();
    sorted.sort_by(|a, b| (a.status, &a.key).cmp(&(b.status, &b.key)));
    for r in sorted {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            r.key,
            r.class,
            r.tier,
            r.license.replace('|', "/"),
            r.status.label(),
            r.reason.replace('|', "/")
        );
    }
    out
}

pub fn counts(rows: &[Row]) -> Vec<(&'static str, usize)> {
    Status::ALL
        .iter()
        .map(|s| (s.label(), rows.iter().filter(|r| r.status == *s).count()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(key: &str, class: &str, tier: &str, depends: &[&str]) -> Record {
        Record {
            key: key.into(),
            class: class.into(),
            tier: tier.into(),
            license: "MIT".into(),
            depends: depends.iter().map(|d| (*d).to_owned()).collect(),
            ..Record::default()
        }
    }

    #[test]
    fn statuses_and_reasons() {
        let work = std::env::temp_dir().join(format!("horizon-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        let write = |key: &str, carry: Value, harness: Option<Value>| {
            let dir = work.join("mods").join(key);
            std::fs::create_dir_all(dir.join("harness")).unwrap();
            std::fs::write(dir.join("carry-report.json"), carry.to_string()).unwrap();
            if let Some(h) = harness {
                std::fs::write(dir.join("harness/result.json"), h.to_string()).unwrap();
            }
        };
        let green = serde_json::json!({"checks": {"blocks": {"passed": 3, "failed": 0}}});
        write(
            "doors",
            serde_json::json!({"blocks": {"a": "minecraft:oak_door"}, "items": 0,
                "differences": {"a": ["class m.Door (behaviour beyond DoorBlock)"]}}),
            Some(green),
        );
        write(
            "paths",
            serde_json::json!({"blocks": {"a": "minecraft:stone"}, "items": 1,
                "blocks_unknown": {"b": "no vanilla Block"}}),
            Some(serde_json::json!({"checks": {"shapes": {"passed": 1, "failed": 2}}})),
        );
        write(
            "clean_data",
            serde_json::json!({"blocks": {"a": "minecraft:stone"}, "items": 2}),
            None,
        );
        write(
            "pending_logic",
            serde_json::json!({"blocks": {"a": "minecraft:stone"}, "items": 0}),
            None,
        );
        write(
            "done_logic",
            serde_json::json!({"blocks": {"a": "minecraft:stone"}, "items": 1,
                "blocks_unknown": {"b": "custom block class"}}),
            None,
        );
        std::fs::create_dir_all(work.join("mods/done_logic/carry")).unwrap();
        std::fs::write(work.join("mods/done_logic/carry/mod.wasm"), b"\0asm").unwrap();
        let records = [
            record("doors", "content", "data", &[]),
            record("paths", "content", "data", &[]),
            record("clean_data", "content", "data", &[]),
            record("pending_logic", "content", "logic", &[]),
            record("done_logic", "content", "logic", &[]),
            record("lib", "library", "none", &[]),
            record("machine", "content", "machines", &["lib"]),
            record("addon", "integration", "logic", &["machine"]),
            record("mystery", "unknown", "none", &[]),
        ];
        let rows = build(&records, &work);
        let by: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.key.as_str(), r)).collect();
        assert_eq!(by["doors"].status, Status::Ported);
        assert!(
            by["doors"].reason.contains("1 with code of their own"),
            "{}",
            by["doors"].reason
        );
        assert_eq!(by["paths"].status, Status::Partial);
        assert!(
            by["paths"].reason.contains("1 blocks left out"),
            "{}",
            by["paths"].reason
        );
        assert!(
            by["paths"].reason.contains("shapes 1/3"),
            "{}",
            by["paths"].reason
        );
        assert_eq!(by["clean_data"].status, Status::Ported);
        assert!(
            by["clean_data"].reason.contains("data carried"),
            "{}",
            by["clean_data"].reason
        );
        assert_eq!(by["pending_logic"].status, Status::Partial);
        assert!(
            by["pending_logic"].reason.contains("needs mod.wasm"),
            "{}",
            by["pending_logic"].reason
        );
        assert_eq!(by["done_logic"].status, Status::Ported);
        assert!(
            by["done_logic"].reason.starts_with("ported-wasm"),
            "{}",
            by["done_logic"].reason
        );
        assert_eq!(by["lib"].status, Status::NotNeeded);
        assert_eq!(by["machine"].status, Status::NotYet);
        assert!(by["machine"].reason.contains("0.4"));
        assert!(by["addon"].reason.starts_with("after the mods it joins"));
        assert!(
            by["addon"].reason.ends_with("waits on machine"),
            "{}",
            by["addon"].reason
        );
        assert_eq!(by["mystery"].status, Status::NotYet);
        assert_eq!(
            counts(&rows),
            [
                ("ported", 3),
                ("partial", 2),
                ("not yet", 3),
                ("not needed", 1)
            ]
        );
        let md = markdown("test", &rows);
        assert!(
            md.contains("| **all** | **3** | **2** | **3** | **1** | **9** |"),
            "{md}"
        );
        std::fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn licence_families() {
        assert_eq!(license_kind("MIT"), "permissive");
        assert_eq!(license_kind("CC0-1.0"), "permissive");
        assert_eq!(license_kind("LGPL-3.0-only"), "copyleft");
        assert_eq!(
            license_kind("GNU Lesser General Public License v3.0"),
            "copyleft"
        );
        assert_eq!(
            license_kind("LicenseRef-All-Rights-Reserved"),
            "all-rights-reserved"
        );
        assert_eq!(license_kind("CC-BY-NC-SA-4.0"), "restricted");
        assert_eq!(license_kind("LicenseRef-Stardust-Labs-License"), "custom");
        assert_eq!(
            license_kind("unknown (read mods.toml at import)"),
            "unknown"
        );
    }
}

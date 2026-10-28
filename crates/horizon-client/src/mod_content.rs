//! Content mods in the resource root (docs/modpacks.md, registries 0.2).
//!
//! Every loader reads one resource root, and most of its block and item
//! tables are exports from vanilla indexed by block-state id or keyed by
//! name. A content mod's block copies a vanilla *template*. So the mods'
//! root is an overlay, like the resource-pack one:
//! - the base root linked, with the per-state and per-name tables
//!   rewritten: each mod block gets its template's rows appended (states
//!   after vanilla's, mods and blocks in id order) with names, translation
//!   keys and in-table state ids moved over, and the declared overrides
//!   applied;
//! - each mod's own `assets/` and `data/` files;
//! - additions to tags merged into the tag files and bundles.
//!
//! The loaders need no mod code, vanilla state ids never move, and with no
//! content mod the root is the base root itself.
use crate::{Result, blocks::MAP_COLORS};
use horizon_mods::content::{Block, Content, Rarity};
use horizon_world::nbt::{self, JavaString, Limits, Tag};
use serde::{Deserialize, Serialize, de};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt, fs,
    io::{BufWriter, Write},
    ops::Range,
    path::{Path, PathBuf},
};

/// A content mod to put in the root: its id and content.
pub struct ContentMod {
    pub id: String,
    pub content: Content,
}

/// JSON with its object keys in file order, so a rewritten table differs
/// from the original only by what was added.
#[derive(Clone, Debug, PartialEq)]
enum J {
    Null,
    Bool(bool),
    Num(serde_json::Number),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}
impl<'de> Deserialize<'de> for J {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = J;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_unit<E>(self) -> std::result::Result<J, E> {
                Ok(J::Null)
            }
            fn visit_bool<E>(self, v: bool) -> std::result::Result<J, E> {
                Ok(J::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> std::result::Result<J, E> {
                Ok(J::Num(v.into()))
            }
            fn visit_u64<E>(self, v: u64) -> std::result::Result<J, E> {
                Ok(J::Num(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<J, E> {
                serde_json::Number::from_f64(v)
                    .map(J::Num)
                    .ok_or_else(|| E::custom("not a JSON number"))
            }
            fn visit_str<E>(self, v: &str) -> std::result::Result<J, E> {
                Ok(J::Str(v.to_owned()))
            }
            fn visit_string<E>(self, v: String) -> std::result::Result<J, E> {
                Ok(J::Str(v))
            }
            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<J, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element()? {
                    out.push(v);
                }
                Ok(J::Arr(out))
            }
            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<J, A::Error> {
                let mut out = Vec::new();
                while let Some(kv) = a.next_entry()? {
                    out.push(kv);
                }
                Ok(J::Obj(out))
            }
        }
        d.deserialize_any(V)
    }
}
impl Serialize for J {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            J::Null => s.serialize_unit(),
            J::Bool(b) => s.serialize_bool(*b),
            J::Num(n) => n.serialize(s),
            J::Str(t) => s.serialize_str(t),
            J::Arr(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for v in a {
                    seq.serialize_element(v)?;
                }
                seq.end()
            }
            J::Obj(o) => {
                let mut map = s.serialize_map(Some(o.len()))?;
                for (k, v) in o {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}
impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn get_mut(&mut self, key: &str) -> Option<&mut J> {
        match self {
            J::Obj(o) => o.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// Sets `key`, in place if present, else at the end.
    fn set(&mut self, key: &str, value: J) {
        if let Some(v) = self.get_mut(key) {
            *v = value;
        } else if let J::Obj(o) = self {
            o.push((key.to_owned(), value));
        }
    }
    /// The value at a `/`-separated path ("" is the document).
    fn at(&mut self, path: &str) -> Option<&mut J> {
        path.split('/')
            .filter(|p| !p.is_empty())
            .try_fold(self, |j, key| j.get_mut(key))
    }
    fn f64(v: f64) -> J {
        serde_json::Number::from_f64(v).map_or(J::Null, J::Num)
    }
    fn as_u32(&self) -> Option<u32> {
        match self {
            J::Num(n) => n.as_u64().and_then(|v| u32::try_from(v).ok()),
            _ => None,
        }
    }
    /// Renames strings and, with `ids`, moves state ids, everywhere inside.
    fn rename(&mut self, r: &Rename, ids: bool) {
        match self {
            J::Str(s) => {
                if let Some(new) = r.string(s) {
                    *s = new;
                }
            }
            J::Num(n) if ids => {
                if let Some(new) = n
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .and_then(|id| r.id(id))
                {
                    *n = new.into();
                }
            }
            J::Arr(a) => a.iter_mut().for_each(|v| v.rename(r, ids)),
            J::Obj(o) => o.iter_mut().for_each(|(_, v)| v.rename(r, ids)),
            _ => {}
        }
    }
}

/// One id moving to another: `minecraft:iron_block` → `ruby:ruby_block`,
/// with its state range for blocks.
#[derive(Clone)]
struct Rename {
    from: String,
    to: String,
    states: Range<u32>,
    first: u32,
}
impl Rename {
    fn new(from: &str, to: &str) -> Self {
        Rename {
            from: from.to_owned(),
            to: to.to_owned(),
            states: 0..0,
            first: 0,
        }
    }
    /// The id itself, `Block{id}`, and `block.<ns>.<path>` / `item.<ns>.<path>` keys.
    fn string(&self, s: &str) -> Option<String> {
        if s == self.from {
            return Some(self.to.clone());
        }
        if s.strip_prefix("Block{").and_then(|x| x.strip_suffix('}')) == Some(&self.from) {
            return Some(format!("Block{{{}}}", self.to));
        }
        let key = |id: &str| id.replacen(':', ".", 1).replace('/', ".");
        for kind in ["block", "item"] {
            if s.strip_prefix(kind).and_then(|x| x.strip_prefix('.')) == Some(&key(&self.from)) {
                return Some(format!("{kind}.{}", key(&self.to)));
            }
        }
        None
    }
    fn id(&self, id: u32) -> Option<u32> {
        self.states
            .contains(&id)
            .then(|| self.first + (id - self.states.start))
    }
}

/// A block being added: its declaration and where its states go.
struct NewBlock<'a> {
    decl: &'a Block,
    rename: Rename,
    /// The template's item, if it has one.
    item: Option<String>,
    /// Custom state property combinations, in registry order. Each one
    /// inherits the template default state's gameplay and shape data.
    custom_states: Vec<BTreeMap<String, String>>,
}
/// An item being added (declared, or a block's).
struct NewItem {
    rename: Rename,
    max_stack_size: Option<u8>,
    rarity: Option<Rarity>,
}

struct Plan<'a> {
    blocks: Vec<NewBlock<'a>>,
    items: Vec<NewItem>,
    /// NeoForge's static fuel map, applied after new item templates are copied.
    furnace_fuels: BTreeMap<String, Option<u32>>,
    replace_furnace_fuels: bool,
    warnings: Vec<String>,
    /// Vanilla's state count (the tables' length before).
    states: usize,
    /// Each vanilla block's default state.
    defaults: BTreeMap<String, u32>,
    /// The gameplay rows' sound objects, by block, for `sounds` overrides.
    sounds: BTreeMap<String, J>,
}

fn custom_state_combinations(
    axes: &BTreeMap<String, Vec<String>>,
) -> Vec<BTreeMap<String, String>> {
    if axes.is_empty() {
        return Vec::new();
    }
    let mut combinations = vec![BTreeMap::new()];
    for (key, values) in axes {
        let mut next = Vec::new();
        for combination in combinations {
            for value in values {
                let mut combination = combination.clone();
                combination.insert(key.clone(), value.clone());
                next.push(combination);
            }
        }
        combinations = next;
    }
    combinations
}

fn block_state_count(block: &NewBlock<'_>) -> usize {
    if block.custom_states.is_empty() {
        block.rename.states.len()
    } else {
        block.custom_states.len()
    }
}

fn state_ranges<'a>(names: &[&'a str], block_count: usize) -> Result<HashMap<&'a str, Range<u32>>> {
    let mut ranges = HashMap::with_capacity(block_count);
    let mut start = 0;
    while start < names.len() {
        let name = names[start];
        let mut end = start + 1;
        while end < names.len() && names[end] == name {
            end += 1;
        }
        if ranges.insert(name, start as u32..end as u32).is_some() {
            return Err(format!("facts.json: states for {name} are not contiguous").into());
        }
        start = end;
    }
    Ok(ranges)
}

fn read_json(path: &Path) -> Result<J> {
    Ok(serde_json::from_slice(&fs::read(path)?).map_err(|e| format!("{}: {e}", path.display()))?)
}

impl<'a> Plan<'a> {
    fn new(base: &Path, mods: &'a [ContentMod]) -> Result<Self> {
        let facts = read_json(&base.join("facts.json"))?;
        let names: Vec<&str> = match facts.get("states") {
            Some(J::Arr(states)) => states
                .iter()
                .map(|s| match s {
                    J::Arr(row) => match row.first() {
                        Some(J::Str(name)) => Ok(name.as_str()),
                        _ => Err("facts.json: a state without a name"),
                    },
                    _ => Err("facts.json: a state that is not a row"),
                })
                .collect::<std::result::Result<_, _>>()?,
            _ => return Err("facts.json: no states".into()),
        };
        let defaults: BTreeMap<String, u32> = match facts.get("defaults") {
            Some(J::Obj(o)) => o
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_u32()?)))
                .collect(),
            _ => return Err("facts.json: no defaults".into()),
        };
        // A block's states are one run in registry order. Index those runs
        // once: searching the suffix for every declaration made planning
        // quadratic for large carried registries (hundreds of thousands of
        // states and thousands of mod blocks).
        let ranges = state_ranges(&names, defaults.len())?;
        let range = |name: &str| ranges.get(name).cloned();
        let mut blocks = Vec::new();
        let mut next = names.len() as u32;
        let mut sorted: Vec<&ContentMod> = mods.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let mut wanted_sounds = Vec::new();
        for m in &sorted {
            let mut declared: Vec<&Block> = m.content.blocks.iter().collect();
            declared.sort_by(|a, b| a.id.cmp(&b.id));
            for decl in declared {
                if defaults.contains_key(&decl.id) {
                    return Err(format!("{}: {} is already a block", m.id, decl.id).into());
                }
                let mut states = range(&decl.template).ok_or_else(|| {
                    format!("{}: template {} is not a block", decl.id, decl.template)
                })?;
                let custom_states = custom_state_combinations(&decl.state_properties);
                if !custom_states.is_empty() {
                    states = defaults[&decl.template]..defaults[&decl.template] + 1;
                }
                let mut rename = Rename::new(&decl.template, &decl.id);
                rename.first = next;
                let count = if custom_states.is_empty() {
                    states.len()
                } else {
                    custom_states.len()
                };
                next += count as u32;
                rename.states = states;
                if let Some(s) = &decl.properties.sounds {
                    if !defaults.contains_key(s) {
                        return Err(format!("{}: sounds {s} is not a block", decl.id).into());
                    }
                    wanted_sounds.push(s.clone());
                }
                blocks.push(NewBlock {
                    decl,
                    rename,
                    item: None,
                    custom_states,
                });
            }
        }
        if next > MAX_STATES {
            let mut by_mod: BTreeMap<&str, u32> = BTreeMap::new();
            for b in &blocks {
                *by_mod
                    .entry(&b.decl.id[..b.decl.id.find(':').unwrap_or(0)])
                    .or_default() += block_state_count(b) as u32;
            }
            let list: Vec<String> = by_mod.iter().map(|(m, n)| format!("{m} {n}")).collect();
            return Err(format!(
                "{next} block states, more than the {MAX_STATES} a world can hold ({} in the game; \
                 the mods: {})",
                names.len(),
                list.join(", ")
            )
            .into());
        }
        // Block items copy the template block's item.
        let gameplay = read_json(&base.join("gameplay-facts.json"))?;
        let rows = match gameplay.get("states") {
            Some(J::Arr(rows)) if rows.len() == names.len() => rows,
            _ => return Err("gameplay-facts.json does not match facts.json".into()),
        };
        let mut sounds = BTreeMap::new();
        for s in wanted_sounds {
            let row = &rows[defaults[&s] as usize];
            sounds.insert(s, row.get("sound").cloned().unwrap_or(J::Null));
        }
        let mut items = Vec::new();
        for b in &mut blocks {
            let row = &rows[b.rename.states.start as usize];
            b.item = match row.get("item") {
                Some(J::Str(i)) if i != "minecraft:air" => Some(i.clone()),
                _ => None,
            };
            if b.decl.item {
                let item = b.item.as_deref().ok_or_else(|| {
                    format!("{}: template {} has no item", b.decl.id, b.decl.template)
                })?;
                items.push(NewItem {
                    rename: Rename::new(item, &b.decl.id),
                    max_stack_size: None,
                    rarity: None,
                });
            }
        }
        for m in &sorted {
            for i in &m.content.items {
                items.push(NewItem {
                    rename: Rename::new(&i.template, &i.id),
                    max_stack_size: i.max_stack_size,
                    rarity: i.rarity,
                });
            }
        }
        let mut furnace_fuels = BTreeMap::new();
        let mut replace_furnace_fuels = false;
        let mut warnings = Vec::new();
        const FURNACE_FUELS: &str = "data/neoforge/data_maps/item/furnace_fuels.json";
        for m in mods {
            if !m.content.files.iter().any(|file| file == FURNACE_FUELS) {
                continue;
            }
            let doc: serde_json::Value =
                match serde_json::from_slice(&m.content.file(FURNACE_FUELS)?) {
                    Ok(doc) => doc,
                    Err(error) => {
                        warnings.push(format!("mod {}: {FURNACE_FUELS}: {error}", m.id));
                        continue;
                    }
                };
            if doc.get("replace") == Some(&serde_json::Value::Bool(true)) {
                replace_furnace_fuels = true;
                furnace_fuels.clear();
            }
            if let Some(values) = doc.get("values").and_then(serde_json::Value::as_object) {
                for (item, value) in values {
                    if item.starts_with('#') {
                        warnings.push(format!(
                            "mod {}: {FURNACE_FUELS}: item tag {item} is not supported yet",
                            m.id
                        ));
                        continue;
                    }
                    let burn_time = value
                        .get("burn_time")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                        .filter(|value| *value > 0);
                    if let Some(burn_time) = burn_time {
                        furnace_fuels.insert(item.clone(), Some(burn_time));
                    } else {
                        warnings.push(format!(
                            "mod {}: {FURNACE_FUELS}: invalid burn_time for {item}",
                            m.id
                        ));
                    }
                }
            }
            if let Some(remove) = doc.get("remove").and_then(serde_json::Value::as_array) {
                for item in remove.iter().filter_map(serde_json::Value::as_str) {
                    if item.starts_with('#') {
                        warnings.push(format!(
                            "mod {}: {FURNACE_FUELS}: item tag removal {item} is not supported yet",
                            m.id
                        ));
                    } else {
                        furnace_fuels.insert(item.to_owned(), None);
                    }
                }
            }
        }
        if !furnace_fuels.is_empty() {
            let mut known: BTreeSet<String> = match read_json(&base.join("items.json"))? {
                J::Obj(entries) => entries.into_iter().map(|(id, _)| id).collect(),
                _ => return Err("items.json: no item registry".into()),
            };
            known.extend(items.iter().map(|item| item.rename.to.clone()));
            furnace_fuels.retain(|item, _| {
                let exists = known.contains(item);
                if !exists {
                    warnings.push(format!(
                        "{FURNACE_FUELS}: {item} is not registered in this mod set; ignored"
                    ));
                }
                exists
            });
        }
        Ok(Plan {
            blocks,
            items,
            furnace_fuels,
            replace_furnace_fuels,
            warnings,
            states: names.len(),
            defaults,
            sounds,
        })
    }

    /// What moves in a new block's copies of its template's rows: the
    /// block, and the template's item when it has another name. A block
    /// without an item gets `minecraft:air` where rows name one.
    fn renames(&self, b: &NewBlock) -> Vec<Rename> {
        let mut out = vec![b.rename.clone()];
        if let Some(item) = b.item.as_deref().filter(|i| *i != b.decl.template)
            && b.decl.item
        {
            out.push(Rename::new(item, &b.decl.id));
        }
        out
    }

    /// Appends each new block's copies of the template's states to `list`;
    /// `gameplay` rows get their index and the declared overrides.
    fn states(
        &self,
        list: &mut Vec<J>,
        ids: bool,
        gameplay: bool,
        registry_identity: bool,
    ) -> Result<()> {
        if list.len() != self.states {
            return Err(format!("{} states, not {}", list.len(), self.states).into());
        }
        for b in &self.blocks {
            let renames = self.renames(b);
            let source_ids: Vec<u32> = if b.custom_states.is_empty() {
                b.rename.states.clone().collect()
            } else {
                vec![b.rename.states.start; b.custom_states.len()]
            };
            for (i, id) in source_ids.into_iter().enumerate() {
                let mut row = list[id as usize].clone();
                for r in &renames {
                    if ids && r.to == b.decl.id && !b.custom_states.is_empty() {
                        let mut state_rename = r.clone();
                        state_rename.states = id..id + 1;
                        state_rename.first = b.rename.first + i as u32;
                        row.rename(&state_rename, true);
                    } else {
                        row.rename(r, ids);
                    }
                }
                if gameplay {
                    row.set("id", J::Num((b.rename.first + i as u32).into()));
                    if !b.decl.item {
                        row.set("item", J::Str("minecraft:air".into()));
                    }
                    self.override_gameplay(b, &mut row);
                } else if registry_identity && !b.custom_states.is_empty() {
                    // facts.json's fifth column is the block registry identity.
                    if let J::Arr(columns) = &mut row
                        && columns.len() > 4
                    {
                        columns[4] = J::Obj(vec![
                            ("id".into(), J::Str(b.decl.id.clone())),
                            (
                                "properties".into(),
                                J::Obj(
                                    b.custom_states[i]
                                        .iter()
                                        .map(|(k, v)| (k.clone(), J::Str(v.clone())))
                                        .collect(),
                                ),
                            ),
                        ]);
                    }
                }
                list.push(row);
            }
        }
        Ok(())
    }

    /// The declared properties, on a gameplay row.
    fn override_gameplay(&self, b: &NewBlock, row: &mut J) {
        let p = &b.decl.properties;
        if let Some(v) = p.hardness {
            row.set("hardness", J::f64(v as f64));
        }
        if let Some(v) = p.explosion_resistance {
            row.set("explosion_resistance", J::f64(v as f64));
        }
        if let Some(v) = p.light {
            row.set("emission", J::Num(v.into()));
        }
        if let Some(v) = p.requires_tool {
            row.set("requires_tool", J::Bool(v));
        }
        if let Some(v) = p.friction {
            row.set("friction", J::f64(v as f64));
        }
        if let Some(v) = p.map_color {
            let col = if v == 0 { 0 } else { MAP_COLORS[v as usize] };
            row.set("map_color", J::Num(col.into()));
        }
        if let Some(s) = &p.sounds {
            row.set("sound", self.sounds[s].clone());
        }
    }

    /// Adds each new block's copy of the template's entry to a map by name.
    fn names(&self, map: &mut J, ids: bool) {
        for b in &self.blocks {
            if let Some(v) = map.get(&b.decl.template) {
                let mut v = v.clone();
                for r in &self.renames(b) {
                    v.rename(r, ids);
                }
                map.set(&b.decl.id, v);
            }
        }
    }

    /// The baked model catalog binds blockstate conditions against the
    /// registered block properties. A custom-state declaration may differ
    /// from its vanilla template, so its model-state schema must follow the
    /// declared axes and their registered combinations as well.
    fn model_states(&self, map: &mut J) {
        for b in &self.blocks {
            if b.custom_states.is_empty() {
                if let Some(mut v) = map.get(&b.decl.template).cloned() {
                    for rename in &self.renames(b) {
                        v.rename(rename, false);
                    }
                    map.set(&b.decl.id, v);
                }
                continue;
            }
            let schema = J::Obj(
                b.decl
                    .state_properties
                    .iter()
                    .map(|(property, values)| {
                        (
                            property.clone(),
                            J::Obj(vec![(
                                "named".into(),
                                J::Arr(values.iter().cloned().map(J::Str).collect()),
                            )]),
                        )
                    })
                    .collect(),
            );
            let states = J::Arr(
                b.custom_states
                    .iter()
                    .map(|properties| {
                        J::Obj(
                            properties
                                .iter()
                                .map(|(property, value)| (property.clone(), J::Str(value.clone())))
                                .collect(),
                        )
                    })
                    .collect(),
            );
            map.set(
                &b.decl.id,
                J::Obj(vec![("schema".into(), schema), ("states".into(), states)]),
            );
        }
    }

    /// Adds each new item's copy of its template's entry to a map by name;
    /// `components` applies the overrides (items.json).
    fn items(&self, map: &mut J, components: bool) {
        for i in &self.items {
            let Some(v) = map.get(&i.rename.from) else {
                continue;
            };
            let mut v = v.clone();
            v.rename(&i.rename, false);
            if components {
                if let Some(n) = i.max_stack_size {
                    v.set("minecraft:max_stack_size", J::Num(n.into()));
                }
                if let Some(r) = i.rarity {
                    let r = match r {
                        Rarity::Common => "common",
                        Rarity::Uncommon => "uncommon",
                        Rarity::Rare => "rare",
                        Rarity::Epic => "epic",
                    };
                    v.set("minecraft:rarity", J::Str(r.into()));
                }
            }
            map.set(&i.rename.to, v);
        }
    }

    /// Applies NeoForge's static furnace-fuel data map to Horizon's item
    /// component table. Data-map entries override only the burn time; retain
    /// a template's speed provider, or use vanilla's default for new fuels.
    fn furnace_fuels(&self, doc: &mut J) -> Result<()> {
        let J::Obj(items) = doc else {
            return Err("items.json: no item registry".into());
        };
        if self.replace_furnace_fuels {
            for (_, components) in items.iter_mut() {
                if let J::Obj(components) = components {
                    components.retain(|(name, _)| name != "minecraft:cooking_fuel");
                }
            }
        }
        for (id, burn_time) in &self.furnace_fuels {
            let Some((_, components)) = items.iter_mut().find(|(name, _)| name == id) else {
                continue;
            };
            let J::Obj(components) = components else {
                return Err(format!("items.json: {id} is not an item component object").into());
            };
            match burn_time {
                Some(burn_time) => {
                    let speed = components
                        .iter()
                        .find(|(name, _)| name == "minecraft:cooking_fuel")
                        .and_then(|(_, fuel)| fuel.get("speed_multiplier"))
                        .cloned()
                        .unwrap_or_else(|| J::Str("minecraft:cooking/speed_default".into()));
                    let mut fuel = J::Obj(Vec::new());
                    fuel.set("burn_time", J::Num((*burn_time).into()));
                    fuel.set("speed_multiplier", speed);
                    if let Some((_, old)) = components
                        .iter_mut()
                        .find(|(name, _)| name == "minecraft:cooking_fuel")
                    {
                        *old = fuel;
                    } else {
                        components.push(("minecraft:cooking_fuel".into(), fuel));
                    }
                }
                None => components.retain(|(name, _)| name != "minecraft:cooking_fuel"),
            }
        }
        Ok(())
    }

    /// One JSON table, rewritten.
    fn json(&self, name: &str, doc: &mut J) -> Result<()> {
        let states = |doc: &mut J, path: &str, ids: bool, gameplay: bool| -> Result<()> {
            match doc.at(path) {
                Some(J::Arr(a)) => self.states(a, ids, gameplay, name == "facts.json"),
                _ => Err(format!("no list at /{path}").into()),
            }
        };
        match name {
            "facts.json" => {
                states(doc, "states", false, false)?;
                let defaults = doc.at("defaults").ok_or("no defaults")?;
                for b in &self.blocks {
                    let default = self.defaults[&b.decl.template];
                    let id = b.rename.id(default).ok_or("a default outside its block")?;
                    defaults.set(&b.decl.id, J::Num(id.into()));
                }
            }
            "gameplay-facts.json" => states(doc, "states", false, true)?,
            "block-behaviours.json" => {
                states(doc, "states", false, false)?;
                let Some(J::Arr(blocks)) = doc.at("blocks") else {
                    return Err("no blocks".into());
                };
                for b in &self.blocks {
                    let template = blocks.iter().find(
                        |x| matches!(x.get("name"), Some(J::Str(n)) if *n == b.decl.template),
                    );
                    if let Some(mut v) = template.cloned() {
                        for r in &self.renames(b) {
                            v.rename(r, false);
                        }
                        if !b.decl.item {
                            v.set("item", J::Str("minecraft:air".into()));
                        }
                        blocks.push(v);
                    }
                }
                self.items(doc.at("items").ok_or("no items")?, false);
                for odds in ["fire/burnOdds", "fire/igniteOdds"] {
                    if let Some(map) = doc.at(odds) {
                        self.names(map, false);
                    }
                }
            }
            "model-states.json" => self.model_states(doc),
            "landing-definitions.json" | "worldgen/blocks/survival-rules.json" => {
                self.names(doc, false)
            }
            "render-definitions.json" => self.names(doc, true),
            "worldgen/blocks/survival-extra-definitions.json" => {
                if let Some(map) = doc.at("ignition_odds") {
                    self.names(map, false);
                }
            }
            "items.json" => {
                self.items(doc, true);
                self.furnace_fuels(doc)?;
            }
            "registries.json" => {
                // Network ids: the game and its server map items and blocks through
                // this report, so new ones get the ids after vanilla's, in plan order.
                let new_items = self.items.iter().map(|i| i.rename.to.clone());
                let new_blocks = self.blocks.iter().map(|b| b.decl.id.clone());
                for (registry, names) in [
                    ("minecraft:item", new_items.collect::<Vec<_>>()),
                    ("minecraft:block", new_blocks.collect()),
                ] {
                    let Some(J::Obj(entries)) =
                        doc.get_mut(registry).and_then(|r| r.get_mut("entries"))
                    else {
                        return Err(format!("no {registry} entries").into());
                    };
                    let next = entries
                        .iter()
                        .filter_map(|(_, e)| match e {
                            J::Obj(o) => o.iter().find(|(k, _)| k == "protocol_id")?.1.as_u32(),
                            _ => None,
                        })
                        .max()
                        .map_or(0, |m| m + 1);
                    for (i, name) in names.into_iter().enumerate() {
                        if entries.iter().any(|(k, _)| *k == name) {
                            continue;
                        }
                        let id = J::Num((next + i as u32).into());
                        entries.push((name, J::Obj(vec![("protocol_id".into(), id)])));
                    }
                }
            }
            "command-registries.json" => {
                self.names(doc.at("blocks").ok_or("no blocks")?, false);
                let new_items = || self.items.iter().map(|i| J::Str(i.rename.to.clone()));
                let new_blocks = || self.blocks.iter().map(|b| J::Str(b.decl.id.clone()));
                let Some(J::Arr(items)) = doc.at("items") else {
                    return Err("no items".into());
                };
                items.extend(new_items());
                // The registries commands check ids against.
                if let Some(J::Arr(ids)) = doc.at("by_registry/minecraft:item/ids") {
                    ids.extend(new_items());
                }
                if let Some(J::Arr(ids)) = doc.at("by_registry/minecraft:block/ids") {
                    ids.extend(new_blocks());
                }
            }
            "mob-data.json" => states(doc, "states", false, false)?,
            "creative-inventory.json" => {
                let Some(J::Arr(tabs)) = doc.at("tabs") else {
                    return Err("creative-inventory.json: missing tabs".into());
                };
                let block_ids: std::collections::BTreeSet<&str> =
                    self.blocks.iter().map(|b| b.decl.id.as_str()).collect();
                let mut by_tab: std::collections::BTreeMap<&str, Vec<J>> =
                    std::collections::BTreeMap::new();
                for i in &self.items {
                    let id = i.rename.to.as_str();
                    let is_block = block_ids.contains(id);
                    let tab_id = creative_tab_for_item(id, i.rename.from.as_str(), is_block);
                    by_tab
                        .entry(tab_id)
                        .or_default()
                        .push(J::Str(format!("{{count:1,id:\"{id}\"}}")));
                }
                for tab in tabs.iter_mut() {
                    let id = match tab.get("id") {
                        Some(J::Str(s)) => s.as_str(),
                        _ => "",
                    };
                    let kind = match tab.get("type") {
                        Some(J::Str(s)) => s.as_str(),
                        _ => "",
                    };
                    if kind == "CATEGORY" {
                        if let Some(mod_items) = by_tab.remove(id)
                            && let Some(J::Arr(items)) = tab.get_mut("items")
                        {
                            items.splice(0..0, mod_items);
                        }
                    } else if kind == "SEARCH" {
                        let mod_items: Vec<J> = self
                            .items
                            .iter()
                            .map(|i| J::Str(format!("{{count:1,id:\"{}\"}}", i.rename.to)))
                            .collect();
                        let count = mod_items.len();
                        if let Some(J::Arr(items)) = tab.get_mut("items") {
                            items.splice(0..0, mod_items);
                        }
                        if let Some(J::Arr(lines)) = tab.get_mut("lines") {
                            lines.splice(0..0, (0..count).map(|_| J::Arr(Vec::new())));
                        }
                        if let Some(J::Arr(tags)) = tab.get_mut("tags") {
                            tags.splice(0..0, (0..count).map(|_| J::Arr(Vec::new())));
                        }
                    }
                }
            }
            "worldgen/block_flags.json" => states(doc, "", false, false)?,
            "worldgen/blocks/structure-blocks.json" => {
                for key in ["rotate", "mirror", "full_block"] {
                    states(doc, key, true, false)?;
                }
                let added: usize = self.blocks.iter().map(block_state_count).sum();
                doc.set("states", J::Num(((self.states + added) as u64).into()));
            }
            _ => return Err("no rule for this table".into()),
        }
        Ok(())
    }

    /// One NBT table: `states` lists, `cached_shapes` arrays and `blocks`
    /// compounds, wherever they are.
    fn nbt(&self, tag: &mut Tag) {
        let Tag::Compound(c) = tag else {
            return;
        };
        for (key, value) in c.iter_mut() {
            match (key.to_string_lossy().as_str(), value) {
                ("states", Tag::List(list)) if list.len() == self.states => {
                    for b in &self.blocks {
                        let source_ids: Vec<u32> = if b.custom_states.is_empty() {
                            b.rename.states.clone().collect()
                        } else {
                            vec![b.rename.states.start; b.custom_states.len()]
                        };
                        for (i, id) in source_ids.into_iter().enumerate() {
                            let mut state = list[id as usize].clone();
                            if let Some(slot) = state.get_mut("id") {
                                *slot = Tag::String(b.decl.id.as_str().into());
                            }
                            if !b.custom_states.is_empty() {
                                let properties = b.custom_states[i]
                                    .iter()
                                    .map(|(k, v)| {
                                        (
                                            JavaString::from(k.as_str()),
                                            Tag::String(v.as_str().into()),
                                        )
                                    })
                                    .collect();
                                if let Tag::Compound(map) = &mut state {
                                    map.insert(
                                        JavaString::from("properties"),
                                        Tag::Compound(properties),
                                    );
                                }
                            }
                            list.push(state);
                        }
                    }
                }
                ("cached_shapes", Tag::IntArray(a)) if a.len() == self.states => {
                    for b in &self.blocks {
                        if b.custom_states.is_empty() {
                            let r = &b.rename.states;
                            let copy = a[r.start as usize..r.end as usize].to_vec();
                            a.extend(copy);
                        } else {
                            a.extend(std::iter::repeat_n(
                                a[b.rename.states.start as usize],
                                b.custom_states.len(),
                            ));
                        }
                    }
                }
                ("blocks", Tag::Compound(map)) => {
                    for b in &self.blocks {
                        let Some(mut v) = map
                            .get(&JavaString::from(b.decl.template.as_str()))
                            .cloned()
                        else {
                            continue;
                        };
                        if let (Some(f), Some(slot)) =
                            (b.decl.properties.friction, v.get_mut("friction"))
                        {
                            *slot = Tag::Float(f.to_bits());
                        }
                        map.insert(b.decl.id.as_str().into(), v);
                    }
                }
                (_, inner @ Tag::Compound(_)) => self.nbt(inner),
                _ => {}
            }
        }
    }
}

fn creative_tab_for_item(id: &str, template: &str, is_block: bool) -> &'static str {
    let path = id.split_once(':').map_or(id, |(_, p)| p);
    let has = |words: &[&str]| words.iter().any(|w| path.contains(w));
    if path.ends_with("_spawn_egg") || path.starts_with("spawn_egg") {
        return "minecraft:spawn_eggs";
    }
    if !is_block {
        if has(&[
            "sword",
            "bow",
            "crossbow",
            "trident",
            "spear",
            "dagger",
            "knife",
            "katana",
            "halberd",
            "mace",
            "rapier",
            "scythe",
            "glaive",
            "claymore",
            "battleaxe",
            "warhammer",
            "shield",
            "quiver",
            "arrow",
            "bullet",
            "ammo",
            "helmet",
            "chestplate",
            "leggings",
            "boots",
            "cuirass",
            "greaves",
            "gauntlet",
            "armor",
            "staff",
            "spell",
            "tome",
            "turret",
        ]) {
            return "minecraft:combat";
        }
        if has(&[
            "pickaxe",
            "shovel",
            "hoe",
            "axe",
            "paxel",
            "excavator",
            "hammer",
            "mattock",
            "saw",
            "wrench",
            "screwdriver",
            "crowbar",
            "configurator",
            "multitool",
            "drill",
            "chainsaw",
            "shears",
            "fishing_rod",
            "brush",
            "bucket",
            "canister",
            "backpack",
            "pouch",
            "bag",
            "sack",
            "wallet",
            "compass",
            "clock",
            "spyglass",
            "map",
            "atlas",
            "waystone",
            "warp",
            "teleport",
            "magnet",
            "charm",
            "ring",
            "amulet",
            "belt",
            "necklace",
            "bracelet",
            "bauble",
            "curio",
            "relic",
            "artifact",
            "totem",
            "talisman",
            "jetpack",
            "glider",
            "elytra",
            "wings",
            "parachute",
            "hook",
            "grapple",
            "boat",
            "raft",
            "minecart",
            "locomotive",
            "carriage",
            "saddle",
            "harness",
            "lead",
            "lasso",
            "net",
            "upgrade",
            "augment",
            "card",
            "filter",
            "module",
            "remote",
            "terminal",
            "wireless",
            "portable",
            "scanner",
            "meter",
            "gauge",
            "linker",
            "binder",
            "wand",
            "rod",
            "chisel",
            "trowel",
            "key",
            "lock",
            "music_disc",
            "disc",
            "horn",
            "broom",
            "paraglider",
        ]) {
            return "minecraft:tools_and_utilities";
        }
        if has(&[
            "apple",
            "bread",
            "meat",
            "beef",
            "pork",
            "mutton",
            "chicken",
            "fish",
            "salmon",
            "cod",
            "berry",
            "berries",
            "fruit",
            "pie",
            "cake",
            "cookie",
            "stew",
            "soup",
            "salad",
            "sandwich",
            "burger",
            "pizza",
            "pasta",
            "noodle",
            "rice",
            "sushi",
            "taco",
            "burrito",
            "toast",
            "waffle",
            "pancake",
            "donut",
            "chocolate",
            "candy",
            "ice_cream",
            "cheese",
            "butter",
            "yogurt",
            "milk",
            "juice",
            "tea",
            "coffee",
            "soda",
            "wine",
            "beer",
            "ale",
            "cider",
            "mead",
            "smoothie",
            "shake",
            "jam",
            "jelly",
            "sauce",
            "curry",
            "roast",
            "steak",
            "bacon",
            "sausage",
            "ham",
            "omelet",
            "potato",
            "carrot",
            "onion",
            "tomato",
            "lettuce",
            "cabbage",
            "corn",
            "pepper",
            "garlic",
            "ginger",
            "lemon",
            "lime",
            "orange",
            "peach",
            "pear",
            "plum",
            "cherry",
            "mango",
            "banana",
            "coconut",
            "pineapple",
            "strawberry",
            "blueberry",
            "raspberry",
            "blackberry",
            "cranberry",
            "grape",
            "melon",
            "squash",
            "fillet",
            "skewer",
            "kebab",
            "dumpling",
            "croissant",
            "bagel",
            "pretzel",
            "cracker",
            "chips",
            "fries",
            "popcorn",
            "jerky",
            "ration",
            "meal",
            "feast",
            "platter",
            "bowl_of",
            "slice_of",
            "cooked_",
            "baked_",
            "fried_",
            "grilled_",
            "roasted_",
            "smoked_",
            "steamed_",
            "boiled_",
            "pickled_",
            "candied_",
            "glazed_",
            "stuffed_",
            "potion",
            "elixir",
            "brew",
            "tonic",
            "flask",
            "bottle",
        ]) {
            return "minecraft:food_and_drinks";
        }
        return "minecraft:ingredients";
    }
    if has(&[
        "pipe",
        "cable",
        "wire",
        "conduit",
        "tube",
        "duct",
        "laser",
        "router",
        "relay",
        "switch",
        "button",
        "lever",
        "pressure_plate",
        "sensor",
        "detector",
        "observer",
        "comparator",
        "repeater",
        "redstone",
        "logic",
        "timer",
        "counter",
        "sequencer",
        "transmitter",
        "receiver",
        "antenna",
        "piston",
        "bearing",
        "chassis",
        "gantry",
        "pulley",
        "clutch",
        "gearshift",
        "gearbox",
        "shaft",
        "cogwheel",
        "chute",
        "funnel",
        "tunnel",
        "mechanical_",
    ]) || template.contains("redstone")
        || template.ends_with("_button")
        || template.ends_with("_pressure_plate")
        || template == "minecraft:lever"
    {
        return "minecraft:redstone_blocks";
    }
    if has(&[
        "machine",
        "furnace",
        "smelter",
        "crusher",
        "pulverizer",
        "grinder",
        "mill",
        "press",
        "mixer",
        "sawmill",
        "centrifuge",
        "electrolyzer",
        "infuser",
        "enrich",
        "combiner",
        "compressor",
        "extruder",
        "condenser",
        "evaporator",
        "refinery",
        "distiller",
        "boiler",
        "turbine",
        "generator",
        "dynamo",
        "reactor",
        "solar",
        "battery",
        "capacitor",
        "energy_cube",
        "controller",
        "interface",
        "importer",
        "exporter",
        "drive",
        "terminal",
        "monitor",
        "crafting",
        "workbench",
        "table",
        "station",
        "anvil",
        "loom",
        "grindstone",
        "kiln",
        "oven",
        "stove",
        "grill",
        "pot",
        "skillet",
        "kettle",
        "cauldron",
        "vat",
        "tank",
        "drum",
        "barrel",
        "chest",
        "crate",
        "drawer",
        "cabinet",
        "cupboard",
        "locker",
        "safe",
        "vault",
        "basket",
        "jar",
        "urn",
        "pedestal",
        "altar",
        "shrine",
        "pylon",
        "obelisk",
        "waystone",
        "teleporter",
        "portal",
        "anchor",
        "beacon",
        "trophy",
        "plush",
        "statue",
        "globe",
        "clock",
        "bell",
        "sign",
        "board",
        "shelf",
        "bookcase",
        "bookshelf",
        "lectern",
        "desk",
        "chair",
        "stool",
        "bench",
        "sofa",
        "couch",
        "bed",
        "hammock",
        "sleeping_bag",
        "lamp",
        "lantern",
        "torch",
        "sconce",
        "chandelier",
        "brazier",
        "candle",
        "light",
        "fixture",
        "bulb",
        "neon",
        "sink",
        "faucet",
        "toilet",
        "bath",
        "shower",
        "mirror",
        "curtain",
        "blinds",
        "awning",
        "mailbox",
        "trash",
        "bin",
        "fan",
        "vent",
        "chimney",
        "scarecrow",
        "dummy",
        "target",
        "spawner",
        "cage",
        "trap",
    ]) || matches!(
        template,
        "minecraft:torch"
            | "minecraft:lantern"
            | "minecraft:soul_lantern"
            | "minecraft:crafting_table"
            | "minecraft:furnace"
            | "minecraft:chest"
            | "minecraft:barrel"
    ) {
        return "minecraft:functional_blocks";
    }
    if has(&[
        "white_",
        "orange_",
        "magenta_",
        "light_blue_",
        "yellow_",
        "lime_",
        "pink_",
        "gray_",
        "light_gray_",
        "cyan_",
        "purple_",
        "blue_",
        "brown_",
        "green_",
        "red_",
        "black_",
        "stained",
        "dyed",
        "carpet",
        "wool",
        "terracotta",
        "concrete",
    ]) || template.contains("stained_glass")
        || template.contains("wool")
        || template.contains("carpet")
        || template.contains("terracotta")
        || template.contains("concrete")
    {
        return "minecraft:colored_blocks";
    }
    if has(&[
        "ore",
        "log",
        "wood",
        "leaves",
        "sapling",
        "flower",
        "grass",
        "fern",
        "bush",
        "shrub",
        "vine",
        "moss",
        "mushroom",
        "fungus",
        "roots",
        "lily",
        "lotus",
        "orchid",
        "rose",
        "tulip",
        "daisy",
        "dandelion",
        "poppy",
        "allium",
        "peony",
        "lilac",
        "sunflower",
        "blossom",
        "petal",
        "coral",
        "kelp",
        "seagrass",
        "algae",
        "reed",
        "bamboo",
        "cactus",
        "succulent",
        "crop",
        "seed",
        "berry_bush",
        "soil",
        "dirt",
        "mud",
        "clay",
        "sand",
        "gravel",
        "pebble",
        "crystal",
        "geode",
        "cluster",
        "bud",
    ]) || matches!(
        template,
        "minecraft:oak_log"
            | "minecraft:oak_leaves"
            | "minecraft:short_grass"
            | "minecraft:dandelion"
            | "minecraft:poppy"
            | "minecraft:oak_sapling"
            | "minecraft:dirt"
            | "minecraft:grass_block"
            | "minecraft:sand"
            | "minecraft:gravel"
    ) {
        return "minecraft:natural_blocks";
    }
    "minecraft:building_blocks"
}

/// The JSON and NBT tables rewritten for new blocks and items.
const JSON_TABLES: [&str; 15] = [
    "facts.json",
    "registries.json",
    "gameplay-facts.json",
    "block-behaviours.json",
    "model-states.json",
    "render-definitions.json",
    "landing-definitions.json",
    "items.json",
    "command-registries.json",
    "creative-inventory.json",
    "mob-data.json",
    "worldgen/block_flags.json",
    "worldgen/blocks/structure-blocks.json",
    "worldgen/blocks/survival-rules.json",
    "worldgen/blocks/survival-extra-definitions.json",
];
const NBT_TABLES: [&str; 6] = [
    "occlusion.nbt",
    "cached-collision.nbt",
    "movement-properties.nbt",
    "worldgen/blocks/block-properties.nbt",
    "worldgen/blocks/cached-support.nbt",
    "worldgen/blocks/cached-collision.nbt",
];
/// In a root built here: the namespaces the content mods own (their ids and
/// listed namespaces), which loaders read besides `minecraft`.
const MARKER: &str = "horizon-mod-content.json";
/// Block states a world can hold. Chunk sections widen their global palette
/// past vanilla's 16 bits to fit the registry (crate::chunk::set_block_states);
/// this caps it at 20 bits, about 29 times vanilla's 35,723 states. Mods past
/// it are refused, not wrapped onto other blocks.
pub const MAX_STATES: u32 = 1 << 20;

/// The content mods' namespaces in a root built by [`build`]; none in any
/// other root, so vanilla's loaders read exactly what they did.
pub fn namespaces(root: &Path) -> Vec<String> {
    fs::read(root.join(MARKER))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// The namespaces whose tags loaders read in `root`: `minecraft`, and in a
/// root built by [`build`] the convention tags (`c`, shared between mods as
/// on NeoForge and Fabric) and the content mods' own.
pub fn tag_namespaces(root: &Path) -> Vec<String> {
    let mods = namespaces(root);
    let mut out = vec!["minecraft".to_owned()];
    if !mods.is_empty() {
        out.push("c".to_owned());
        out.extend(mods);
    }
    out
}

/// Bundles of block tag files (tag id → the tag file's text).
const TAG_BUNDLES: [&str; 2] = ["block-tags.json", "worldgen/block_tags.json"];
/// The worldgen loader's bundle of biome tags.
const BIOME_TAGS: &str = "worldgen/biome_tags.json";
/// Worldgen registries a content mod may add to, and the bundle the
/// worldgen loader reads each from (id → the file's text).
const WORLDGEN_BUNDLES: [(&str, &str); 12] = [
    ("structure", "worldgen/structures.json"),
    ("structure_set", "worldgen/structure_sets.json"),
    ("template_pool", "worldgen/template_pools.json"),
    ("processor_list", "worldgen/processor_lists.json"),
    ("configured_feature", "worldgen/features.json"),
    ("placed_feature", "worldgen/placed_features.json"),
    ("configured_carver", "worldgen/carvers.json"),
    ("noise", "worldgen/noises.json"),
    ("density_function", "worldgen/densities.json"),
    ("noise_settings", "worldgen/settings.json"),
    ("world_preset", "worldgen/world_presets.json"),
    ("biome", "worldgen/biomes.json"),
];

/// A mod file that adds to a worldgen bundle: (bundle, id).
fn worldgen_entry(file: &str) -> Option<(&'static str, String)> {
    let path = file.strip_prefix("data/")?;
    let (ns, path) = path.split_once('/')?;
    let rest = path.strip_prefix("worldgen/")?;
    let (kind, path) = rest.split_once('/')?;
    let (_, bundle) = WORLDGEN_BUNDLES.iter().find(|(k, _)| *k == kind)?;
    Some((bundle, format!("{ns}:{}", path.strip_suffix(".json")?)))
}

fn is_biome_modifier_file(file: &str) -> bool {
    let Some((_, rest)) = file.strip_prefix("data/").and_then(|r| r.split_once('/')) else {
        return false;
    };
    (rest.starts_with("neoforge/biome_modifier/") || rest.starts_with("forge/biome_modifier/"))
        && rest.ends_with(".json")
}

/// The density decoder only implements vanilla's density types. A mod's
/// custom type cannot be represented as a registry entry and must not make
/// every other dimension's content root fail to load.
fn custom_density_type(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| kind.contains(':') && !kind.starts_with("minecraft:"))
            .map(str::to_owned)
            .or_else(|| object.values().find_map(custom_density_type)),
        serde_json::Value::Array(values) => values.iter().find_map(custom_density_type),
        _ => None,
    }
}

fn references_density(
    value: &serde_json::Value,
    ids: &std::collections::BTreeSet<String>,
    field: Option<&str>,
) -> bool {
    match value {
        serde_json::Value::String(id) => {
            field != Some("noise") && field != Some("type") && ids.contains(id)
        }
        serde_json::Value::Object(object) => object
            .iter()
            .any(|(key, value)| references_density(value, ids, Some(key))),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| references_density(value, ids, field)),
        _ => false,
    }
}

fn flatten_config_recursive(val: &mut serde_json::Value, changed: &mut bool) {
    use serde_json::Value;
    match val {
        Value::Object(map) => {
            if map.contains_key("type")
                && let Some(Value::Object(cfg)) = map.remove("config")
            {
                *changed = true;
                for (k, v) in cfg {
                    map.entry(k).or_insert(v);
                }
            }
            for v in map.values_mut() {
                flatten_config_recursive(v, changed);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                flatten_config_recursive(v, changed);
            }
        }
        _ => {}
    }
}

/// Normalizes custom mod `configured_feature` and `placed_feature` JSONs into
/// vanilla-compatible representations that `horizon-world`'s worldgen engine
/// executes natively.
fn normalize_worldgen_entry(bundle: &str, text: &str) -> String {
    use serde_json::{Value, json};
    let Ok(mut doc) = serde_json::from_str::<Value>(text) else {
        return text.to_owned();
    };
    let mut changed = false;
    if bundle == "worldgen/features.json" {
        let ty = doc
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        match ty.as_str() {
            "mekanism:ore" | "mekanism:ore_retrogen" | "immersiveengineering:ie_ore" => {
                doc["type"] = Value::String("minecraft:ore".into());
                if let Some(cfg) = doc.get_mut("config").and_then(Value::as_object_mut) {
                    cfg.entry("size").or_insert_with(|| json!(9));
                    cfg.entry("discard_chance_on_air_exposure")
                        .or_insert_with(|| json!(0.0));
                }
                changed = true;
            }
            "silentgear:wild_plant" | "minecraft:flower" => {
                doc["type"] = Value::String("minecraft:random_patch".into());
                changed = true;
            }
            "farmersdelight:wild_crop" => {
                doc["type"] = Value::String("minecraft:random_patch".into());
                if let Some(cfg) = doc.get_mut("config").and_then(Value::as_object_mut) {
                    if let Some(primary) = cfg.remove("primary_feature") {
                        cfg.insert("feature".into(), primary);
                    }
                    cfg.remove("secondary_feature");
                    cfg.remove("floor_feature");
                }
                changed = true;
            }
            _ => {}
        }
        flatten_config_recursive(&mut doc, &mut changed);
    } else if bundle == "worldgen/settings.json" {
        // 26.3 added this router input. Older noise settings carried in
        // modpacks omit it; their vanilla-era analogue is the neutral field
        // value used by caves, the Nether, and the End.
        let router = doc.get_mut("noise_router").and_then(Value::as_object_mut);
        if let Some(router) = router
            && !router.contains_key("chunk_surface_level")
        {
            router.insert("chunk_surface_level".into(), json!(0.0));
            changed = true;
        }
    } else if bundle == "worldgen/carvers.json" {
        flatten_config_recursive(&mut doc, &mut changed);
        if let Some(object) = doc.as_object_mut()
            && let Some(y_scale) = object.remove("yScale")
        {
            if object.get("type").and_then(Value::as_str) == Some("minecraft:canyon") {
                let shape = object.entry("shape").or_insert_with(|| json!({}));
                if let Some(shape) = shape.as_object_mut() {
                    shape.entry("y_scale").or_insert(y_scale);
                }
            }
            changed = true;
        }
    } else if bundle == "worldgen/biomes.json" {
        if let Some(object) = doc.as_object_mut() {
            if let Some(stages) = object.get_mut("carvers").and_then(Value::as_object_mut) {
                let mut ordered = Vec::new();
                for stage in ["air", "liquid"] {
                    if let Some(entries) = stages.remove(stage).and_then(|v| v.as_array().cloned())
                    {
                        ordered.extend(entries);
                    }
                }
                for entries in stages.values_mut().filter_map(Value::as_array_mut) {
                    ordered.append(entries);
                }
                object.insert("carvers".into(), Value::Array(ordered));
                changed = true;
            }
            let spawners = object.remove("spawners");
            let spawn_costs = object.remove("spawn_costs");
            if spawners.is_some() || spawn_costs.is_some() {
                let mut categories = serde_json::Map::new();
                if let Some(spawners) = spawners.as_ref().and_then(Value::as_object) {
                    for (category, entries) in spawners {
                        let converted = entries
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|entry| {
                                let mut entry = entry.clone();
                                let min = entry
                                    .get("minCount")
                                    .or_else(|| entry.get("min_count"))
                                    .and_then(Value::as_u64)
                                    .unwrap_or(1);
                                let max = entry
                                    .get("maxCount")
                                    .or_else(|| entry.get("max_count"))
                                    .and_then(Value::as_u64)
                                    .unwrap_or(min);
                                if let Some(entry) = entry.as_object_mut() {
                                    entry.remove("minCount");
                                    entry.remove("maxCount");
                                    entry.remove("min_count");
                                    entry.remove("max_count");
                                    entry.insert(
                                        "count".into(),
                                        if min == max {
                                            Value::from(min)
                                        } else {
                                            json!({
                                                "type": "minecraft:uniform",
                                                "min_inclusive": min,
                                                "max_inclusive": max,
                                            })
                                        },
                                    );
                                }
                                entry
                            })
                            .collect();
                        categories.insert(category.clone(), Value::Array(converted));
                    }
                }
                let natural = json!({
                    "spawn_costs": spawn_costs.unwrap_or_else(|| json!({})),
                    "spawns_by_category": categories,
                });
                let attributes = object.entry("attributes").or_insert_with(|| json!({}));
                if let Some(attributes) = attributes.as_object_mut() {
                    attributes
                        .entry("minecraft:gameplay/natural_mob_spawns")
                        .or_insert(natural);
                }
                changed = true;
            }
        }
    } else if bundle == "worldgen/placed_features.json"
        && let Some(placement) = doc.get_mut("placement").and_then(Value::as_array_mut)
    {
        let mut out = Vec::with_capacity(placement.len());
        for mut m in std::mem::take(placement) {
            let mtype = m
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            match mtype.as_str() {
                "create:config_filter"
                | "mekanism:disableable"
                | "farmersdelight:biome_tag"
                | "artifacts:config_value_filter"
                | "yungsextras:rng_initializer"
                | "twilightforest:no_structure"
                | "the_bumblezone:structure_disallow_by_tag"
                | "aether:dungeon_blacklist_filter"
                | "pneumaticcraft:oil_lake_filter" => {
                    changed = true;
                    continue;
                }
                "immersiveengineering:ie_count" => {
                    m = json!({ "type": "minecraft:count", "count": 8 });
                    changed = true;
                }
                "repurposed_structures:unlimited_count" => {
                    m["type"] = Value::String("minecraft:count".into());
                    changed = true;
                }
                "eternal_starlight:height_range_filter" => {
                    m["type"] = Value::String("minecraft:height_range".into());
                    changed = true;
                }
                _ => {}
            }
            if m.get("type").and_then(Value::as_str) == Some("minecraft:count")
                && m.get("count")
                    .and_then(Value::as_object)
                    .and_then(|c| c.get("type"))
                    .and_then(Value::as_str)
                    == Some("mekanism:configurable_constant")
            {
                m["count"] = json!(8);
                changed = true;
            }
            if m.get("type").and_then(Value::as_str) == Some("minecraft:height_range")
                && matches!(
                    m.get("height")
                        .and_then(Value::as_object)
                        .and_then(|h| h.get("type"))
                        .and_then(Value::as_str),
                    Some("mekanism:configurable" | "immersiveengineering:ie_range")
                )
            {
                m["height"] = json!({
                    "type": "minecraft:trapezoid",
                    "min_inclusive": { "absolute": -64 },
                    "max_inclusive": { "absolute": 80 }
                });
                changed = true;
            }
            out.push(m);
        }
        *placement = out;
    }
    if changed {
        serde_json::to_string(&doc).unwrap_or_else(|_| text.to_owned())
    } else {
        text.to_owned()
    }
}

const DECORATION_STEPS: [&str; 11] = [
    "raw_generation",
    "lakes",
    "local_modifications",
    "underground_structures",
    "surface_structures",
    "strongholds",
    "underground_ores",
    "underground_decoration",
    "fluid_springs",
    "vegetal_decoration",
    "top_layer_modification",
];

fn expand_biome_tag(
    tag: &str,
    raw_tags: &BTreeMap<String, Vec<String>>,
    known_biomes: &std::collections::BTreeSet<String>,
    visiting: &mut std::collections::BTreeSet<String>,
    out: &mut std::collections::BTreeSet<String>,
) {
    let key = if tag.contains(':') {
        tag.to_owned()
    } else {
        format!("minecraft:{tag}")
    };
    if !visiting.insert(key.clone()) {
        return;
    }
    if let Some(entries) = raw_tags.get(&key) {
        for entry in entries {
            if let Some(sub) = entry.strip_prefix('#') {
                expand_biome_tag(sub, raw_tags, known_biomes, visiting, out);
            } else {
                let id = if entry.contains(':') {
                    entry.clone()
                } else {
                    format!("minecraft:{entry}")
                };
                if known_biomes.contains(&id) {
                    out.insert(id);
                }
            }
        }
    }
    visiting.remove(&key);
}

fn resolve_biome_selector(
    val: &serde_json::Value,
    raw_tags: &BTreeMap<String, Vec<String>>,
    known_biomes: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeSet<String> {
    use serde_json::Value;
    use std::collections::BTreeSet;
    let mut out = BTreeSet::new();
    match val {
        Value::String(s) => {
            if let Some(tag) = s.strip_prefix('#') {
                let mut visiting = BTreeSet::new();
                expand_biome_tag(tag, raw_tags, known_biomes, &mut visiting, &mut out);
            } else {
                let id = if s.contains(':') {
                    s.clone()
                } else {
                    format!("minecraft:{s}")
                };
                if known_biomes.contains(&id) {
                    out.insert(id);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                out.extend(resolve_biome_selector(item, raw_tags, known_biomes));
            }
        }
        Value::Object(map) => {
            let ty = map.get("type").and_then(Value::as_str).unwrap_or_default();
            match ty {
                "neoforge:and" | "forge:and" => {
                    if let Some(Value::Array(vals)) = map.get("values") {
                        let mut first = true;
                        for v in vals {
                            let sub = resolve_biome_selector(v, raw_tags, known_biomes);
                            if first {
                                out = sub;
                                first = false;
                            } else {
                                out.retain(|b| sub.contains(b));
                            }
                        }
                    }
                }
                "neoforge:or" | "forge:or" => {
                    if let Some(Value::Array(vals)) = map.get("values") {
                        for v in vals {
                            out.extend(resolve_biome_selector(v, raw_tags, known_biomes));
                        }
                    }
                }
                "neoforge:not" | "forge:not" => {
                    if let Some(v) = map.get("value") {
                        let excluded = resolve_biome_selector(v, raw_tags, known_biomes);
                        for b in known_biomes {
                            if !excluded.contains(b) {
                                out.insert(b.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    out
}

/// Applies NeoForge / Forge `add_features` biome modifiers to `worldgen/biomes.json`.
/// Added features are appended per step in canonical alphabetical order so
/// `FeatureDecorationPlan`'s cross-biome topological sort stays cycle-free.
fn apply_biome_modifiers(base: &Path, overlay: &Overlay<'_>, modifiers: &[String]) -> Result<()> {
    use serde_json::{Value, json};
    use std::collections::BTreeSet;
    if modifiers.is_empty() {
        return Ok(());
    }
    let biomes_rel = "worldgen/biomes.json";
    let biomes_path = if overlay.out.join(biomes_rel).exists() {
        overlay.out.join(biomes_rel)
    } else {
        base.join(biomes_rel)
    };
    if !biomes_path.exists() {
        return Ok(());
    }
    let mut biomes_doc = read_json(&biomes_path)?;
    let J::Obj(biome_entries) = &mut biomes_doc else {
        return Ok(());
    };
    let known_biomes: BTreeSet<String> = biome_entries.iter().map(|(k, _)| k.clone()).collect();

    let placed_path = if overlay.out.join("worldgen/placed_features.json").exists() {
        overlay.out.join("worldgen/placed_features.json")
    } else {
        base.join("worldgen/placed_features.json")
    };
    let known_placed: BTreeSet<String> = match read_json(&placed_path) {
        Ok(J::Obj(o)) => o.into_iter().map(|(k, _)| k).collect(),
        _ => BTreeSet::new(),
    };

    let biome_tags_path = if overlay.out.join(BIOME_TAGS).exists() {
        overlay.out.join(BIOME_TAGS)
    } else {
        base.join(BIOME_TAGS)
    };
    let mut raw_biome_tags: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Ok(J::Obj(o)) = read_json(&biome_tags_path) {
        for (tag_id, val) in o {
            let Some(J::Str(text)) = Some(val) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let mut items = Vec::new();
            for entry in v["values"].as_array().into_iter().flatten() {
                if let Some(s) = entry.as_str() {
                    items.push(s.to_owned());
                } else if let Some(s) = entry.get("id").and_then(Value::as_str) {
                    items.push(s.to_owned());
                }
            }
            let key = if tag_id.contains(':') {
                tag_id
            } else {
                format!("minecraft:{tag_id}")
            };
            raw_biome_tags.insert(key, items);
        }
    }
    for (alias, defaults) in [
        ("c:is_overworld", &["#minecraft:is_overworld"][..]),
        ("forge:is_overworld", &["#minecraft:is_overworld"][..]),
        ("c:is_nether", &["#minecraft:is_nether"][..]),
        ("forge:is_nether", &["#minecraft:is_nether"][..]),
        ("c:is_end", &["#minecraft:is_end"][..]),
        ("forge:is_end", &["#minecraft:is_end"][..]),
        ("c:is_forest", &["#minecraft:is_forest"][..]),
        (
            "c:is_tree/coniferous",
            &["#minecraft:is_taiga", "minecraft:grove"][..],
        ),
        ("c:is_tree/deciduous", &["#minecraft:is_forest"][..]),
        (
            "c:is_plains",
            &[
                "minecraft:plains",
                "minecraft:sunflower_plains",
                "minecraft:meadow",
            ][..],
        ),
        (
            "c:is_hot/overworld",
            &[
                "#minecraft:is_jungle",
                "#minecraft:is_savanna",
                "#minecraft:is_badlands",
                "minecraft:desert",
            ][..],
        ),
        (
            "c:is_cold/overworld",
            &[
                "#minecraft:is_taiga",
                "minecraft:snowy_plains",
                "minecraft:ice_spikes",
                "minecraft:frozen_peaks",
                "minecraft:jagged_peaks",
                "minecraft:snowy_slopes",
                "minecraft:grove",
            ][..],
        ),
        (
            "c:is_wet/overworld",
            &[
                "#minecraft:is_jungle",
                "minecraft:swamp",
                "minecraft:mangrove_swamp",
                "minecraft:lush_caves",
            ][..],
        ),
        (
            "c:is_dense_vegetation/overworld",
            &["#minecraft:is_forest", "#minecraft:is_jungle"][..],
        ),
        (
            "c:is_sparse_vegetation/overworld",
            &[
                "#minecraft:is_savanna",
                "#minecraft:is_badlands",
                "minecraft:plains",
            ][..],
        ),
        ("c:is_mushroom", &["minecraft:mushroom_fields"][..]),
        (
            "c:is_swamp",
            &["minecraft:swamp", "minecraft:mangrove_swamp"][..],
        ),
        ("c:is_sandy", &["minecraft:desert", "minecraft:beach"][..]),
        (
            "c:is_snowy",
            &[
                "minecraft:snowy_plains",
                "minecraft:ice_spikes",
                "minecraft:snowy_taiga",
                "minecraft:snowy_beach",
                "minecraft:snowy_slopes",
                "minecraft:grove",
            ][..],
        ),
    ] {
        raw_biome_tags
            .entry(alias.to_owned())
            .or_insert_with(|| defaults.iter().map(|s| (*s).to_owned()).collect());
    }

    let mut additions: BTreeMap<String, [BTreeSet<String>; 11]> = BTreeMap::new();
    let str_list = |v: &Value| -> Vec<String> {
        match v {
            Value::String(s) => vec![s.clone()],
            Value::Array(a) => a
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            _ => Vec::new(),
        }
    };
    let mut spawn_additions: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for text in modifiers {
        let Ok(doc) = serde_json::from_str::<Value>(text) else {
            continue;
        };
        let ty = doc.get("type").and_then(Value::as_str).unwrap_or_default();
        if ty == "neoforge:add_spawns" || ty == "forge:add_spawns" {
            let Some(biomes_val) = doc.get("biomes") else {
                continue;
            };
            let target_biomes = resolve_biome_selector(biomes_val, &raw_biome_tags, &known_biomes);
            let mut list = Vec::new();
            if let Some(sp) = doc.get("spawners") {
                if let Some(arr) = sp.as_array() {
                    list.extend(arr.iter().cloned());
                } else if sp.is_object() {
                    list.push(sp.clone());
                }
            }
            if !list.is_empty() {
                for b in target_biomes {
                    spawn_additions.entry(b).or_default().extend(list.clone());
                }
            }
            continue;
        }
        if ty == "aquaculture:fish_spawn" {
            let Some(biomes_val) = doc.get("includeBiomes") else {
                continue;
            };
            let mut target_biomes =
                resolve_biome_selector(biomes_val, &raw_biome_tags, &known_biomes);
            if let Some(denied) = doc.get("excludeBiomes") {
                let denied_set = resolve_biome_selector(denied, &raw_biome_tags, &known_biomes);
                target_biomes.retain(|b| !denied_set.contains(b));
            }
            if let Some(sp) = doc.get("spawn") {
                for b in target_biomes {
                    spawn_additions.entry(b).or_default().push(sp.clone());
                }
            }
            continue;
        }
        let (biomes_val, denied_val, features_val, step_raw) = match ty {
            "neoforge:add_features"
            | "forge:add_features"
            | "actuallyadditions:bool_config_feature_modifier" => (
                doc.get("biomes"),
                None,
                doc.get("features").or_else(|| doc.get("feature")),
                doc.get("step")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase(),
            ),
            "farmersdelight:add_features_by_filter" => (
                doc.get("allowed_biomes"),
                doc.get("denied_biomes"),
                doc.get("features"),
                doc.get("step")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase(),
            ),
            "productivebees:nest" => (
                doc.get("biomes"),
                None,
                doc.get("features"),
                doc.get("generation_stage")
                    .and_then(Value::as_str)
                    .unwrap_or("top_layer_modification")
                    .to_ascii_lowercase(),
            ),
            "bigreactors:oregen_yellorite"
            | "bigreactors:oregen_anglesite"
            | "bigreactors:oregen_benitoite"
            | "mysticalagriculture:inferium_ore"
            | "mysticalagriculture:prosperity_ore"
            | "mysticalagriculture:soulstone"
            | "mysticalagradditions:end_inferium_ore"
            | "mysticalagradditions:end_prosperity_ore"
            | "mysticalagradditions:nether_inferium_ore"
            | "mysticalagradditions:nether_prosperity_ore" => (
                doc.get("biomes"),
                None,
                doc.get("feature").or_else(|| doc.get("features")),
                doc.get("step")
                    .and_then(Value::as_str)
                    .unwrap_or("underground_ores")
                    .to_ascii_lowercase(),
            ),
            _ => continue,
        };
        let step_name = step_raw.trim_start_matches("minecraft:");
        let Some(step_idx) = DECORATION_STEPS.iter().position(|&s| s == step_name) else {
            continue;
        };
        let Some(biomes_val) = biomes_val else {
            continue;
        };
        let Some(features_val) = features_val else {
            continue;
        };
        let mut target_biomes = resolve_biome_selector(biomes_val, &raw_biome_tags, &known_biomes);
        if let Some(denied) = denied_val {
            let denied_set = resolve_biome_selector(denied, &raw_biome_tags, &known_biomes);
            target_biomes.retain(|b| !denied_set.contains(b));
        }
        if target_biomes.is_empty() {
            continue;
        }
        let valid_features: Vec<String> = str_list(features_val)
            .into_iter()
            .map(|f| {
                if f.contains(':') {
                    f
                } else {
                    format!("minecraft:{f}")
                }
            })
            .filter(|f| known_placed.contains(f))
            .collect();
        if valid_features.is_empty() {
            continue;
        }
        for biome in target_biomes {
            let steps = additions.entry(biome).or_default();
            for feat in &valid_features {
                steps[step_idx].insert(feat.clone());
            }
        }
    }
    if !additions.is_empty() {
        for (biome_id, val) in biome_entries.iter_mut() {
            let Some(per_step) = additions.get(biome_id) else {
                continue;
            };
            let J::Str(text) = val else {
                continue;
            };
            let Ok(mut bdoc) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            let Some(features) = bdoc.get_mut("features").and_then(Value::as_array_mut) else {
                continue;
            };
            while features.len() < 11 {
                features.push(Value::Array(Vec::new()));
            }
            for (step_idx, added) in per_step.iter().enumerate() {
                if added.is_empty() {
                    continue;
                }
                if let Some(step_arr) = features[step_idx].as_array_mut() {
                    let existing: BTreeSet<String> = step_arr
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect();
                    for feat in added {
                        if !existing.contains(feat) {
                            step_arr.push(Value::String(feat.clone()));
                        }
                    }
                }
            }
            *val = J::Str(serde_json::to_string(&bdoc)?);
        }
        overlay.write(biomes_rel, &serde_json::to_vec(&biomes_doc)?)?;
    }

    if !spawn_additions.is_empty() {
        for (biome_id, spawns) in spawn_additions {
            let Some(name) = biome_id.strip_prefix("minecraft:") else {
                continue;
            };
            let rel = format!("data/minecraft/worldgen/biome/{name}.json");
            let source_path = if overlay.out.join(&rel).exists() {
                overlay.out.join(&rel)
            } else {
                base.join(&rel)
            };
            if !source_path.exists() {
                continue;
            }
            let Ok(bytes) = fs::read(&source_path) else {
                continue;
            };
            let Ok(mut bdoc) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            let Value::Object(ref mut bdoc_map) = bdoc else {
                continue;
            };
            let Some(nat_map) = bdoc_map
                .entry("attributes")
                .or_insert_with(|| json!({}))
                .as_object_mut()
            else {
                continue;
            };
            let natural_target = if nat_map.contains_key("argument") {
                nat_map
                    .get_mut("argument")
                    .and_then(Value::as_object_mut)
                    .unwrap()
            } else {
                nat_map
                    .entry("minecraft:gameplay/natural_mob_spawns")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .unwrap()
            };
            let categories = natural_target
                .entry("spawns_by_category")
                .or_insert_with(|| json!({}));
            if let Some(cmap) = categories.as_object_mut() {
                for sp in spawns {
                    let ent_type = sp
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let lower = ent_type.to_ascii_lowercase();
                    let cat = if lower.contains("fish")
                        || lower.contains("aquaculture")
                        || lower.contains("water")
                    {
                        "water_creature"
                    } else if lower.contains("zombie")
                        || lower.contains("skeleton")
                        || lower.contains("monster")
                        || lower.contains("creeper")
                    {
                        "monster"
                    } else {
                        "creature"
                    };
                    let arr = cmap.entry(cat).or_insert_with(|| json!([]));
                    if let Some(list) = arr.as_array_mut() {
                        let weight = sp.get("weight").and_then(Value::as_u64).unwrap_or(1);
                        let min_count = sp
                            .get("minCount")
                            .or_else(|| sp.get("min_count"))
                            .and_then(Value::as_u64)
                            .unwrap_or(1);
                        let max_count = sp
                            .get("maxCount")
                            .or_else(|| sp.get("max_count"))
                            .and_then(Value::as_u64)
                            .unwrap_or(1);
                        list.push(json!({
                            "type": ent_type,
                            "weight": weight,
                            "min_count": min_count,
                            "max_count": max_count
                        }));
                    }
                }
            }
            overlay.write(&rel, &serde_json::to_vec(&bdoc)?)?;
        }
    }
    Ok(())
}

/// The key of a mod's structure set or template pool file that adds its
/// entries to other sets or pools (vanilla's, another mod's) instead of
/// being one of its own. Vanilla data never has it.
const ADD_TO: &str = "horizon:add_to";
/// The bundles a mod may add to, and the list each entry goes into.
const ADDABLE: [(&str, &str); 2] = [
    ("worldgen/structure_sets.json", "structures"),
    ("worldgen/template_pools.json", "elements"),
];

/// A mod file's additions (`horizon:add_to`): its entries appended to the
/// list of each set or pool it names, as lithostitched's
/// `add_structure_set_entries` and `add_template_pool_elements` do for
/// NeoForge and Fabric packs.
struct Addition {
    targets: Vec<String>,
    list: &'static str,
    entries: Vec<J>,
}
impl Addition {
    /// What a mod file of `bundle` adds, if it is an addition file.
    fn read(bundle: &str, text: &str) -> Result<Option<Self>> {
        let Some((_, list)) = ADDABLE.iter().find(|(b, _)| *b == bundle) else {
            return Ok(None);
        };
        if !text.contains(ADD_TO) {
            return Ok(None);
        }
        let doc: J = serde_json::from_str(text)?;
        let Some(targets) = doc.get(ADD_TO) else {
            return Ok(None);
        };
        let targets = match targets {
            J::Str(s) => vec![s.clone()],
            J::Arr(a) => a
                .iter()
                .map(|v| match v {
                    J::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Option<_>>()
                .ok_or(format!("{ADD_TO}: not a list of ids"))?,
            _ => return Err(format!("{ADD_TO}: not an id or a list of ids").into()),
        };
        let Some(J::Arr(entries)) = doc.get(list) else {
            return Err(format!("{ADD_TO} without a {list} list").into());
        };
        Ok(Some(Self {
            targets,
            list,
            entries: entries.clone(),
        }))
    }
    /// Appends the entries to each target in the bundle `doc` (id → the
    /// file's text). A target nobody defines is skipped, as a tag entry
    /// naming an unknown id is.
    fn apply(&self, doc: &mut J) -> Result<()> {
        for target in &self.targets {
            let Some(J::Str(text)) = doc.get(target) else {
                continue;
            };
            let mut v: J = serde_json::from_str(text).map_err(|e| format!("{target}: {e}"))?;
            let Some(J::Arr(list)) = v.get_mut(self.list) else {
                continue;
            };
            list.extend(self.entries.iter().cloned());
            doc.set(target, J::Str(serde_json::to_string(&v)?));
        }
        Ok(())
    }
}

/// The overlay being written: `out` mirrors `base` through links, and a
/// directory becomes real only when something inside it is written.
struct Overlay<'a> {
    base: &'a Path,
    out: &'a Path,
}
impl Overlay<'_> {
    fn new<'a>(base: &'a Path, out: &'a Path) -> Result<Overlay<'a>> {
        if fs::symlink_metadata(out).is_ok() {
            fs::remove_dir_all(out)?;
        }
        fs::create_dir_all(out)?;
        let overlay = Overlay { base, out };
        overlay.link_children(Path::new(""))?;
        Ok(overlay)
    }
    fn link_children(&self, rel: &Path) -> Result<()> {
        let Ok(read) = fs::read_dir(self.base.join(rel)) else {
            return Ok(());
        };
        for entry in read.flatten() {
            let rel = rel.join(entry.file_name());
            link(&self.base.join(&rel), &self.out.join(&rel))?;
        }
        Ok(())
    }
    /// Makes `rel`'s directories real, each holding links to the base's.
    fn dir(&self, rel: &Path) -> Result<()> {
        let mut at = PathBuf::new();
        for part in rel.iter() {
            at.push(part);
            let path = self.out.join(&at);
            let meta = fs::symlink_metadata(&path);
            if meta
                .as_ref()
                .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
            {
                continue;
            }
            if meta.is_ok() {
                fs::remove_file(&path)?;
            }
            fs::create_dir(&path)?;
            self.link_children(&at)?;
        }
        Ok(())
    }
    fn write(&self, rel: &str, bytes: &[u8]) -> Result<()> {
        let mut file = self.create(rel)?;
        file.write_all(bytes)?;
        Ok(())
    }

    /// Replace a linked base file and stream a generated table into the overlay.
    fn create(&self, rel: &str) -> Result<fs::File> {
        let rel = Path::new(rel);
        if let Some(parent) = rel.parent() {
            self.dir(parent)?;
        }
        let path = self.out.join(rel);
        if fs::symlink_metadata(&path).is_ok() {
            fs::remove_file(&path)?;
        }
        Ok(fs::File::create(path)?)
    }

    /// Reuse an installed directory mod's asset inode when possible. Carries
    /// live outside the cache, so copying them into every content root wastes
    /// space; cross-filesystem sources fall back to a regular copy.
    fn link_or_copy(&self, rel: &str, source: &Path) -> Result<()> {
        let rel = Path::new(rel);
        if let Some(parent) = rel.parent() {
            self.dir(parent)?;
        }
        let path = self.out.join(rel);
        if fs::symlink_metadata(&path).is_ok() {
            fs::remove_file(&path)?;
        }
        if fs::hard_link(source, &path).is_err() {
            fs::copy(source, path)?;
        }
        Ok(())
    }
}
#[cfg(unix)]
fn link(target: &Path, at: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}
#[cfg(not(unix))]
fn link(target: &Path, at: &Path) -> std::io::Result<()> {
    if target.is_dir() {
        fs::create_dir_all(at)?;
        for entry in fs::read_dir(target)?.flatten() {
            link(&entry.path(), &at.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(target, at).map(|_| ())
    }
}

/// A tag file's JSON text with `more` appended to its values (no repeats).
fn merge_tag(text: Option<&str>, more: &[J], what: &str) -> Result<String> {
    let mut doc = match text {
        Some(t) => serde_json::from_str(t).map_err(|e| format!("{what}: {e}"))?,
        None => J::Obj(vec![("values".into(), J::Arr(Vec::new()))]),
    };
    let Some(J::Arr(values)) = doc.get_mut("values") else {
        return Err(format!("{what}: no values").into());
    };
    for v in more {
        if !values.contains(v) {
            values.push(v.clone());
        }
    }
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// The block and item ids and tags a content root has, for tag entries.
struct Known {
    base: PathBuf,
    blocks: std::collections::BTreeSet<String>,
    block_tags: std::collections::BTreeSet<String>,
    items: std::collections::BTreeSet<String>,
    /// The worldgen loader's biomes and biome tags.
    biomes: std::collections::BTreeSet<String>,
    biome_tags: std::collections::BTreeSet<String>,
    /// `data/<ns>/tags/<kind>/<path>.json` files the mods add.
    tag_files: std::collections::BTreeSet<String>,
}
impl Known {
    fn new(base: &Path, plan: &Plan, tags: &BTreeMap<String, TagMerge>) -> Result<Self> {
        let mut blocks: std::collections::BTreeSet<String> =
            plan.defaults.keys().cloned().collect();
        blocks.extend(plan.blocks.iter().map(|b| b.decl.id.clone()));
        let mut items = std::collections::BTreeSet::new();
        if let J::Obj(o) = read_json(&base.join("items.json"))? {
            items.extend(o.into_iter().map(|(k, _)| k));
        }
        items.extend(plan.items.iter().map(|i| i.rename.to.clone()));
        let keys = |name: &str| -> Result<std::collections::BTreeSet<String>> {
            Ok(match base.join(name).exists() {
                true => match read_json(&base.join(name))? {
                    J::Obj(o) => o.into_iter().map(|(k, _)| k).collect(),
                    _ => Default::default(),
                },
                false => Default::default(),
            })
        };
        Ok(Known {
            base: base.to_owned(),
            blocks,
            block_tags: keys("block-tags.json")?,
            items,
            biomes: keys("worldgen/biomes.json")?,
            biome_tags: keys(BIOME_TAGS)?,
            tag_files: tags.keys().cloned().collect(),
        })
    }
    /// Whether tag `file` keeps entry `v`: a known id or an existing tag,
    /// for block, item and biome tags (other registries are kept as they
    /// are). The worldgen loader refuses a biome tag naming what it lacks
    /// (another mod's biomes, a loader's tags), so those entries go.
    fn keeps(&self, file: &str, v: &J) -> bool {
        let id = match v {
            J::Str(s) => s.as_str(),
            J::Obj(_) => match v.get("id") {
                Some(J::Str(s)) => s.as_str(),
                _ => return true,
            },
            _ => return true,
        };
        let Some(rest) = file
            .strip_prefix("data/")
            .and_then(|r| r.split_once("/tags/"))
            .map(|(_, rest)| rest)
        else {
            return true;
        };
        let kind = match rest.strip_prefix("worldgen/biome/") {
            Some(_) => "worldgen/biome",
            None => rest.split_once('/').map_or("", |(k, _)| k),
        };
        let ids = match kind {
            "block" => &self.blocks,
            "item" => &self.items,
            "worldgen/biome" => &self.biomes,
            _ => return true,
        };
        match id.strip_prefix('#') {
            Some(tag) => {
                let (tns, path) = tag.split_once(':').unwrap_or(("minecraft", tag));
                let rel = format!("data/{tns}/tags/{kind}/{path}.json");
                self.tag_files.contains(&rel)
                    || match kind {
                        "block" => self.block_tags.contains(&format!("{tns}:{path}")),
                        "worldgen/biome" => self.biome_tags.contains(&format!("{tns}:{path}")),
                        _ => self.base.join(&rel).exists(),
                    }
            }
            None => ids.contains(id),
        }
    }
}

/// What the mods do to one tag file, in load order (a data pack's
/// TagLoader): each adds its values, and one that replaces the tag first
/// drops vanilla's entries and the mods' before it.
#[derive(Default)]
struct TagMerge {
    /// A mod replaced the tag: vanilla's entries are left out.
    replaced: bool,
    /// The mods whose values are in `values`.
    mods: Vec<String>,
    values: Vec<J>,
}

impl TagMerge {
    /// Adds one mod's tag file. A replace that drops entries (vanilla's in a
    /// `shared` tag, one not in the mod's namespaces, or other mods') warns.
    fn add(
        &mut self,
        id: &str,
        file: &str,
        replace: bool,
        shared: bool,
        values: &[J],
    ) -> Option<String> {
        let mut warning = None;
        if replace {
            let mut dropped: Vec<String> = Vec::new();
            if shared && !self.replaced {
                dropped.push("vanilla's entries".into());
            }
            if !self.mods.is_empty() {
                dropped.push(format!("those of {}", self.mods.join(", ")));
            }
            if !dropped.is_empty() {
                warning = Some(format!(
                    "mod {id} replaces {file}: {} are left out, as a data pack's replace \
                     does; the mods after it in load order still add to it",
                    dropped.join(" and ")
                ));
            }
            self.replaced = true;
            self.mods.clear();
            self.values.clear();
        }
        if !self.mods.iter().any(|m| m == id) {
            self.mods.push(id.to_owned());
        }
        self.values.extend(values.iter().cloned());
        warning
    }
}

/// Threads rewriting the tables at once. Carried mod blocks expand
/// gameplay-facts.json substantially: ATM10 adds over 678,000 states, so
/// keeping more than one expanded table in memory can exhaust the launcher.
const TABLE_THREADS: usize = 1;

/// Rewrites the base's per-state tables for the mods' states into the
/// overlay. They are independent: [`TABLE_THREADS`] threads take them,
/// the largest first.
fn tables(base: &Path, plan: &Plan, overlay: &Overlay<'_>) -> Result<()> {
    let mut jobs: Vec<(&str, u64)> = JSON_TABLES
        .iter()
        .chain(&NBT_TABLES)
        .filter_map(|name| Some((*name, fs::metadata(base.join(name)).ok()?.len())))
        .collect();
    jobs.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    // Their directories become real here, so the threads only write files.
    for (name, _) in &jobs {
        if let Some(parent) = Path::new(name).parent() {
            overlay.dir(parent)?;
        }
    }
    let table = |name: &str| -> Result<()> {
        let path = base.join(name);
        if JSON_TABLES.contains(&name) {
            let mut doc = read_json(&path)?;
            plan.json(name, &mut doc)
                .map_err(|e| format!("{name}: {e}"))?;
            let mut writer = BufWriter::new(overlay.create(name)?);
            serde_json::to_writer(&mut writer, &doc)?;
            writer.flush()?;
        } else {
            let (mut named, _) = nbt::read_named(&fs::read(&path)?, Limits::tables())
                .map_err(|e| format!("{name}: {e}"))?;
            plan.nbt(&mut named.tag);
            let bytes =
                nbt::write_named(&named, Limits::tables()).map_err(|e| format!("{name}: {e}"))?;
            overlay.write(name, &bytes)?;
        }
        Ok(())
    };
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads.min(TABLE_THREADS))
            .map(|_| {
                scope.spawn(|| -> std::result::Result<(), String> {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((name, _)) = jobs.get(i) else {
                            return Ok(());
                        };
                        table(name).map_err(|e| e.to_string())?;
                    }
                })
            })
            .collect();
        workers.into_iter().try_for_each(|w| {
            w.join()
                .unwrap_or_else(|_| Err("a table thread panicked".into()))
        })
    })?;
    Ok(())
}

/// Builds the content root for `mods` over `base` in `out`. Returns the
/// warnings for the log: the shared tags a mod replaced.
pub fn build(base: &Path, mods: &[ContentMod], out: &Path) -> Result<Vec<String>> {
    // Two mods owning one namespace would mix their files.
    for (i, a) in mods.iter().enumerate() {
        for b in &mods[..i] {
            let (b, a) = (
                (b.id.as_str(), b.content.namespaces.as_slice()),
                (a.id.as_str(), a.content.namespaces.as_slice()),
            );
            if let Some(e) = horizon_mods::content::clash(b, a) {
                return Err(e.into());
            }
        }
    }
    // The overlay links to the base: a relative base would leave them dangling.
    let base = &std::path::absolute(base)?;
    let plan = Plan::new(base, mods)?;
    let overlay = Overlay::new(base, out)?;
    tables(base, &plan, &overlay)?;
    // Mod files, and every tag addition gathered per tag file.
    let mut tags: BTreeMap<String, TagMerge> = BTreeMap::new();
    let mut shared_registry_files: BTreeMap<String, String> = BTreeMap::new();
    let mut warnings = plan.warnings.clone();
    let mut worldgen: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    let mut additions: BTreeMap<&str, Vec<Addition>> = BTreeMap::new();
    let mut biome_modifiers: Vec<String> = Vec::new();
    for m in mods {
        let declarations = format!("data/{}/horizon/", m.id);
        let mut reader = m.content.reader()?;
        for file in &m.content.files {
            let is_tag = file.starts_with("data/") && file.contains("/tags/");
            if is_tag && file.ends_with(".json") {
                let bytes = reader.read(file)?;
                let doc: J = serde_json::from_slice(&bytes).map_err(|e| format!("{file}: {e}"))?;
                let Some(J::Arr(values)) = doc.get("values") else {
                    return Err(format!("{}: {file}: no values", m.id).into());
                };
                let replace = doc.get("replace") == Some(&J::Bool(true));
                let ns = file.split('/').nth(1).unwrap_or_default();
                let shared = !m.content.namespaces.iter().any(|n| n == ns);
                let merge = tags.entry(file.clone()).or_default();
                warnings.extend(merge.add(&m.id, file, replace, shared, values));
            } else if !file.starts_with(&declarations) {
                let source = m.content.source();
                let directory = source.is_dir();
                let worldgen_file = worldgen_entry(file);
                let biome_modifier = is_biome_modifier_file(file);
                let structure = file
                    .strip_prefix("data/")
                    .and_then(|r| {
                        r.split_once("/structure/")
                            .or_else(|| r.split_once("/structures/"))
                    })
                    .filter(|(_, path)| path.ends_with(".nbt"))
                    .map(|(ns, path)| format!("worldgen/structure/{ns}/{path}"));
                // Carried directories already contain ordinary resources as
                // files. Reusing their inodes avoids reading and copying the
                // much larger data trees into the content root on every build.
                if directory {
                    overlay.link_or_copy(file, &source.join(file))?;
                    if let Some(target) = &structure {
                        overlay.link_or_copy(target, &source.join(file))?;
                    }
                }
                if horizon_mods::content::minecraft_registry_addition(file) {
                    if let Some(previous) = shared_registry_files.get(file) {
                        warnings.push(format!(
                            "mods {previous} and {} both add {file}; {} wins by load order",
                            m.id, m.id
                        ));
                    } else if base.join(file).is_file() {
                        warnings.push(format!("mod {} replaces vanilla {file}", m.id));
                    }
                    shared_registry_files.insert(file.clone(), m.id.clone());
                }
                if let Some((bundle, id)) = worldgen_file {
                    let bytes = reader.read(file)?;
                    if !directory {
                        overlay.write(file, &bytes)?;
                    }
                    let text = String::from_utf8(bytes).map_err(|e| format!("{file}: {e}"))?;
                    let text = normalize_worldgen_entry(bundle, &text);
                    if bundle == "worldgen/biomes.json" {
                        // Biomes::load reads the source biome file as well
                        // as the registry bundle; keep both representations
                        // on the same migrated 26.3 data.
                        overlay.write(file, text.as_bytes())?;
                    }
                    let entries = worldgen.entry(bundle).or_default();
                    match Addition::read(bundle, &text).map_err(|e| format!("{file}: {e}"))? {
                        Some(addition) => additions.entry(bundle).or_default().push(addition),
                        None => entries.push((id, text)),
                    }
                } else if biome_modifier {
                    let bytes = reader.read(file)?;
                    if !directory {
                        overlay.write(file, &bytes)?;
                    }
                    if let Ok(text) = String::from_utf8(bytes) {
                        biome_modifiers.push(text);
                    }
                } else if !directory {
                    let bytes = reader.read(file)?;
                    overlay.write(file, &bytes)?;
                    if let Some(target) = structure {
                        overlay.write(&target, &bytes)?;
                    }
                }
            }
        }
    }
    // A tag naming an id nobody registers fails as a whole in vanilla, and
    // carried-over mods name blocks left out or other mods' ids: such
    // entries are dropped instead, so the rest of the tag holds.
    let known = Known::new(base, &plan, &tags)?;
    for (file, merge) in &mut tags {
        merge.values.retain(|v| known.keeps(file, v));
    }
    let mut bundles: Vec<(&str, J)> = Vec::new();
    for name in TAG_BUNDLES {
        if base.join(name).exists() {
            bundles.push((name, read_json(&base.join(name))?));
        }
    }
    for (file, merge) in &tags {
        let vanilla = fs::read_to_string(base.join(file))
            .ok()
            .filter(|_| !merge.replaced);
        overlay.write(
            file,
            merge_tag(vanilla.as_deref(), &merge.values, file)?.as_bytes(),
        )?;
        // data/<ns>/tags/block/<path>.json is the block tag <ns>:<path>.
        let Some((ns, path)) = file
            .strip_prefix("data/")
            .and_then(|r| r.split_once("/tags/block/"))
            .and_then(|(ns, p)| Some((ns, p.strip_suffix(".json")?)))
        else {
            continue;
        };
        let id = format!("{ns}:{path}");
        for (name, doc) in &mut bundles {
            let text = match doc.get(&id) {
                Some(J::Str(t)) if !merge.replaced => Some(t.clone()),
                _ => None,
            };
            let merged = merge_tag(text.as_deref(), &merge.values, name)?;
            doc.set(&id, J::Str(merged));
        }
    }
    for (name, doc) in &bundles {
        overlay.write(name, &serde_json::to_vec(doc)?)?;
    }
    // Biome tags, merged into the worldgen loader's bundle as block tags are.
    let biome_tags: Vec<(String, &TagMerge)> = tags
        .iter()
        .filter_map(|(file, merge)| {
            let (ns, path) = file
                .strip_prefix("data/")?
                .split_once("/tags/worldgen/biome/")?;
            Some((format!("{ns}:{}", path.strip_suffix(".json")?), merge))
        })
        .collect();
    if !biome_tags.is_empty() && base.join(BIOME_TAGS).exists() {
        let mut doc = read_json(&base.join(BIOME_TAGS))?;
        for (id, merge) in biome_tags {
            let text = match doc.get(&id) {
                Some(J::Str(t)) if !merge.replaced => Some(t.clone()),
                _ => None,
            };
            let merged = merge_tag(text.as_deref(), &merge.values, BIOME_TAGS)?;
            doc.set(&id, J::Str(merged));
        }
        overlay.write(BIOME_TAGS, &serde_json::to_vec(&doc)?)?;
    }
    if let Some(carvers) = worldgen.get_mut("worldgen/carvers.json") {
        let vanilla: BTreeMap<String, String> =
            serde_json::from_slice(&fs::read(base.join("worldgen/carvers.json"))?)?;
        for (_, text) in carvers {
            let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else {
                continue;
            };
            let defaults = match value.get("type").and_then(serde_json::Value::as_str) {
                Some("minecraft:cave") => vanilla.get("minecraft:cave"),
                Some("minecraft:canyon") => vanilla.get("minecraft:canyon"),
                _ => None,
            };
            let (Some(defaults), Some(target)) = (defaults, value.as_object_mut()) else {
                continue;
            };
            let defaults: serde_json::Value = serde_json::from_str(defaults)?;
            if let Some(defaults) = defaults.as_object() {
                for (key, default) in defaults {
                    if key == "shape"
                        && let (Some(target_shape), Some(default_shape)) = (
                            target
                                .get_mut(key)
                                .and_then(serde_json::Value::as_object_mut),
                            default.as_object(),
                        )
                    {
                        for (shape_key, shape_default) in default_shape {
                            target_shape
                                .entry(shape_key.clone())
                                .or_insert_with(|| shape_default.clone());
                        }
                        continue;
                    }
                    target.entry(key.clone()).or_insert_with(|| default.clone());
                }
            }
            *text = serde_json::to_string(&value)?;
        }
    }
    // Structures and the pools, sets, processors and features they use, then
    // what mods add to sets and pools, in mod and file order.
    let skipped_carvers: BTreeMap<String, String> = worldgen
        .get("worldgen/carvers.json")
        .into_iter()
        .flatten()
        .filter_map(|(id, text)| {
            let error = horizon_world::worldgen::carver::WorldCarver::from_json(text).err()?;
            (!id.starts_with("minecraft:")).then(|| (id.clone(), error.to_string()))
        })
        .collect();
    let mut skipped_densities: BTreeMap<String, String> = worldgen
        .get("worldgen/densities.json")
        .into_iter()
        .flatten()
        .filter_map(|(id, text)| {
            let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
            custom_density_type(&value).map(|kind| (id.clone(), kind))
        })
        .collect();
    loop {
        let missing: std::collections::BTreeSet<String> =
            skipped_densities.keys().cloned().collect();
        let dependent: Vec<String> = worldgen
            .get("worldgen/densities.json")
            .into_iter()
            .flatten()
            .filter(|(id, _)| !skipped_densities.contains_key(id))
            .filter_map(|(id, text)| {
                let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
                references_density(&value, &missing, None).then(|| id.clone())
            })
            .collect();
        if dependent.is_empty() {
            break;
        }
        for id in dependent {
            skipped_densities.insert(id, "depends on an unsupported density function".into());
        }
    }
    for (id, error) in &skipped_carvers {
        warnings.push(format!(
            "worldgen carver {id} was skipped because Horizon cannot load it: {error}"
        ));
    }
    for (id, reason) in &skipped_densities {
        warnings.push(format!("worldgen density {id} was skipped: {reason}"));
    }
    for (bundle, entries) in &worldgen {
        if !base.join(bundle).exists() {
            continue;
        }
        let mut doc = read_json(&base.join(bundle))?;
        for (id, text) in entries {
            if *bundle == "worldgen/carvers.json" && skipped_carvers.contains_key(id) {
                continue;
            }
            if *bundle == "worldgen/densities.json" && skipped_densities.contains_key(id) {
                continue;
            }
            doc.set(id, J::Str(text.clone()));
        }
        for addition in additions.get(bundle).into_iter().flatten() {
            addition
                .apply(&mut doc)
                .map_err(|e| format!("{bundle}: {e}"))?;
        }
        if *bundle == "worldgen/biomes.json" && !skipped_carvers.is_empty() {
            if let J::Obj(biomes) = &mut doc {
                for (_, raw) in biomes {
                    let J::Str(text) = raw else { continue };
                    let Ok(mut biome) = serde_json::from_str::<serde_json::Value>(text) else {
                        continue;
                    };
                    let Some(carvers) = biome.get_mut("carvers").and_then(|v| v.as_array_mut())
                    else {
                        continue;
                    };
                    let before = carvers.len();
                    carvers.retain(|entry| {
                        !entry
                            .as_str()
                            .is_some_and(|id| skipped_carvers.contains_key(id))
                    });
                    if carvers.len() != before {
                        *text = serde_json::to_string(&biome)?;
                    }
                }
            }
        }
        overlay.write(bundle, &serde_json::to_vec(&doc)?)?;
    }
    // Density references are resolved when a terrain generator is built. Do
    // the same registry compilation here so an unsupported or broken mod
    // entry is reported while building the content root, before a world
    // starts generation.
    if worldgen.contains_key("worldgen/noises.json")
        || worldgen.contains_key("worldgen/densities.json")
    {
        let bundle = |name: &str| -> Result<BTreeMap<String, String>> {
            let overlaid = overlay.out.join(name);
            let path = if overlaid.exists() {
                overlaid
            } else {
                base.join(name)
            };
            Ok(serde_json::from_slice(&fs::read(path)?)?)
        };
        let noises = bundle("worldgen/noises.json")?;
        let densities = bundle("worldgen/densities.json")?;
        horizon_world::worldgen::density::DensityRegistry::from_json(&noises, &densities).map_err(
            |e| std::io::Error::other(format!("invalid mod worldgen noise/density registry: {e}")),
        )?;
    }
    if worldgen.contains_key("worldgen/carvers.json") {
        let overlaid = overlay.out.join("worldgen/carvers.json");
        let path = if overlaid.exists() {
            overlaid
        } else {
            base.join("worldgen/carvers.json")
        };
        let carvers: BTreeMap<String, String> = serde_json::from_slice(&fs::read(path)?)?;
        horizon_world::worldgen::carver::CarverCatalog::from_json(&carvers).map_err(|e| {
            std::io::Error::other(format!("invalid mod worldgen carver registry: {e}"))
        })?;
    }
    apply_biome_modifiers(base, &overlay, &biome_modifiers)?;
    if worldgen.contains_key("worldgen/biomes.json") {
        let bundle = |name: &str| -> Result<BTreeMap<String, String>> {
            let overlaid = overlay.out.join(name);
            let path = if overlaid.exists() {
                overlaid
            } else {
                base.join(name)
            };
            Ok(serde_json::from_slice(&fs::read(path)?)?)
        };
        let biomes = bundle("worldgen/biomes.json")?;
        let carvers = bundle("worldgen/carvers.json")?;
        let carvers = horizon_world::worldgen::carver::CarverCatalog::from_json(&carvers)
            .map_err(|e| std::io::Error::other(format!("invalid mod worldgen carvers: {e}")))?;
        for (id, text) in biomes {
            horizon_world::worldgen::biome_temperature::BiomeTemperatureSettings::from_json(&text)
                .map_err(|e| std::io::Error::other(format!("invalid worldgen biome {id}: {e}")))?;
            let value: serde_json::Value = serde_json::from_str(&text)?;
            carvers
                .decode_set(&value["carvers"].to_string())
                .map_err(|e| {
                    std::io::Error::other(format!("invalid carvers for worldgen biome {id}: {e}"))
                })?;
        }
    }
    // A block without its own blockstate looks like its template, so every
    // state still has a model.
    for m in mods {
        for block in &m.content.blocks {
            let (ns, path) = block.id.split_once(':').unwrap_or((&m.id, &block.id));
            let own = format!("assets/{ns}/blockstates/{path}.json");
            let template = block.template.trim_start_matches("minecraft:");
            if !m.content.files.contains(&own) {
                let vanilla = base.join(format!("assets/minecraft/blockstates/{template}.json"));
                overlay.write(&own, &fs::read(&vanilla)?)?;
            }
            let item_def = format!("assets/{ns}/items/{path}.json");
            if block.item && !m.content.files.contains(&item_def) {
                let block_model = format!("assets/{ns}/models/block/{path}.json");
                let item_model = format!("assets/{ns}/models/item/{path}.json");
                if m.content.files.contains(&item_model) {
                    let body = format!(
                        r#"{{"model":{{"type":"minecraft:model","model":"{ns}:item/{path}"}}}}"#
                    );
                    overlay.write(&item_def, body.as_bytes())?;
                } else if m.content.files.contains(&block_model) {
                    let body = format!(
                        r#"{{"model":{{"type":"minecraft:model","model":"{ns}:block/{path}"}}}}"#
                    );
                    overlay.write(&item_def, body.as_bytes())?;
                } else {
                    let vanilla = base.join(format!("assets/minecraft/items/{template}.json"));
                    if let Ok(bytes) = fs::read(&vanilla) {
                        overlay.write(&item_def, &bytes)?;
                    }
                }
            }
        }
        for item in &m.content.items {
            let (ns, path) = item.id.split_once(':').unwrap_or((&m.id, &item.id));
            let item_def = format!("assets/{ns}/items/{path}.json");
            if !m.content.files.contains(&item_def) {
                let item_model = format!("assets/{ns}/models/item/{path}.json");
                if m.content.files.contains(&item_model) {
                    let body = format!(
                        r#"{{"model":{{"type":"minecraft:model","model":"{ns}:item/{path}"}}}}"#
                    );
                    overlay.write(&item_def, body.as_bytes())?;
                } else {
                    let template = item.template.trim_start_matches("minecraft:");
                    let vanilla = base.join(format!("assets/minecraft/items/{template}.json"));
                    if let Ok(bytes) = fs::read(&vanilla) {
                        overlay.write(&item_def, &bytes)?;
                    }
                }
            }
        }
    }
    let mut namespaces: Vec<&str> = mods
        .iter()
        .flat_map(|m| m.content.namespaces.iter().map(String::as_str))
        .collect();
    namespaces.sort();
    overlay.write(MARKER, &serde_json::to_vec(&namespaces)?)?;
    overlay.write(WARNINGS, warnings.join("\n").as_bytes())?;
    Ok(warnings)
}

/// The root to load: `base` itself with no content, else the mods' overlay
/// over it, built in `cache/mod-content`.
///
/// The root is built once and kept: later starts with the same build, base
/// and mod files reuse it. With ATM10's carried mods a build takes about
/// 25 s and peaks above 3 GB, more than everything the game keeps.
pub fn resource_root(base: &Path, mods: &[ContentMod], cache: &Path) -> Result<PathBuf> {
    if mods.iter().all(|m| m.content.is_empty()) {
        return Ok(base.to_owned());
    }
    let out = cache.join("mod-content");
    let stamp = stamp(base, mods)?;
    let keep = out.join("horizon-mod-content-keep");
    if keep.exists() && out.join(MARKER).exists() {
        let _ = fs::remove_file(&keep);
        let _ = fs::write(out.join(STAMP), &stamp);
    }
    if fs::read(out.join(STAMP)).is_ok_and(|s| s == stamp.as_bytes()) {
        // A reused root warns as the build did.
        let warnings = fs::read_to_string(out.join(WARNINGS)).unwrap_or_default();
        warnings.lines().for_each(|w| log::warn!("Horizon: {w}"));
        return Ok(out);
    }
    for warning in build(base, mods, &out)? {
        log::warn!("Horizon: {warning}");
    }
    fs::write(out.join(STAMP), stamp)?;
    Ok(out)
}

/// In a root built by [`build`]: its warnings, one per line.
const WARNINGS: &str = "horizon-mod-content-warnings.txt";

/// In a root built by [`resource_root`]: what it was built from.
const STAMP: &str = "horizon-mod-content.stamp";

/// What a content root is built from: the content transform schema, the
/// base's manifest and tables [`build`] rewrites, and every file of every mod
/// in order. A renderer or gameplay executable rebuild does not change the
/// generated content. Bump this schema when [`build`] changes its output.
fn stamp(base: &Path, mods: &[ContentMod]) -> Result<String> {
    use sha2::{Digest, Sha256};
    let meta = |path: &Path| -> Result<String> {
        let m = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let modified = m
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Ok(format!(
            "{} {} {}",
            path.display(),
            m.len(),
            modified.as_nanos()
        ))
    };
    let mut parts = vec!["horizon mod content 4".to_owned()];
    let base = std::path::absolute(base)?;
    parts.push(
        String::from_utf8_lossy(&fs::read(base.join("horizon-resources.json"))?).into_owned(),
    );
    for name in JSON_TABLES.iter().chain(&NBT_TABLES).chain(&TAG_BUNDLES) {
        if base.join(name).exists() {
            parts.push(meta(&base.join(name))?);
        }
    }
    for m in mods {
        parts.push(m.id.clone());
        let source = m.content.source();
        if source.is_dir() {
            for file in &m.content.files {
                parts.push(meta(&source.join(file))?);
            }
        } else {
            parts.push(meta(source)?);
        }
    }
    let mut hasher = Sha256::new();
    for part in &parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
#[path = "mod_content_tests.rs"]
mod tests;

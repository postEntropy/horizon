//! FlatLevelSource: superflat worlds (`minecraft:flat` generator).
use crate::{chunk::Chunk, streaming::ChunkGenerator};
use horizon_world::{nbt::Tag, registry::BlockRegistry};

/// The layers of a flat generator, bottom up from the world's minimum Y.
pub struct FlatTerrain {
    layers: Vec<u32>,
    biome: u16,
    surface: Option<crate::lod::Surface>,
    platform: Option<u32>,
}
impl FlatTerrain {
    /// Loads a data-pack flat dimension's explicit biome and bottom-up layers.
    /// Returns `None` when no definition exists; invalid definitions are errors.
    pub fn from_dimension_file(
        root: &std::path::Path,
        dimension: &str,
        min_y: i32,
        height: i32,
        registry: &BlockRegistry,
        biomes: &crate::biome::Biomes,
    ) -> Result<Option<Self>, String> {
        Self::from_dimension_file_with_biomes(
            root,
            dimension,
            min_y,
            height,
            registry,
            &biomes.index,
        )
    }

    fn from_dimension_file_with_biomes(
        root: &std::path::Path,
        dimension: &str,
        min_y: i32,
        height: i32,
        registry: &BlockRegistry,
        biomes: &std::collections::HashMap<String, u16>,
    ) -> Result<Option<Self>, String> {
        let (namespace, path) = dimension.split_once(':').ok_or("invalid dimension id")?;
        if namespace.is_empty()
            || namespace == "."
            || namespace == ".."
            || path.is_empty()
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
            || path.split('/').any(|part| {
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || !part.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b)
                    })
            })
        {
            return Err(format!("invalid dimension id {dimension}"));
        }
        let path = root
            .join("data")
            .join(namespace)
            .join("dimension")
            .join(format!("{path}.json"));
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let doc: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let generator = &doc["generator"];
        if generator["type"].as_str() != Some("minecraft:flat") {
            return Err(format!(
                "{}: expected minecraft:flat generator",
                path.display()
            ));
        }
        let settings = &generator["settings"];
        if settings["features"].as_bool().unwrap_or(false)
            || settings["lakes"].as_bool().unwrap_or(false)
        {
            return Err("flat dimension features and lakes are not yet supported".into());
        }
        let biome_name = settings["biome"]
            .as_str()
            .ok_or("flat dimension has no biome")?;
        let biome = *biomes
            .get(biome_name)
            .ok_or_else(|| format!("unknown biome {biome_name}"))?;
        let layers_json = settings["layers"]
            .as_array()
            .ok_or("flat dimension has no layers")?;
        if !(crate::chunk::MIN_Y..crate::chunk::MIN_Y + crate::chunk::HEIGHT).contains(&min_y)
            || height <= 0
        {
            return Err("dimension starts outside supported height".into());
        }
        let mut layers = vec![0; (min_y - crate::chunk::MIN_Y) as usize];
        let mut count = 0_i32;
        for layer in layers_json {
            let name = layer["block"].as_str().ok_or("flat layer has no block")?;
            let state = registry
                .default_state(name)
                .ok_or_else(|| format!("unknown flat layer block {name}"))?
                .0;
            let thickness = layer["height"]
                .as_u64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or("invalid flat layer height")?;
            count = count
                .checked_add(thickness)
                .ok_or("flat layers exceed dimension height")?;
            if count > height
                || min_y
                    .checked_add(count)
                    .is_none_or(|top| top > crate::chunk::MIN_Y + crate::chunk::HEIGHT)
            {
                return Err("flat layers exceed dimension height".into());
            }
            layers.extend(std::iter::repeat_n(state, thickness as usize));
        }
        let mut water = None;
        let mut surface = None;
        for (i, &id) in layers.iter().enumerate().rev() {
            let state = horizon_world::registry::BlockStateId(id);
            let Some(block) = registry.state(state) else {
                continue;
            };
            if matches!(
                block.name.as_str(),
                "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
            ) {
                continue;
            }
            let height = crate::chunk::MIN_Y + i as i32 + 1;
            if matches!(block.name.as_str(), "minecraft:water" | "minecraft:lava") {
                water.get_or_insert((height, state));
            } else {
                surface = Some(crate::lod::Surface {
                    height,
                    state,
                    water,
                    biome,
                });
                break;
            }
        }
        Ok(Some(Self {
            layers,
            biome,
            surface,
            platform: None,
        }))
    }
    /// The overworld generator of `world_gen_settings` (`data.dimensions`), if it
    /// is flat. Its `features` and `lakes` decorate the world; a world without
    /// them (the default) gets only its layers, as FlatLevelSource places them.
    pub fn from_settings(
        settings: &Tag,
        registry: &BlockRegistry,
        biomes: &crate::biome::Biomes,
    ) -> Option<Self> {
        let generator = settings
            .get("dimensions")?
            .get("minecraft:overworld")?
            .get("generator")?;
        if crate::save::text(generator.get("type"))?.trim_start_matches("minecraft:") != "flat" {
            return None;
        }
        let s = generator.get("settings")?;
        let biome = crate::save::text(s.get("biome")).unwrap_or_else(|| "minecraft:plains".into());
        let mut layers = Vec::new();
        if let Some(Tag::List(list)) = s.get("layers") {
            for layer in list {
                let block = crate::save::text(layer.get("block"))?;
                let height = crate::save::number(layer.get("height")).unwrap_or(0).max(0);
                let state = registry.default_state(&block)?.0;
                layers.extend(std::iter::repeat_n(state, height as usize));
            }
        }
        let biome = biomes.id(&biome);
        let mut water = None;
        let mut surface = None;
        for (i, &id) in layers
            .iter()
            .take(crate::chunk::HEIGHT as usize)
            .enumerate()
            .rev()
        {
            let state = horizon_world::registry::BlockStateId(id);
            let name = registry.state(state)?.name.as_str();
            if matches!(
                name,
                "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
            ) {
                continue;
            }
            let height = crate::chunk::MIN_Y + i as i32 + 1;
            if matches!(name, "minecraft:water" | "minecraft:lava") {
                water.get_or_insert((height, state));
            } else {
                surface = Some(crate::lod::Surface {
                    height,
                    state,
                    water,
                    biome,
                });
                break;
            }
        }
        Some(Self {
            layers,
            biome,
            surface,
            platform: None,
        })
    }
    /// Flat or void terrain for a custom mod dimension without a noise preset.
    pub fn for_mod_dimension(
        min_y: i32,
        void_world: bool,
        surface_block: &str,
        registry: &BlockRegistry,
        biomes: &crate::biome::Biomes,
    ) -> Self {
        let biome = biomes.id("minecraft:plains");
        let surface_id = if surface_block.trim().is_empty() {
            "minecraft:stone".to_owned()
        } else if surface_block.contains(':') {
            surface_block.to_owned()
        } else {
            format!("minecraft:{surface_block}")
        };
        let surface_state = registry
            .default_state(&surface_id)
            .or_else(|| registry.default_state("minecraft:stone"))
            .map_or(0, |id| id.0);
        if void_world {
            return Self {
                layers: Vec::new(),
                biome,
                surface: None,
                platform: (surface_state != 0).then_some(surface_state),
            };
        }
        let bedrock = registry
            .default_state("minecraft:bedrock")
            .map_or(surface_state, |id| id.0);
        let stone = registry
            .default_state("minecraft:stone")
            .map_or(surface_state, |id| id.0);
        let floor_y = min_y.clamp(crate::chunk::MIN_Y, 60);
        let air_layers = (floor_y - crate::chunk::MIN_Y).max(0) as usize;
        let filler_layers = (63 - floor_y - 1).max(0) as usize;
        let mut layers = Vec::with_capacity(air_layers + 1 + filler_layers + 1);
        layers.extend(std::iter::repeat_n(0, air_layers));
        layers.push(bedrock);
        layers.extend(std::iter::repeat_n(stone, filler_layers));
        layers.push(surface_state);
        let surface = (surface_state != 0).then_some(crate::lod::Surface {
            height: 64,
            state: horizon_world::registry::BlockStateId(surface_state),
            water: None,
            biome,
        });
        Self {
            layers,
            biome,
            surface,
            platform: None,
        }
    }
    /// FlatLevelSource.getSpawnHeight: the top of the layers.
    pub fn spawn_height(&self) -> i32 {
        if self.platform.is_some() && self.layers.is_empty() {
            return 64;
        }
        crate::chunk::MIN_Y + self.layers.len().min(crate::chunk::HEIGHT as usize) as i32
    }
}
/// DimensionType.Y_SIZE: the most layers FlatLevelGeneratorSettings takes.
const Y_SIZE: i64 = 4064;

/// FlatLevelGeneratorSettings.CODEC reading a dedicated server's
/// `generator-settings` (DedicatedServerProperties.WorldDimensionData for
/// the `minecraft:flat` preset): the settings as `world_gen_settings` stores
/// them, and the errors vanilla logs, in order (the constructor's, then
/// `resultOrPartial`'s). No settings means the preset's own layers; a layer
/// that doesn't parse is left out, as the codec's partial result leaves it.
/// `is_block` says whether a block id is registered, `is_biome` a biome id.
pub fn settings_from_json(
    json: &crate::server::json::Json,
    is_block: impl Fn(&str) -> bool,
    is_biome: impl Fn(&str) -> bool,
) -> (Option<Tag>, Vec<String>) {
    use crate::server::json::Json;
    let id = |s: &str| {
        if s.contains(':') {
            s.to_owned()
        } else {
            format!("minecraft:{s}")
        }
    };
    let Some(Json::Array(list)) = json.get("layers") else {
        let error = match json.get("layers") {
            None => format!("No key layers in MapLike[{}]", json.compact()),
            Some(v) => format!("Not a list: {}", v.compact()),
        };
        return (None, vec![error]);
    };
    let mut errors = Vec::new();
    let mut layers = Vec::new();
    let mut total = 0;
    for layer in list {
        let height = layer.get("height").and_then(Json::as_string);
        let height = height.as_deref().and_then(|h| h.parse::<i64>().ok());
        let block = layer.get("block").and_then(Json::as_string).map(|b| id(&b));
        match (height, block) {
            (Some(h), _) if !(0..=Y_SIZE).contains(&h) => {
                errors.push(format!("Value {h} outside of range [0:{Y_SIZE}]"));
            }
            (Some(h), Some(b)) if is_block(&b) => {
                total += h;
                layers.push(Tag::compound([
                    ("height", Tag::Int(h as i32)),
                    ("block", Tag::String(b.as_str().into())),
                ]));
            }
            (Some(_), Some(b)) => errors.push(format!(
                "Unknown registry key in ResourceKey[minecraft:root / minecraft:block]: {b}"
            )),
            (None, _) => errors.push(format!("No key height in MapLike[{}]", layer.compact())),
            (_, None) => errors.push(format!("No key block in MapLike[{}]", layer.compact())),
        }
    }
    // The constructor: Biome.CODEC is lenient, and a biome that is missing
    // or unknown is Plains.
    let biome = json
        .get("biome")
        .and_then(Json::as_string)
        .map(|b| id(&b))
        .filter(|b| is_biome(b));
    let mut logged = Vec::new();
    if biome.is_none() {
        logged.push("Unknown biome, defaulting to plains".to_owned());
    }
    let biome = biome.unwrap_or_else(|| "minecraft:plains".to_owned());
    // validateHeight.
    if total > Y_SIZE {
        errors.push(format!("Sum of layer heights is > {Y_SIZE}"));
        logged.push(errors.join("; "));
        return (None, logged);
    }
    let flag = |key: &str| match json.get(key) {
        Some(Json::Bool(b)) => *b,
        Some(Json::Number(n)) => n.parse::<f64>().is_ok_and(|v| v != 0.0),
        _ => false,
    };
    let mut settings = Vec::new();
    // RegistryCodecs.holderSet, lenient too: a tag or a list of ids.
    match json.get("structure_overrides") {
        Some(Json::String(s)) => {
            settings.push(("structure_overrides", Tag::String(s.as_str().into())))
        }
        Some(Json::Array(ids)) if ids.iter().all(|i| matches!(i, Json::String(_))) => settings
            .push((
                "structure_overrides",
                Tag::List(
                    ids.iter()
                        .filter_map(Json::as_string)
                        .map(|i| Tag::String(id(&i).as_str().into()))
                        .collect(),
                ),
            )),
        _ => {}
    }
    settings.extend([
        ("layers", Tag::List(layers)),
        ("lakes", Tag::Byte(i8::from(flag("lakes")))),
        ("features", Tag::Byte(i8::from(flag("features")))),
        ("biome", Tag::String(biome.as_str().into())),
    ]);
    if !errors.is_empty() {
        logged.push(errors.join("; "));
    }
    (Some(Tag::compound(settings)), logged)
}
impl ChunkGenerator for FlatTerrain {
    type Worker = ();
    fn worker(&self) {}
    fn lod_surface(&self, _: &mut (), _: i32, _: i32) -> Option<crate::lod::Surface> {
        self.surface
    }
    fn proto(&self, _: &mut (), cx: i32, cz: i32) -> crate::streaming::Proto {
        let mut chunk = Chunk::empty(cx, cz);
        for s in &mut chunk.sections {
            std::sync::Arc::make_mut(s).biomes = horizon_core::paletted::PalettedContainer::new(
                crate::chunk::BIOME_STRATEGY,
                u32::from(self.biome),
            );
        }
        for (i, &block) in self.layers.iter().enumerate().take(384) {
            if block == 0 {
                continue;
            }
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set(x, -64 + i as i32, z, block);
                }
            }
        }
        if let Some(block) = self.platform
            && (-1..=0).contains(&cx)
            && (-1..=0).contains(&cz)
        {
            for wx in -2..=2_i32 {
                for wz in -2..=2_i32 {
                    if (wx >> 4) == cx && (wz >> 4) == cz {
                        chunk.set(wx.rem_euclid(16), 63, wz.rem_euclid(16), block);
                    }
                }
            }
        }
        for s in &mut chunk.sections {
            std::sync::Arc::make_mut(s).compact();
        }
        Box::new(chunk)
    }
    /// FlatLevelSource.spawnOriginalMobs is empty: no generation-time spawns.
    fn finish(&self, proto: crate::streaming::Proto) -> Chunk {
        *proto.downcast::<Chunk>().expect("flat terrain proto")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{number, text};
    use crate::server::json::{self, Json};

    fn read(text: &str) -> (Option<Tag>, Vec<String>) {
        let json: Json = json::parse(text, false).unwrap();
        settings_from_json(
            &json,
            |b| ["minecraft:stone", "minecraft:bedrock"].contains(&b),
            |b| ["minecraft:plains", "minecraft:desert"].contains(&b),
        )
    }

    #[test]
    fn mod_dimension_uses_data_layers_and_biome() {
        use horizon_world::{DATA_VERSION, chunk::BlockState, nbt::JavaString};
        let names = ["minecraft:air", "minecraft:bedrock", "example:soil"];
        let states = names
            .iter()
            .map(|name| {
                BlockState {
                    name: (*name).into(),
                    properties: Default::default(),
                }
                .to_nbt()
            })
            .collect();
        let defaults = Tag::compound(
            names
                .iter()
                .enumerate()
                .map(|(id, name)| (JavaString::from(*name), Tag::Int(id as i32))),
        );
        let registry = BlockRegistry::from_nbt(&Tag::compound([
            ("DataVersion", Tag::Int(DATA_VERSION)),
            ("states", Tag::List(states)),
            ("defaults", defaults),
        ]))
        .unwrap();
        let root = std::env::temp_dir().join(format!("horizon-flatgen-{}", std::process::id()));
        let path = root.join("data/example/dimension");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("moon.json"), r#"{"type":"example:moon","generator":{"type":"minecraft:flat","settings":{"biome":"example:moon","layers":[{"block":"minecraft:bedrock","height":1},{"block":"example:soil","height":3}]}}}"#).unwrap();
        let biomes = [("example:moon".to_owned(), 7)].into_iter().collect();
        let terrain = FlatTerrain::from_dimension_file_with_biomes(
            &root,
            "example:moon",
            -32,
            64,
            &registry,
            &biomes,
        )
        .unwrap()
        .unwrap();
        let chunk = terrain.finish(terrain.proto(&mut (), 0, 0));
        assert_eq!(chunk.get(0, -33, 0), 0);
        assert_eq!(chunk.get(0, -32, 0), 1);
        assert_eq!(chunk.get(0, -29, 0), 2);
        assert_eq!(chunk.get(0, -28, 0), 0);
        assert_eq!(chunk.biome(0, -32, 0), 7);
        assert_eq!(terrain.spawn_height(), -28);
        assert!(
            FlatTerrain::from_dimension_file_with_biomes(
                &root,
                "example:moon",
                -32,
                2,
                &registry,
                &biomes
            )
            .is_err()
        );
        assert!(
            FlatTerrain::from_dimension_file_with_biomes(
                &root,
                "example:../moon",
                -32,
                64,
                &registry,
                &biomes
            )
            .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generator_settings_read_as_flat_level_generator_settings() {
        // server.properties' default: the preset's own layers, and the codec's error.
        assert_eq!(
            read("{}"),
            (None, vec!["No key layers in MapLike[{}]".into()])
        );
        let (settings, errors) = read(
            r#"{"layers":[{"block":"bedrock","height":1},{"block":"minecraft:stone","height":3}],"biome":"desert","features":true}"#,
        );
        assert!(errors.is_empty());
        let settings = settings.unwrap();
        let Some(Tag::List(layers)) = settings.get("layers") else {
            panic!("{settings:?}")
        };
        assert_eq!(layers.len(), 2);
        assert_eq!(
            text(layers[0].get("block")).as_deref(),
            Some("minecraft:bedrock")
        );
        assert_eq!(number(layers[1].get("height")), Some(3));
        assert_eq!(
            text(settings.get("biome")).as_deref(),
            Some("minecraft:desert")
        );
        assert_eq!(number(settings.get("features")), Some(1));
        assert_eq!(number(settings.get("lakes")), Some(0));
        // A layer that doesn't parse is left out; an unknown biome is Plains, and logged first.
        let (settings, errors) = read(
            r#"{"layers":[{"block":"nope","height":2},{"block":"stone","height":2}],"biome":"x"}"#,
        );
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "Unknown biome, defaulting to plains");
        assert!(errors[1].contains("minecraft:nope"));
        let settings = settings.unwrap();
        let Some(Tag::List(layers)) = settings.get("layers") else {
            panic!("{settings:?}")
        };
        assert_eq!(layers.len(), 1);
        assert_eq!(
            text(settings.get("biome")).as_deref(),
            Some("minecraft:plains")
        );
        // validateHeight.
        assert_eq!(
            read(
                r#"{"layers":[{"block":"stone","height":4000},{"block":"stone","height":100}],"biome":"plains"}"#
            ),
            (None, vec!["Sum of layer heights is > 4064".into()])
        );
    }
}

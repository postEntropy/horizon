//! Bedrock geometry cubes and articulated bones, converted to the entity mesh.
use super::{Part, Polygon};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

fn vector<const N: usize>(v: &Value) -> Result<[f32; N], String> {
    let a = v
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or("invalid Geo vector")?;
    let mut result = [0.; N];
    for (out, value) in result.iter_mut().zip(a) {
        *out = value
            .as_f64()
            .filter(|v| v.is_finite() && v.abs() <= 100_000.0)
            .ok_or("invalid Geo number")? as f32;
    }
    Ok(result)
}
fn optional<const N: usize>(v: &Value, default: [f32; N]) -> Result<[f32; N], String> {
    if v.is_null() { Ok(default) } else { vector(v) }
}
fn part(name: String, pos: [f32; 3], rotation: [f32; 3]) -> Part {
    Part {
        name,
        pose: [
            pos[0],
            pos[1],
            pos[2],
            rotation[0].to_radians(),
            rotation[1].to_radians(),
            rotation[2].to_radians(),
            1.,
            1.,
            1.,
        ],
        polygons: Vec::new(),
        children: Vec::new(),
    }
}

/// Parse one Geo 1.12+ geometry. Unsupported elements are rejected, not ignored.
pub fn parse(doc: &Value) -> Result<Part, String> {
    let geometries = doc["minecraft:geometry"]
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or("Geo must contain one geometry")?;
    let geometry = &geometries[0];
    let width = geometry["description"]["texture_width"]
        .as_f64()
        .filter(|v| *v > 0. && *v <= 16384.)
        .ok_or("invalid Geo texture width")? as f32;
    let height = geometry["description"]["texture_height"]
        .as_f64()
        .filter(|v| *v > 0. && *v <= 16384.)
        .ok_or("invalid Geo texture height")? as f32;
    let bones = geometry["bones"].as_array().ok_or("Geo has no bones")?;
    if bones.len() > 1024 {
        return Err("too many Geo bones".into());
    }
    let mut indexed = BTreeMap::new();
    for bone in bones {
        let name = bone["name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .ok_or("Geo bone has no name")?;
        if name.starts_with("#geo_") {
            return Err("reserved Geo bone name".into());
        }
        if indexed.insert(name, bone).is_some() {
            return Err("duplicate Geo bone".into());
        }
        if bone.get("poly_mesh").is_some() || bone.get("texture_meshes").is_some() {
            return Err("unsupported Geo mesh".into());
        }
    }
    for bone in bones {
        if let Some(parent) = bone["parent"].as_str()
            && !indexed.contains_key(parent)
        {
            return Err("unknown Geo parent".into());
        }
    }
    fn build(
        name: &str,
        parent_pivot: [f32; 3],
        indexed: &BTreeMap<&str, &Value>,
        visited: &mut HashSet<String>,
        depth: usize,
        count: &mut usize,
        tex: [f32; 2],
    ) -> Result<Part, String> {
        if depth > 32 || !visited.insert(name.to_owned()) {
            return Err("cyclic or excessively deep Geo bones".into());
        }
        let bone = indexed[name];
        let pivot = optional(&bone["pivot"], [0.; 3])?;
        let mut result = part(
            name.to_owned(),
            std::array::from_fn(|i| pivot[i] - parent_pivot[i]),
            optional(&bone["rotation"], [0.; 3])?,
        );
        let cubes = match bone.get("cubes") {
            Some(cubes) => cubes.as_array().ok_or("invalid Geo cubes")?.as_slice(),
            None => &[],
        };
        *count += cubes.len();
        if *count > 4096 {
            return Err("too many Geo cubes".into());
        }
        for (cube_index, cube) in cubes.iter().enumerate() {
            let origin: [f32; 3] = vector(&cube["origin"])?;
            let size: [f32; 3] = vector(&cube["size"])?;
            if size.iter().any(|n| *n < 0.) {
                return Err("negative Geo cube size".into());
            }
            let inflate = match cube.get("inflate") {
                Some(value) => value.as_f64().ok_or("invalid Geo inflate")? as f32,
                None => 0.,
            };
            if !inflate.is_finite() || inflate.abs() > 100_000. {
                return Err("invalid Geo inflate".into());
            }
            let cp = optional(&cube["pivot"], pivot)?;
            let mut piece = part(
                format!("#geo_cube_{name}_{cube_index}"),
                std::array::from_fn(|i| cp[i] - pivot[i]),
                optional(&cube["rotation"], [0.; 3])?,
            );
            let lo: [f32; 3] = std::array::from_fn(|i| origin[i] - inflate - cp[i]);
            let hi: [f32; 3] = std::array::from_fn(|i| origin[i] + size[i] + inflate - cp[i]);
            let [x, y, z] = lo;
            let [max_x, max_y, max_z] = hi;
            let mirror = cube["mirror"]
                .as_bool()
                .or_else(|| bone["mirror"].as_bool())
                .unwrap_or(false);
            let uv = &cube["uv"];
            if !uv.is_array() && !uv.is_object() {
                return Err("invalid Geo cube UV".into());
            }
            let box_uv = if uv.is_array() {
                Some(vector::<2>(uv)?)
            } else {
                None
            };
            let [w, h, d] = size;
            let faces = [
                (
                    "north",
                    [[max_x, y, z], [x, y, z], [x, max_y, z], [max_x, max_y, z]],
                    [0., 0., -1.],
                    [d, d, w, h],
                ),
                (
                    "south",
                    [
                        [x, y, max_z],
                        [max_x, y, max_z],
                        [max_x, max_y, max_z],
                        [x, max_y, max_z],
                    ],
                    [0., 0., 1.],
                    [d + w + d, d, w, h],
                ),
                (
                    "west",
                    [[x, y, z], [x, y, max_z], [x, max_y, max_z], [x, max_y, z]],
                    [-1., 0., 0.],
                    [0., d, d, h],
                ),
                (
                    "east",
                    [
                        [max_x, y, max_z],
                        [max_x, y, z],
                        [max_x, max_y, z],
                        [max_x, max_y, max_z],
                    ],
                    [1., 0., 0.],
                    [d + w, d, d, h],
                ),
                (
                    "up",
                    [
                        [x, max_y, max_z],
                        [max_x, max_y, max_z],
                        [max_x, max_y, z],
                        [x, max_y, z],
                    ],
                    [0., 1., 0.],
                    [d, 0., w, d],
                ),
                (
                    "down",
                    [[x, y, z], [max_x, y, z], [max_x, y, max_z], [x, y, max_z]],
                    [0., -1., 0.],
                    [d + w, 0., w, d],
                ),
            ];
            for (face, coords, normal, mut rect) in faces {
                let uv_face = if mirror {
                    match face {
                        "east" => {
                            rect[0] = 0.;
                            "west"
                        }
                        "west" => {
                            rect[0] = d + w;
                            "east"
                        }
                        _ => face,
                    }
                } else {
                    face
                };
                let (start, extent) = if let Some(base) = box_uv {
                    ([base[0] + rect[0], base[1] + rect[1]], [rect[2], rect[3]])
                } else if let Some(face) = uv.get(uv_face) {
                    if face.get("uv_rotation").is_some() || face.get("material_instance").is_some()
                    {
                        return Err("unsupported Geo face property".into());
                    }
                    (
                        vector::<2>(&face["uv"])?,
                        optional(&face["uv_size"], [rect[2], rect[3]])?,
                    )
                } else {
                    continue;
                };
                let [u, v] = start;
                let [du, dv] = extent;
                let mut uvs = [[u + du, v + dv], [u, v + dv], [u, v], [u + du, v]];
                if mirror {
                    for p in &mut uvs {
                        p[0] = 2. * u + du - p[0];
                    }
                }
                piece.polygons.push(Polygon {
                    normal,
                    vertices: std::array::from_fn(|i| {
                        [
                            coords[i][0],
                            coords[i][1],
                            coords[i][2],
                            uvs[i][0] / tex[0],
                            uvs[i][1] / tex[1],
                        ]
                    }),
                });
            }
            result.children.push(piece);
        }
        for (child, data) in indexed {
            if data["parent"].as_str() == Some(name) {
                result.children.push(build(
                    child,
                    pivot,
                    indexed,
                    visited,
                    depth + 1,
                    count,
                    tex,
                )?);
            }
        }
        Ok(result)
    }
    // Geo has Y=0 at the feet; the vanilla living-entity renderer uses a
    // downward model axis with the origin 24 model pixels above the feet.
    let mut root = part("#geo_root".into(), [0., 24., 0.], [0.; 3]);
    root.pose[7] = -1.;
    let mut visited = HashSet::new();
    let mut count = 0;
    for (name, bone) in &indexed {
        if bone["parent"].is_null() {
            root.children.push(build(
                name,
                [0.; 3],
                &indexed,
                &mut visited,
                0,
                &mut count,
                [width, height],
            )?);
        }
    }
    if visited.len() != bones.len() {
        return Err("cyclic Geo bones".into());
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn imports_bone_hierarchy_box_uv_and_per_face_uv() {
        let model = parse(&json!({"format_version":"1.12.0", "minecraft:geometry":[{
            "description":{"identifier":"geometry.test","texture_width":64,"texture_height":32},
            "bones":[
                {"name":"root_bone","pivot":[0,12,0],"cubes":[{"origin":[-2,10,-2],"size":[4,4,4],"uv":[0,0]}]},
                {"name":"head","parent":"root_bone","pivot":[0,16,0],"rotation":[0,90,0],"cubes":[{"origin":[-1,15,-1],"size":[2,2,2],"uv":{"north":{"uv":[4,2],"uv_size":[2,2]}}}]}
            ]
        }]})).unwrap();
        let body = &model.children[0];
        assert_eq!(body.name, "root_bone");
        assert_eq!(body.children[0].polygons.len(), 6);
        let head = &body.children[1];
        assert_eq!(head.name, "head");
        assert_eq!(head.pose[1], 4.0);
        assert!((head.pose[4] - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(head.children[0].polygons.len(), 1);
        let mut vertices = Vec::new();
        let poses = crate::mobs::anim::Poses::new(&model);
        crate::mobs::render_model(
            &model,
            glam::Mat4::IDENTITY,
            &|p| poses.pose(p),
            [0., 0., 1., 1.],
            0,
            0,
            &mut vertices,
        );
        assert_eq!(vertices.len(), 28);
    }

    #[test]
    fn rejects_cycles_and_missing_parents() {
        let mut doc = json!({"minecraft:geometry":[{"description":{"texture_width":16,"texture_height":16},"bones":[{"name":"a","parent":"b"},{"name":"b","parent":"a"}]}]});
        assert!(parse(&doc).is_err());
        doc["minecraft:geometry"][0]["bones"][0]["parent"] = json!("missing");
        assert!(parse(&doc).is_err());
    }
}

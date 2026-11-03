//! Package a rebuilt port plus locally carried resources into a scratch hmod.
//! MANIFEST CORE_WASM CARRY_DIR OUTPUT_HMOD. Never modifies the carry directory.
use horizon_mods::{Package, manifest::componentize};
use std::{fs, path::Path};

fn collect(root: &Path, path: &Path, out: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect(root, &entry.path(), out)?;
        } else if entry.file_type()?.is_file() {
            let name = entry
                .path()
                .strip_prefix(root)
                .expect("under carry root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((name, fs::read(entry.path())?));
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("MANIFEST CORE_WASM CARRY_DIR OUTPUT_HMOD".into());
    }
    let root = Path::new(&args[2]);
    let mut files = Vec::new();
    for prefix in ["assets", "data"] {
        if root.join(prefix).is_dir() {
            collect(root, &root.join(prefix), &mut files)?;
        }
    }
    let refs: Vec<_> = files
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();
    let component = componentize(&fs::read(&args[1])?)?;
    fs::write(
        &args[3],
        Package::write_files(&fs::read_to_string(&args[0])?, Some(&component), &refs)?,
    )?;
    Ok(())
}

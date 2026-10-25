---
name: port-modpack
description: >-
  Locate an imported Minecraft modpack (such as All the Mods 10) in Horizon,
  inspect pending mods across Mod API 0.3 (logic), 0.4 (machines), and 0.5 (deep),
  and port them to WebAssembly (mod.wasm) so the Horizon Launcher and game load them natively.
  Triggers: port modpack, port ATM10, port pending mods, needs-logic, mod.wasm, horizon-port-pack.
---

# Porting Modpack Mods to Horizon (`port-modpack`)

Use this skill when asked to inspect an imported modpack, see which mods still need code ported, or port one or more mods from `needs-logic` (`0.3` `logic`, `0.4` `machines`, `0.5` `deep`) to `ported-wasm`.

---

## 1. Where Imported Modpacks Live

When a player imports a `.zip` or `.mrpack` in the **Horizon Launcher** (or via `horizon-launcher modpack import <file>`), the work directory `<work>` is stored at:

- **Default player path:** `~/.local/share/horizon/ports/<pack-slug>/` (or `$XDG_DATA_HOME/horizon/ports/<pack-slug>/`)
- **Custom `HORIZON_HOME`:** `$HORIZON_HOME/ports/<pack-slug>/`

You can list all imported modpacks and their current progress via:

```bash
./launcher/target/debug/horizon-launcher modpack list --json
```

Inside `<work>/` (`~/.local/share/horizon/ports/<pack-slug>/`):
- `pack.json`: normalised pack manifest (Minecraft version, loader, files, SHA-1s).
- `inventory.json`: every mod's `id`, `name`, `class`, `tier` (`data`, `logic`, `machines`, `deep`, `client`, `none`), `license`, `license_kind`, `deps`, and dependency `layer`.
- `coverage.json` & `coverage.md`: current port status of every mod (`ported`, `partial`, `not-yet`, `not-needed`) and `reason`.
- `jars/`: downloaded `.jar` files for the pack.
- `mods/<id>/carry/`: carried-over 26.3 `assets/`, `data/`, `horizon-mod.toml`, and (once ported) `mod.wasm`.
- `mods/<id>/carry-report.json`: summary of carried blocks, items, recipes, loot tables, and the Java class features detected (`block-entities`, `menus`, `blocks`, `items`, `entities`, `mixins`, etc.).

---

## 2. Listing Pending Mods to Port

Run this Python snippet against `<work>` to list all downloaded mods that have their assets/data carried over and are waiting for their WebAssembly logic (`mod.wasm`), sorted by tier and dependency layer:

```bash
python3 -c '
import json, pathlib, sys

work = pathlib.Path(sys.argv[1]).expanduser()
inv = {m["key"]: m for m in json.loads((work / "inventory.json").read_text())}
cov = json.loads((work / "coverage.json").read_text())

pending = []
for r in cov:
    if r["tier"] in ("logic", "machines", "deep") and not r["reason"].startswith("ported-wasm"):
        m = inv.get(r["key"], {})
        mod_id = m.get("id") or r["key"]
        has_jar = bool(m.get("jar")) and (work / "jars" / m["jar"]).is_file()
        has_carry = (work / "mods" / mod_id / "carry" / "horizon-mod.toml").is_file()
        pending.append((m.get("layer", 0), r["tier"], mod_id, r["name"], has_jar, has_carry, r["reason"]))

pending.sort(key=lambda x: (not x[5], not x[4], {"logic": 1, "machines": 2, "deep": 3}[x[1]], x[0], x[2]))
print(f"Total pending code mods: {len(pending)}")
for layer, tier, mod_id, name, has_jar, has_carry, reason in pending[:40]:
    print(f"  [{tier:8s} | layer {layer}] {mod_id:24s} ({name}) jar={has_jar} carry={has_carry} -> {reason}")
' ~/.local/share/horizon/ports/all-the-mods-10
```

---

## 3. Rules Before Writing a Port

1. **Never decompile or read a mod's Java `.class` source code** (`docs/modpacks.md` § Licences rule 4).
2. Inspect **only**:
   - `<work>/mods/<id>/carry-report.json` (which lists the high-level features detected: `blocks`, `items`, `block-entities`, `menus`, `entities`, etc.)
   - `<work>/mods/<id>/carry/data/<id>/horizon/block/*.json` and `item/*.json` (the registered blocks, states, and items)
   - `<work>/mods/<id>/carry/assets/<id>/lang/en_us.json` (user-facing names, tooltips, messages, and config labels)
   - `<work>/mods/<id>/carry/data/<id>/recipe/*.json` and `loot_table/**/*.json`
   - Public documentation of the mod's gameplay behaviour.
3. If `<work>/mods/<id>/carry/` does not exist yet for `<id>`, run `carry` first:
   ```bash
   cargo run -p horizon-port-pack -- carry <work> --resources .local/horizon-client/26.3 <id>
   ```

---

## 4. Mapping Tiers to Mod API Versions & Capabilities

| Tier | API Version | Typical `capabilities` in `horizon-mod.toml` | Key SDK Interfaces (`horizon_mod_sdk`) |
| :--- | :---: | :--- | :--- |
| **`logic`** | `"0.3"` | `["registries", "config", "server", "keys", "target", "inventory-read", "world-read", "chat", "hud", "game-read"]` | `Mod::logic_hooks`, `on_block`, `on_item`, `on_server`, `server::*`, `world::*`, `config::*`, `keys::*`, `target::*`, `inventory::*` |
| **`machines`** | `"0.4"` | `["registries", "block-entities", "capabilities", "ui", "net", "recipes", "world-read", "render", ...]` | `Mod::block_entity_types`, `create_block_entity`, `tick_block_entities`, `save_block_entity`, `machine::*`, `capabilities::*`, `ui::*`, `net::*`, `recipes::*`, `render::*` |
| **`deep`** | `"0.5"` | `["registries", "entities", "models", "dimensions", "vanilla-hooks", ...]` | `Mod::entity_types`, `create_entity`, `tick_entities`, `save_entity`, `on_entity_interact`, `on_entity_hurt`, `on_vanilla_hook`, `entities::*`, `models::*`, `dimensions::*`, `vanilla_hooks::*` |

Full WIT contract and capability table: `docs/api/mods-reference.md` and `crates/horizon-mod-sdk/src/lib.rs`.

---

## 5. Step-by-Step Porting Workflow for a Mod (`<id>`)

### Step 1: Inspect the carried data of `<id>`
Check the blocks, items, and language keys in `<work>/mods/<id>/carry/`:
```bash
ls <work>/mods/<id>/carry/data/<id>/horizon/block/ <work>/mods/<id>/carry/data/<id>/horizon/item/
cat <work>/mods/<id>/carry-report.json
cat <work>/mods/<id>/carry/assets/<id>/lang/en_us.json
```

### Step 2: Create `<work>/mods/<id>/port/`
Create `<work>/mods/<id>/port/Cargo.toml`:
```toml
[package]
name = "<id>-port"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
horizon-mod-sdk = { path = "/home/leonardo/Documents/horizon-fork/crates/horizon-mod-sdk" }
```

And implement `Mod` in `<work>/mods/<id>/port/src/lib.rs`, ending with:
```rust
horizon_mod_sdk::export_mod!(MyMod);
```

### Step 3: Build `wasm32-unknown-unknown` and Componentize into `carry/mod.wasm`
Build the WASM guest and wrap the core module into a WebAssembly Component (`mod.wasm`) inside `<work>/mods/<id>/carry/mod.wasm`:

```bash
cargo build --manifest-path <work>/mods/<id>/port/Cargo.toml --target wasm32-unknown-unknown --release
```

Then componentize the resulting `.wasm` into `<work>/mods/<id>/carry/mod.wasm` and update `<work>/mods/<id>/carry/horizon-mod.toml`:
- Set `api = "0.3"` (or `"0.4"` / `"0.5"`).
- List every capability imported by the WASM component in `capabilities = [...]` (keep `"registries"` if the mod has `assets/` or `data/`).
- Preserve `namespaces = [...]` if present in `horizon-mod.toml`.

You can componentize and verify that the manifest + WASM load cleanly using `horizon-mods`:
```bash
cargo test -p horizon-mods --features runtime,pack -- --nocapture
```
Or with a one-liner in a small Rust example/test calling `horizon_mods::manifest::componentize(&raw_wasm)` and writing `<work>/mods/<id>/carry/mod.wasm`.

### Step 4: Refresh `<work>` Report so the Launcher Updates
Run `carry` and `report` on `<work>`:
```bash
cargo run -p horizon-port-pack -- carry <work> --resources .local/horizon-client/26.3 <id>
cargo run -p horizon-port-pack -- report <work>
```
- `carry.rs` automatically preserves `<work>/mods/<id>/carry/mod.wasm` and your `horizon-mod.toml` (`api`, `capabilities`, `config`).
- `report.rs` detects `carry/mod.wasm` and marks the mod as `ported-wasm (<details>)`.
- In the **Horizon Launcher**, the modpack's **`X / Y mods ported`** counter increments immediately (and clicking **Launch** enables the mod via `pipeline::loadable`).

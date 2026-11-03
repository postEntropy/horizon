use horizon_mod_sdk::{
    Mod, Registration, chat, game, hud, inventory, keys, server, storage, target, world,
};

struct CrashUtilitiesMod {
    hud_enabled: bool,
    show_slot_tooltips: bool,
}

impl CrashUtilitiesMod {
    fn update_hud(&self) {
        if !self.hud_enabled {
            hud::clear();
            return;
        }
        let mut lines = Vec::new();
        if let Some(p) = game::get_player() {
            let bx = p.x.floor() as i32;
            let by = p.y.floor() as i32;
            let bz = p.z.floor() as i32;
            let biome = world::get_biome(bx, by, bz).unwrap_or_else(|| "unloaded".into());
            let light = world::get_light(bx, by, bz).unwrap_or(0);
            lines.push(hud::Line {
                text: format!(
                    "CrashUtils: {} ({bx},{by},{bz}) chunk=({},{})",
                    p.dimension,
                    bx >> 4,
                    bz >> 4
                ),
                color: 0x55FFFF,
            });
            lines.push(hud::Line {
                text: format!(
                    "Biome={biome} sky={} blk={}",
                    light >> 4,
                    light & 0x0F
                ),
                color: 0xAAAAAA,
            });
        }
        if self.show_slot_tooltips {
            let inv = inventory::get_player_inventory();
            let open = inventory::get_open_container();
            lines.push(hud::Line {
                text: format!("Slots: inv={} open={}", inv.len(), open.len()),
                color: 0xFFAA00,
            });
        }
        if let Some(ent) = target::get_entity() {
            lines.push(hud::Line {
                text: format!("Target Entity: {} ({:.1}/{:.1})", ent.kind, ent.health, ent.max_health),
                color: 0xFF5555,
            });
        } else if let Some(hit) = target::get_block() {
            lines.push(hud::Line {
                text: format!("Target Block: {} @ {},{},{}", hit.state, hit.x, hit.y, hit.z),
                color: 0x55FF55,
            });
        }
        hud::set_panel(hud::Corner::TopRight, &lines);
    }
}

impl Mod for CrashUtilitiesMod {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&keys::KeyBinding {
            id: "crashutils.openscreen".into(),
            label: "Open the Crash Utilities Screen".into(),
            default_key: "KeyU".into(),
        });
        let _ = keys::register(&keys::KeyBinding {
            id: "crashutils.slottooltips".into(),
            label: "Show Slot Index Tooltips".into(),
            default_key: "KeyI".into(),
        });
        let hud_enabled = storage::get("hud_enabled")
            .and_then(|b| String::from_utf8(b).ok())
            .is_some_and(|s| s == "true");
        let show_slot_tooltips = storage::get("slot_tooltips")
            .and_then(|b| String::from_utf8(b).ok())
            .map_or(true, |s| s == "true");
        Ok((
            Self {
                hud_enabled,
                show_slot_tooltips,
            },
            Registration::new().command("crashutilities").on_tick(),
        ))
    }

    fn tick(&mut self, tick: u64) {
        if keys::was_pressed("crashutils.openscreen") {
            self.hud_enabled = !self.hud_enabled;
            let _ = storage::set(
                "hud_enabled",
                if self.hud_enabled { b"true" } else { b"false" },
            );
            chat::show(&format!("Crash Utilities overlay: {}", self.hud_enabled));
            self.update_hud();
        }
        if keys::was_pressed("crashutils.slottooltips") {
            self.show_slot_tooltips = !self.show_slot_tooltips;
            let _ = storage::set(
                "slot_tooltips",
                if self.show_slot_tooltips {
                    b"true"
                } else {
                    b"false"
                },
            );
            chat::show(&format!(
                "Slot Index Tooltips: {}",
                self.show_slot_tooltips
            ));
            self.update_hud();
        }
        if self.hud_enabled && tick.is_multiple_of(10) {
            self.update_hud();
        }
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["check"] | ["status"] => {
                let Some(p) = game::get_player() else {
                    return Ok(Some("CrashUtilities: no active player".into()));
                };
                let (bx, by, bz) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
                let mut loaded_cells = 0u32;
                for dx in -2..=2 {
                    for dz in -2..=2 {
                        if world::get_block(bx + dx, by - 1, bz + dz).is_some() {
                            loaded_cells += 1;
                        }
                    }
                }
                let biome = world::get_biome(bx, by, bz).unwrap_or_else(|| "unloaded".into());
                let light = world::get_light(bx, by, bz).unwrap_or(0);
                let target_desc = if let Some(ent) = target::get_entity() {
                    format!("entity:{}({:.1}/{:.1})", ent.kind, ent.health, ent.max_health)
                } else if let Some(blk) = target::get_block() {
                    format!("block:{}@{},{},{}", blk.state, blk.x, blk.y, blk.z)
                } else {
                    "none".into()
                };
                let tick_speed =
                    server::get_rule("random_tick_speed").unwrap_or_else(|| "3".into());
                Ok(Some(format!(
                    "CrashUtilities: dim={} pos=({bx},{by},{bz}) chunk=({},{}) biome={biome} light={}/{} loaded_5x5={loaded_cells}/25 target={target_desc} random_tick_speed={tick_speed}",
                    p.dimension,
                    bx >> 4,
                    bz >> 4,
                    light >> 4,
                    light & 0x0F
                )))
            }
            ["invsee"] => {
                let main = inventory::get_main_hand()
                    .map(|s| format!("{}:{}*{}", s.slot, s.id, s.count))
                    .unwrap_or_else(|| "empty".into());
                let off = inventory::get_off_hand()
                    .map(|s| format!("{}:{}*{}", s.slot, s.id, s.count))
                    .unwrap_or_else(|| "empty".into());
                let inv = inventory::get_player_inventory();
                let open = inventory::get_open_container();
                let slots_preview: Vec<String> = inv
                    .iter()
                    .take(8)
                    .map(|s| format!("[{}]{}*{}", s.slot, s.id, s.count))
                    .collect();
                let open_preview: Vec<String> = open
                    .iter()
                    .take(8)
                    .map(|s| format!("[{}]{}*{}", s.slot, s.id, s.count))
                    .collect();
                Ok(Some(format!(
                    "CrashUtilities InvSee: main={main} off={off} inv({})=[{}] open({})=[{}]",
                    inv.len(),
                    slots_preview.join(", "),
                    open.len(),
                    open_preview.join(", ")
                )))
            }
            ["hud", state] => {
                self.hud_enabled = *state == "on" || *state == "true";
                let _ = storage::set(
                    "hud_enabled",
                    if self.hud_enabled { b"true" } else { b"false" },
                );
                self.update_hud();
                Ok(Some(format!("CrashUtilities HUD={}", self.hud_enabled)))
            }
            _ => Err("Usage: /crashutilities [check | invsee | hud <on|off>]".into()),
        }
    }
}

pub fn parse_hud_state(state: &str) -> bool {
    state == "on" || state == "true"
}

pub fn format_coords_info(dim: &str, x: i32, y: i32, z: i32) -> String {
    format!("CrashUtils: {} ({x},{y},{z}) chunk=({},{})", dim, x >> 4, z >> 4)
}

horizon_mod_sdk::export_mod!(CrashUtilitiesMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hud_state() {
        assert!(parse_hud_state("on"));
        assert!(parse_hud_state("true"));
        assert!(!parse_hud_state("off"));
        assert!(!parse_hud_state("false"));
    }

    #[test]
    fn test_format_coords_info() {
        let text = format_coords_info("minecraft:overworld", 100, 64, -200);
        assert_eq!(text, "CrashUtils: minecraft:overworld (100,64,-200) chunk=(6,-13)");
    }
}

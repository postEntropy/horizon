//! Chisel Reborn: the chisel re-shapes blocks into architectural and decorative variants.
//!
//! Supports cycling patterns for materials (e.g. stone, andesite, cobblestone, concrete)
//! across all carried Chisel groups, transforming vanilla blocks into chisel variants,
//! and multi-block chisel modes: Single, 3x3 Panel, 3x1 Column, 3x1 Row, and Contiguous.

mod items;
mod variants;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const MODES: [&str; 5] = ["Single", "Panel 3x3", "Column 1x3", "Row 3x1", "Contiguous"];
const MODE_SCREEN: &str = "screen:chisel:mode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChiselMode {
    Single = 0,
    Panel3x3 = 1,
    Column3x1 = 2,
    Row3x1 = 3,
    Contiguous = 4,
}

impl ChiselMode {
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Panel3x3,
            2 => Self::Column3x1,
            3 => Self::Row3x1,
            4 => Self::Contiguous,
            _ => Self::Single,
        }
    }

    pub fn name(&self) -> &'static str {
        MODES[*self as usize]
    }
}

pub fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

/// `chisel:<group>/<variant>` split into (group, material).
pub fn split_chisel_id(id: &str) -> Option<(&str, &str)> {
    id.strip_prefix("chisel:")?.split_once('/')
}

/// Strip namespace (e.g. "minecraft:stone" -> "stone", "chisel:array/stone" -> "stone").
pub fn extract_material(id: &str) -> &str {
    let base = block_id(id);
    if let Some((_, mat)) = split_chisel_id(base) {
        return mat;
    }
    base.split(':').nth(1).unwrap_or(base)
}

/// Find all groups in `variants::GROUPS` that contain the given material.
pub fn groups_for_material(material: &str) -> Vec<&'static str> {
    variants::GROUPS
        .iter()
        .filter_map(|(group, variants)| {
            if variants.contains(&material) {
                Some(*group)
            } else {
                None
            }
        })
        .collect()
}

/// Determine the next variant for a given block id.
pub fn next_variant_for(id: &str) -> Option<String> {
    let clean = block_id(id);
    if let Some((current_group, material)) = split_chisel_id(clean) {
        let groups = groups_for_material(material);
        if groups.is_empty() {
            return None;
        }
        if let Some(pos) = groups.iter().position(|&g| g == current_group) {
            let next_group = groups[(pos + 1) % groups.len()];
            Some(format!("chisel:{next_group}/{material}"))
        } else {
            Some(format!("chisel:{}/{material}", groups[0]))
        }
    } else {
        // Vanilla or other block: try to match material in Chisel groups
        let material = clean.split(':').nth(1).unwrap_or(clean);
        let groups = groups_for_material(material);
        if !groups.is_empty() {
            Some(format!("chisel:{}/{material}", groups[0]))
        } else {
            None
        }
    }
}

/// Compute relative block offsets for a chiseling operation based on mode and click normal/face.
pub fn compute_offsets(mode: ChiselMode) -> Vec<(i32, i32, i32)> {
    match mode {
        ChiselMode::Single => vec![(0, 0, 0)],
        ChiselMode::Panel3x3 => {
            let mut offsets = Vec::with_capacity(9);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    offsets.push((dx, 0, dz));
                }
            }
            offsets
        }
        ChiselMode::Column3x1 => vec![(0, -1, 0), (0, 0, 0), (0, 1, 0)],
        ChiselMode::Row3x1 => vec![(-1, 0, 0), (0, 0, 0), (1, 0, 0)],
        ChiselMode::Contiguous => {
            // Plus-shaped cross (5 blocks)
            vec![(0, 0, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)]
        }
    }
}

pub struct Chisel {
    pub mode: ChiselMode,
}

impl Chisel {
    pub fn new() -> Self {
        Self {
            mode: ChiselMode::Single,
        }
    }

    pub fn set_mode(&mut self, mode: ChiselMode) {
        self.mode = mode;
    }

    pub fn mode_screen(&self) -> ScreenSpec {
        let current_idx = self.mode as usize;
        let mut widgets = vec![Widget {
            id: 10,
            kind: WidgetKind::Label,
            x: 8,
            y: 8,
            width: 160,
            height: 10,
            text: format!("Chisel Mode: {}", self.mode.name()),
            value: 0,
            max_value: 0,
        }];

        for (i, name) in MODES.iter().enumerate() {
            let y = 24 + (i as i32 * 20);
            widgets.push(Widget {
                id: i as u32,
                kind: WidgetKind::Button,
                x: 8,
                y,
                width: 160,
                height: 18,
                text: if i == current_idx {
                    format!("▶ {name} ◀")
                } else {
                    (*name).to_string()
                },
                value: (i == current_idx) as u64,
                max_value: 1,
            });
        }

        ScreenSpec {
            id: MODE_SCREEN.into(),
            title: "Chisel Settings".into(),
            width: 176,
            height: 136,
            widgets,
        }
    }
}

impl Mod for Chisel {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Chisel::new(), Registration::new().command("chisel")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseInAir {
            let _ = ui::open_screen(&self.mode_screen());
            return EventOutcome::Consume;
        }
        if event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }

        let origin = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        let target = if event.target.is_empty() {
            match server::get_block(&event.dimension, origin) {
                Some(state) => state,
                None => return EventOutcome::Pass,
            }
        } else {
            event.target.clone()
        };

        let target_base = block_id(&target);
        let Some(next) = next_variant_for(target_base) else {
            let mat = extract_material(target_base);
            chat::show(&format!("🔨 '{mat}' has no Chisel variants."));
            return EventOutcome::Pass;
        };

        let offsets = compute_offsets(self.mode);
        let mut transformed = 0;

        for (dx, dy, dz) in offsets {
            let pos = BlockPos {
                x: origin.x + dx,
                y: origin.y + dy,
                z: origin.z + dz,
            };

            // In multi-block mode, only replace blocks of the same matching material or group
            if dx != 0 || dy != 0 || dz != 0 {
                if let Some(existing) = server::get_block(&event.dimension, pos) {
                    if extract_material(block_id(&existing)) != extract_material(target_base) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if server::set_block(&event.dimension, pos, &next).is_ok() {
                transformed += 1;
            }
        }

        if transformed > 0 {
            server::play_sound(&event.dimension, origin, "chisel:chisel_sound", 0.8, 1.0);
            let display_name = next.strip_prefix("chisel:").unwrap_or(&next);
            chat::show(&format!(
                "🔨 Chiseled {transformed} block(s) into '{display_name}' [{}]",
                self.mode.name()
            ));
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != MODE_SCREEN {
            return;
        }
        let index = button as usize;
        if index < MODES.len() {
            self.mode = ChiselMode::from_index(index);
            chat::show(&format!("🔨 Chisel mode set to: {}", self.mode.name()));
            let _ = ui::open_screen(&self.mode_screen());
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "chisel" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] => Ok(Some(format!(
                "Chisel: mode '{}' · {} groups carried · Right-click block to chisel, air to configure.",
                self.mode.name(),
                variants::GROUPS.len()
            ))),
            ["mode", mode_name] => {
                let lower = mode_name.to_lowercase();
                if let Some(pos) = MODES.iter().position(|m| m.to_lowercase().contains(&lower)) {
                    self.mode = ChiselMode::from_index(pos);
                    Ok(Some(format!("🔨 Chisel mode set to: {}", self.mode.name())))
                } else {
                    Err(format!(
                        "Unknown mode '{mode_name}'. Valid: {}",
                        MODES.join(", ")
                    ))
                }
            }
            ["groups"] => Ok(Some(format!(
                "Groups ({}): {}",
                variants::GROUPS.len(),
                variants::GROUPS
                    .iter()
                    .map(|(g, _)| *g)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
            ["material", mat] => {
                let groups = groups_for_material(mat);
                if groups.is_empty() {
                    Ok(Some(format!("No Chisel groups support '{mat}'")))
                } else {
                    Ok(Some(format!(
                        "Material '{mat}' in {} groups: {}",
                        groups.len(),
                        groups.join(", ")
                    )))
                }
            }
            _ => Err("Usage: /chisel [mode <name> | groups | material <name>]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(Chisel);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_material() {
        assert_eq!(extract_material("chisel:array/andesite"), "andesite");
        assert_eq!(extract_material("chisel:braid/stone[axis=y]"), "stone");
        assert_eq!(extract_material("minecraft:andesite"), "andesite");
        assert_eq!(extract_material("cobblestone"), "cobblestone");
    }

    #[test]
    fn test_groups_for_material() {
        let andesite_groups = groups_for_material("andesite");
        assert!(!andesite_groups.is_empty());
        assert!(andesite_groups.contains(&"array"));

        let stone_groups = groups_for_material("stone");
        assert!(!stone_groups.is_empty());
        assert!(stone_groups.contains(&"array"));
    }

    #[test]
    fn test_next_variant_cycling() {
        // Cycling a chisel block
        let next = next_variant_for("chisel:array/andesite").expect("should find next variant");
        assert!(next.starts_with("chisel:"));
        assert!(next.ends_with("/andesite"));
        assert_ne!(next, "chisel:array/andesite");

        // Cycling vanilla block
        let vanilla_converted =
            next_variant_for("minecraft:andesite").expect("should convert vanilla");
        assert!(vanilla_converted.starts_with("chisel:"));
        assert!(vanilla_converted.ends_with("/andesite"));
    }

    #[test]
    fn test_chisel_modes_and_offsets() {
        let single = compute_offsets(ChiselMode::Single);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0], (0, 0, 0));

        let panel = compute_offsets(ChiselMode::Panel3x3);
        assert_eq!(panel.len(), 9);

        let column = compute_offsets(ChiselMode::Column3x1);
        assert_eq!(column.len(), 3);

        let row = compute_offsets(ChiselMode::Row3x1);
        assert_eq!(row.len(), 3);

        let contiguous = compute_offsets(ChiselMode::Contiguous);
        assert_eq!(contiguous.len(), 5);
    }

    #[test]
    fn test_chisel_mode_switch() {
        let mut chisel = Chisel::new();
        assert_eq!(chisel.mode, ChiselMode::Single);

        chisel.set_mode(ChiselMode::Panel3x3);
        assert_eq!(chisel.mode, ChiselMode::Panel3x3);

        chisel.set_mode(ChiselMode::Row3x1);
        assert_eq!(chisel.mode, ChiselMode::Row3x1);

        assert_eq!(ChiselMode::from_index(2), ChiselMode::Column3x1);
        assert_eq!(ChiselMode::from_index(4), ChiselMode::Contiguous);
    }
}

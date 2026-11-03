//! Construction Sticks: building wands that extend block surfaces rapidly.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WandTier {
    Wooden,
    Copper,
    Iron,
    Diamond,
    Netherite,
}

impl WandTier {
    pub fn max_blocks(&self) -> usize {
        match self {
            WandTier::Wooden => 9,
            WandTier::Copper => 16,
            WandTier::Iron => 32,
            WandTier::Diamond => 64,
            WandTier::Netherite => 128,
        }
    }

    pub fn from_item(item: &str) -> Self {
        if item.contains("netherite") {
            WandTier::Netherite
        } else if item.contains("diamond") {
            WandTier::Diamond
        } else if item.contains("iron") {
            WandTier::Iron
        } else if item.contains("copper") {
            WandTier::Copper
        } else {
            WandTier::Wooden
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WandMode {
    Normal,
    Angel,
    Destruction,
}

impl WandMode {
    pub fn from_item(item: &str) -> Self {
        if item.contains("angel") {
            WandMode::Angel
        } else if item.contains("destruction") {
            WandMode::Destruction
        } else {
            WandMode::Normal
        }
    }
}

pub fn calculate_surface_positions(
    origin: (i32, i32, i32),
    face_normal: (i32, i32, i32),
    limit: usize,
) -> Vec<(i32, i32, i32)> {
    let mut positions = Vec::new();
    let (ox, oy, oz) = origin;
    let (nx, ny, nz) = face_normal;

    // Placed position directly adjacent to the face
    let start_x = ox + nx;
    let start_y = oy + ny;
    let start_z = oz + nz;

    // Determine orthogonal axes to the clicked face
    let (u_axis, v_axis): ((i32, i32, i32), (i32, i32, i32)) = if nx != 0 {
        // Face is X-aligned; spread in Y and Z
        ((0, 1, 0), (0, 0, 1))
    } else if ny != 0 {
        // Face is Y-aligned; spread in X and Z
        ((1, 0, 0), (0, 0, 1))
    } else {
        // Face is Z-aligned; spread in X and Y
        ((1, 0, 0), (0, 1, 0))
    };

    let side_span = ((limit as f32).sqrt().ceil() as i32).max(1);

    for u in 0..side_span {
        for v in 0..side_span {
            if positions.len() >= limit {
                break;
            }
            let px = start_x + u * u_axis.0 + v * v_axis.0;
            let py = start_y + u * u_axis.1 + v * v_axis.1;
            let pz = start_z + u * u_axis.2 + v * v_axis.2;
            positions.push((px, py, pz));
        }
    }

    positions
}

pub fn wand_settings_screen(item_name: &str, tier: WandTier, mode: WandMode) -> ScreenSpec {
    let mode_str = match mode {
        WandMode::Normal => "Extend Surfaces",
        WandMode::Angel => "Angel (Mid-Air)",
        WandMode::Destruction => "Destruction (Break Connected)",
    };

    ScreenSpec {
        id: "screen:constructionstick:settings".into(),
        title: format!("{item_name} Settings"),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 10,
                width: 160,
                height: 12,
                text: format!("Tier: {:?} (Max: {})", tier, tier.max_blocks()),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 28,
                width: 160,
                height: 12,
                text: format!("Mode: {mode_str}"),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 76,
                y: 50,
                width: 24,
                height: 17,
                text: "Configured".into(),
                value: 100,
                max_value: 100,
            },
        ],
    }
}

pub struct ConstructionStick;

impl Mod for ConstructionStick {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ConstructionStick, Registration::new()))
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
        let tier = WandTier::from_item(&event.item);
        let mode = WandMode::from_item(&event.item);

        if event.action == ItemAction::UseInAir {
            if event.item.contains("stick") {
                let screen = wand_settings_screen(&event.item, tier, mode);
                let _ = ui::open_screen(&screen);
                chat::show(&format!(
                    "🪄 Construction Stick ({:?}, {:?} mode, limit: {})",
                    tier,
                    mode,
                    tier.max_blocks()
                ));
                return EventOutcome::Consume;
            }
        }

        if event.action == ItemAction::UseOnBlock {
            let limit = tier.max_blocks();
            let base_pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };

            let Some(target_state) = server::get_block(&event.dimension, base_pos) else {
                return EventOutcome::Pass;
            };

            if mode == WandMode::Destruction {
                // Destruction wand breaks adjacent identical blocks
                let mut destroyed = 0;
                for offset in 1..=limit {
                    let next_pos = BlockPos {
                        x: event.x + offset as i32,
                        y: event.y,
                        z: event.z,
                    };
                    if let Some(blk) = server::get_block(&event.dimension, next_pos) {
                        if blk == target_state {
                            let _ = server::set_block(&event.dimension, next_pos, "minecraft:air");
                            destroyed += 1;
                        }
                    }
                }
                if destroyed > 0 {
                    chat::show(&format!("💥 Destruction Stick removed {destroyed} blocks."));
                    server::play_sound(
                        &event.dimension,
                        base_pos,
                        "minecraft:block.stone.break",
                        1.0,
                        1.0,
                    );
                    return EventOutcome::Consume;
                }
            } else {
                // Normal & Angel extension
                let candidates =
                    calculate_surface_positions((event.x, event.y, event.z), (0, 1, 0), limit);
                let mut placed = 0;
                for (cx, cy, cz) in candidates {
                    let pos = BlockPos {
                        x: cx,
                        y: cy,
                        z: cz,
                    };
                    let existing = server::get_block(&event.dimension, pos);
                    if existing.is_none() || existing.as_deref() == Some("minecraft:air") {
                        if server::set_block(&event.dimension, pos, &target_state).is_ok() {
                            placed += 1;
                        }
                    }
                }
                if placed > 0 {
                    chat::show(&format!("🧱 Construction Stick placed {placed} blocks."));
                    server::play_sound(
                        &event.dimension,
                        base_pos,
                        "minecraft:block.stone.place",
                        1.0,
                        1.0,
                    );
                    return EventOutcome::Consume;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ConstructionStick);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wand_tiers_and_modes() {
        assert_eq!(
            WandTier::from_item("constructionstick:wooden_stick"),
            WandTier::Wooden
        );
        assert_eq!(
            WandTier::from_item("constructionstick:wooden_stick").max_blocks(),
            9
        );

        assert_eq!(
            WandTier::from_item("constructionstick:iron_stick"),
            WandTier::Iron
        );
        assert_eq!(
            WandTier::from_item("constructionstick:iron_stick").max_blocks(),
            32
        );

        assert_eq!(
            WandTier::from_item("constructionstick:netherite_stick_angel"),
            WandTier::Netherite
        );
        assert_eq!(
            WandTier::from_item("constructionstick:netherite_stick_angel").max_blocks(),
            128
        );

        assert_eq!(
            WandMode::from_item("constructionstick:diamond_stick_destruction"),
            WandMode::Destruction
        );
        assert_eq!(
            WandMode::from_item("constructionstick:copper_stick_angel"),
            WandMode::Angel
        );
        assert_eq!(
            WandMode::from_item("constructionstick:wooden_stick"),
            WandMode::Normal
        );
    }

    #[test]
    fn test_surface_positions_calculation() {
        // Y-normal (clicking top face of 0, 64, 0)
        let positions = calculate_surface_positions((0, 64, 0), (0, 1, 0), 9);
        assert_eq!(positions.len(), 9);
        // All placed blocks should be at Y = 65
        for &(_x, y, _z) in &positions {
            assert_eq!(y, 65);
        }

        // X-normal (clicking East face of 10, 64, 10)
        let positions_x = calculate_surface_positions((10, 64, 10), (1, 0, 0), 4);
        assert_eq!(positions_x.len(), 4);
        for &(x, _y, _z) in &positions_x {
            assert_eq!(x, 11);
        }
    }
}

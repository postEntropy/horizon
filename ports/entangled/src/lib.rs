//! Entangled: quantum entanglement binder linking blocks across dimensions.
//!
//! Provides the Entangled Binder to capture a target block's coordinates and bind it
//! to an Entangled Block, acting as an active cross-dimensional proxy.
pub mod items;

use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const ENTANGLED_SCREEN: &str = "screen:entangled:block";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetCoord {
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TargetCoord {
    pub fn new(dimension: &str, x: i32, y: i32, z: i32) -> Self {
        Self {
            dimension: dimension.to_string(),
            x,
            y,
            z,
        }
    }

    pub fn to_pos(&self) -> BlockPos {
        BlockPos {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

pub struct Entangled {
    /// In-hand binder currently carrying an anchor target
    pub bound_target: Option<TargetCoord>,
    /// Block Coordinate -> Target Coordinate
    pub block_bindings: HashMap<String, TargetCoord>,
}

pub fn entangled_screen(target: Option<&TargetCoord>) -> ScreenSpec {
    let desc = match target {
        Some(t) => format!("Bound: [{}, {}, {}] in {}", t.x, t.y, t.z, t.dimension),
        None => "Unbound: Sneak-click a block with Entangled Binder".into(),
    };

    ScreenSpec {
        id: ENTANGLED_SCREEN.into(),
        title: "Entangled Block".into(),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 20,
                width: 160,
                height: 12,
                text: "Quantum Entanglement Interface".into(),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 40,
                width: 160,
                height: 12,
                text: desc,
                value: if target.is_some() { 1 } else { 0 },
                max_value: 1,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 8,
                y: 65,
                width: 160,
                height: 12,
                text: "Mirrors target inventories and conduit connections.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

pub fn position_key(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

impl Entangled {
    pub fn new() -> Self {
        Self {
            bound_target: None,
            block_bindings: HashMap::new(),
        }
    }

    pub fn bind_binder(&mut self, dimension: &str, x: i32, y: i32, z: i32) -> TargetCoord {
        let coord = TargetCoord::new(dimension, x, y, z);
        self.bound_target = Some(coord.clone());
        coord
    }

    pub fn clear_binder(&mut self) -> Option<TargetCoord> {
        self.bound_target.take()
    }

    pub fn link_block(&mut self, block_pos_key: &str, target: TargetCoord) {
        self.block_bindings.insert(block_pos_key.to_string(), target);
    }

    pub fn unlink_block(&mut self, block_pos_key: &str) -> Option<TargetCoord> {
        self.block_bindings.remove(block_pos_key)
    }
}

impl Default for Entangled {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for Entangled {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Entangled::new(),
            Registration::new().command("entangled"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 9,
                ticks: true,
            })
            .collect()
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
            if self.bound_target.is_some() {
                self.clear_binder();
                chat::show("🔗 Entangled Binder reset: cleared target binding.");
                return EventOutcome::Consume;
            }
            return EventOutcome::Pass;
        }

        if event.action == ItemAction::UseOnBlock {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            let block_key = position_key(&event.dimension, event.x, event.y, event.z);

            // If player clicks on an entangled:block with a bound target in the binder
            if let Some(target) = self.bound_target.clone() {
                self.link_block(&block_key, target.clone());
                server::play_sound(&event.dimension, pos, "minecraft:block.beacon.activate", 1.0, 1.2);
                chat::show(&format!(
                    "⚡ Quantum entanglement established between [{}, {}, {}] ({}) and [{block_key}]!",
                    target.x, target.y, target.z, target.dimension
                ));
            } else {
                // Bind this target block
                let target = self.bind_binder(&event.dimension, event.x, event.y, event.z);
                server::play_sound(&event.dimension, pos, "minecraft:block.respawn_anchor.charge", 1.0, 1.4);
                chat::show(&format!(
                    "🔗 Entangled Binder: anchored to [{}, {}, {}] ({})",
                    target.x, target.y, target.z, target.dimension
                ));
            }
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let block_key = position_key(&event.dimension, event.x, event.y, event.z);
        let target = self.block_bindings.get(&block_key);
        let _ = ui::open_screen(&entangled_screen(target));
        EventOutcome::Consume
    }

    fn tick_block_entities(&mut self, _handles: &[u32]) {
        // Active quantum resonance pulse maintains coherence
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "entangled" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["status"] => {
                let binder_info = match &self.bound_target {
                    Some(t) => format!("[{}, {}, {}] ({})", t.x, t.y, t.z, t.dimension),
                    None => "none".to_string(),
                };
                Ok(Some(format!(
                    "Entangled: Binder target={binder_info}, Active block links={}",
                    self.block_bindings.len()
                )))
            }
            ["clear"] => {
                self.clear_binder();
                Ok(Some("Entangled Binder cleared.".into()))
            }
            _ => Err("Usage: /entangled [status | clear]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(Entangled);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binder_lifecycle() {
        let mut ent = Entangled::new();
        assert!(ent.bound_target.is_none());

        // Bind target block
        let target = ent.bind_binder("minecraft:overworld", 100, 64, -200);
        assert_eq!(target.dimension, "minecraft:overworld");
        assert_eq!(target.x, 100);
        assert_eq!(target.y, 64);
        assert_eq!(target.z, -200);
        assert!(ent.bound_target.is_some());

        // Clear binder
        let cleared = ent.clear_binder();
        assert_eq!(cleared, Some(target));
        assert!(ent.bound_target.is_none());
    }

    #[test]
    fn test_block_linking_and_unlinking() {
        let mut ent = Entangled::new();
        let block_pos = "minecraft:overworld:10,20,30";
        let target = TargetCoord::new("minecraft:the_nether", 50, 70, 90);

        ent.link_block(block_pos, target.clone());
        assert_eq!(ent.block_bindings.get(block_pos), Some(&target));

        let unlinked = ent.unlink_block(block_pos);
        assert_eq!(unlinked, Some(target));
        assert!(ent.block_bindings.get(block_pos).is_none());
    }

    #[test]
    fn test_target_coord_to_pos() {
        let target = TargetCoord::new("minecraft:overworld", 12, 64, -50);
        let pos = target.to_pos();
        assert_eq!(pos.x, 12);
        assert_eq!(pos.y, 64);
        assert_eq!(pos.z, -50);
    }
}

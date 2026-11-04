//! Create's first StoneBlock path: a hand crank turns a cogwheel and drives a
//! Millstone. The host currently does not expose Minecraft's Level RNG to
//! machine callbacks, so chance outputs are registered exactly but processing
//! A crank click starts one bounded 100-tick stroke because a held client
//! button isn't available on the authoritative server-thread Game. This is a
//! repeatable quest path, not a claim of full Create hand-crank timing parity.
//! Chance outputs pause at recipe completion until Level RNG is available.
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    item_transfer::{self, Face},
    machine, recipes,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};
use std::collections::HashMap;

const HAND_CRANK: &str = "create:hand_crank";
const COGWHEEL: &str = "create:cogwheel";
const MILLSTONE: &str = "create:millstone";
const MILLING_KIND: &str = "create:milling";
const MILLING_TICKS: u32 = 100;
const CRANK_STROKE_TICKS: u32 = MILLING_TICKS;
const MILLSTONE_SCREEN: &str = "screen:create:millstone";

#[derive(Clone)]
struct MachineState {
    block: String,
    dimension: String,
    x: i32,
    y: i32,
    z: i32,
    progress: u32,
    powered: bool,
}

#[derive(Clone)]
struct CrankStroke {
    dimension: String,
    x: i32,
    y: i32,
    z: i32,
    remaining_ticks: u32,
}

#[derive(Default)]
pub struct CreateMod {
    machines: HashMap<u32, MachineState>,
    active_crank: Option<CrankStroke>,
}

fn stoneblock_milling_recipe() -> recipes::RecipeEntry {
    recipes::RecipeEntry {
        id: "create:stoneblock/gravel".into(),
        kind: MILLING_KIND.into(),
        inputs: vec![recipes::IngredientSlot {
            items: vec!["#forge:gravel".into()],
            count: 1,
        }],
        outputs: vec![
            "minecraft:coal*1@0.35".into(),
            "minecraft:raw_iron*1@0.35".into(),
            "create:raw_zinc*1@0.2".into(),
            "minecraft:sand*1@0.4".into(),
        ],
        ticks: MILLING_TICKS,
        energy: 0,
    }
}

fn register_stoneblock_milling_recipe() -> Result<(), String> {
    recipes::register(&stoneblock_milling_recipe())
}

fn milling_recipe_is_available() -> bool {
    recipes::get("create:stoneblock/gravel").is_some_and(|recipe| {
        recipe.kind == MILLING_KIND
            && recipe.inputs.len() == 1
            && recipe.inputs[0]
                .items
                .iter()
                .any(|item| item == "#forge:gravel")
            && recipe.inputs[0].count == 1
            && recipe.outputs
                == [
                    "minecraft:coal*1@0.35",
                    "minecraft:raw_iron*1@0.35",
                    "create:raw_zinc*1@0.2",
                    "minecraft:sand*1@0.4",
                ]
    })
}

impl CreateMod {
    fn block_entity_types() -> Vec<BlockEntityType> {
        vec![
            BlockEntityType {
                block: HAND_CRANK.into(),
                slots: 0,
                ticks: true,
            },
            BlockEntityType {
                block: COGWHEEL.into(),
                slots: 0,
                ticks: true,
            },
            BlockEntityType {
                block: MILLSTONE.into(),
                slots: 5,
                ticks: true,
            },
        ]
    }

    fn is_powered_millstone(&self, handle: u32) -> bool {
        let Some(millstone) = self.machines.get(&handle) else {
            return false;
        };
        if millstone.block != MILLSTONE {
            return false;
        }

        for neighbor in machine::get_adjacent_block_entities(handle) {
            let Some(gear) = self.machines.get(&neighbor.handle) else {
                continue;
            };
            // The StoneBlock Ponder builds the vertical hand-crank/cogwheel
            // axis beside the Millstone. Only that direct mechanical layout
            // transmits power in this initial Create slice.
            if gear.block != COGWHEEL
                || neighbor.face != machine::Face::West
                || machine::get_property(neighbor.handle, "axis").as_deref() != Some("y")
            {
                continue;
            }
            for gear_neighbor in machine::get_adjacent_block_entities(neighbor.handle) {
                let Some(crank) = self.machines.get(&gear_neighbor.handle) else {
                    continue;
                };
                if crank.block != HAND_CRANK
                    || gear_neighbor.face != machine::Face::Up
                    || machine::get_property(gear_neighbor.handle, "facing").as_deref()
                        != Some("up")
                {
                    continue;
                }
                if self.active_crank.as_ref().is_some_and(|stroke| {
                    stroke.remaining_ticks > 0
                        && stroke.dimension == crank.dimension
                        && stroke.x == crank.x
                        && stroke.y == crank.y
                        && stroke.z == crank.z
                }) {
                    return true;
                }
            }
        }
        false
    }

    fn open_millstone_screen(&self, progress: u32, powered: bool) {
        let status = if !powered {
            "No rotation"
        } else if progress >= MILLING_TICKS {
            "Awaiting host RNG for chance output"
        } else {
            "Rotating at 10 RPM"
        };
        let _ = ui::open_screen(&ScreenSpec {
            id: MILLSTONE_SCREEN.into(),
            title: "Millstone".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 8,
                    width: 160,
                    height: 16,
                    text: "Hand crank → cogwheel → Millstone".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::ProgressArrow,
                    x: 76,
                    y: 42,
                    width: 24,
                    height: 17,
                    text: status.into(),
                    value: u64::from(progress),
                    max_value: u64::from(MILLING_TICKS),
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 72,
                    width: 160,
                    height: 16,
                    text: "Use a hopper for item input/output.".into(),
                    value: 0,
                    max_value: 0,
                },
            ],
        });
    }
}

impl Mod for CreateMod {
    fn init() -> Result<(Self, Registration), String> {
        register_stoneblock_milling_recipe()?;
        Ok((Self::default(), Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        Self::block_entity_types()
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().block(HAND_CRANK).block(MILLSTONE)
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let Some(position) = machine::get_position(handle) else {
            return;
        };
        let progress = saved
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
            .min(MILLING_TICKS);
        self.machines.insert(
            handle,
            MachineState {
                block: block.to_owned(),
                dimension: position.dimension,
                x: position.x,
                y: position.y,
                z: position.z,
                progress,
                powered: false,
            },
        );
        if block == MILLSTONE {
            for face in [
                Face::Down,
                Face::Up,
                Face::North,
                Face::South,
                Face::West,
                Face::East,
            ] {
                let _ = item_transfer::set_slot_access(handle, 0, face, face == Face::Up, false);
                for slot in 1..5 {
                    let _ = item_transfer::set_slot_access(
                        handle,
                        slot,
                        face,
                        false,
                        face == Face::Down,
                    );
                }
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        let powered: HashMap<u32, bool> = handles
            .iter()
            .copied()
            .filter(|handle| {
                self.machines
                    .get(handle)
                    .is_some_and(|state| state.block == MILLSTONE)
            })
            .map(|handle| (handle, self.is_powered_millstone(handle)))
            .collect();

        for &handle in handles {
            let Some(state) = self.machines.get_mut(&handle) else {
                continue;
            };
            let is_powered = powered.get(&handle).copied().unwrap_or(false);
            state.powered = is_powered;
            if state.block == MILLSTONE {
                let has_gravel = machine::get_slot(handle, 0)
                    .is_some_and(|stack| stack.id == "minecraft:gravel" && stack.count > 0);
                if is_powered && has_gravel && milling_recipe_is_available() {
                    state.progress = state.progress.saturating_add(1).min(MILLING_TICKS);
                }
                let status = if !is_powered {
                    "No rotation"
                } else if state.progress >= MILLING_TICKS {
                    "Awaiting host RNG for chance output"
                } else {
                    "Rotating at 10 RPM"
                };
                let _ = ui::set_gauge(
                    MILLSTONE_SCREEN,
                    1,
                    u64::from(state.progress),
                    u64::from(MILLING_TICKS),
                    status.into(),
                );
            }
        }

        if let Some(stroke) = self.active_crank.as_mut() {
            stroke.remaining_ticks = stroke.remaining_ticks.saturating_sub(1);
            if stroke.remaining_ticks == 0 {
                self.active_crank = None;
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.machines
            .get(&handle)
            .map(|state| state.progress.to_le_bytes().to_vec())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        let Some(state) = self.machines.remove(&handle) else {
            return;
        };
        if state.block == HAND_CRANK
            && self.active_crank.as_ref().is_some_and(|stroke| {
                stroke.dimension == state.dimension
                    && stroke.x == state.x
                    && stroke.y == state.y
                    && stroke.z == state.z
            })
        {
            self.active_crank = None;
        }
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        self.machines
            .get(&handle)
            .map(|state| vec![state.progress as i32, MILLING_TICKS as i32])
            .unwrap_or_default()
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if !matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            return EventOutcome::Pass;
        }
        if event.state.starts_with(HAND_CRANK) {
            self.active_crank = Some(CrankStroke {
                dimension: event.dimension,
                x: event.x,
                y: event.y,
                z: event.z,
                remaining_ticks: CRANK_STROKE_TICKS,
            });
            return EventOutcome::Consume;
        }
        if event.state.starts_with(MILLSTONE) {
            let state = self.machines.values().find(|state| {
                state.block == MILLSTONE
                    && state.dimension == event.dimension
                    && state.x == event.x
                    && state.y == event.y
                    && state.z == event.z
            });
            let (progress, powered) = state
                .map(|state| (state.progress, state.powered))
                .unwrap_or((0, false));
            self.open_millstone_screen(progress, powered);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(CreateMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stoneblock_milling_recipe_preserves_pack_probabilities() {
        // This is the exact KubeJS recipe. The host recipe adapter currently
        // exposes these chances but the runtime has no Level-RNG bridge yet.
        let recipe = stoneblock_milling_recipe();
        assert_eq!(recipe.id, "create:stoneblock/gravel");
        assert_eq!(recipe.kind, MILLING_KIND);
        assert_eq!(recipe.inputs[0].items, ["#forge:gravel"]);
        assert_eq!(recipe.inputs[0].count, 1);
        assert_eq!(
            recipe.outputs,
            [
                "minecraft:coal*1@0.35",
                "minecraft:raw_iron*1@0.35",
                "create:raw_zinc*1@0.2",
                "minecraft:sand*1@0.4",
            ]
        );
        assert_eq!(recipe.ticks, 100);
    }
}

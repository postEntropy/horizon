//! Farmer's Delight: agricultural cuisine, cooking pots, cutting boards, and feasts.
mod cooking_items;
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration,
    block_state::BlockState,
    chat,
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    machine_slot_limits,
    recipes::{self, IngredientSlot, RecipeEntry},
    server::{self, BlockPos},
    status_effects,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const COOKING_POT_COOK_TIME: u32 = 200;

fn dog_food_interaction(event: &ItemEvent) -> EventOutcome {
    if event.item == "farmersdelight:dog_food"
        && event.action == ItemAction::InteractEntity
        && event.target.starts_with("minecraft:wolf;")
        && event.target.contains("tamed=true")
        && event.target.contains("owner_is_player=true")
    {
        EventOutcome::Consume
    } else {
        EventOutcome::Pass
    }
}

fn pot_is_heated(handle: u32) -> bool {
    let Some(position) = machine::get_position(handle) else {
        return false;
    };
    let Some(y) = position.y.checked_sub(1) else {
        return false;
    };
    let below = BlockPos {
        x: position.x,
        y,
        z: position.z,
    };
    let Some(mut text) = server::get_block(&position.dimension, below) else {
        return false;
    };
    let Some(state) = BlockState::parse(&text) else {
        return false;
    };
    if machine::block_in_tag(state.id(), "farmersdelight:heat_conductors") {
        let Some(y) = y.checked_sub(1) else {
            return false;
        };
        let Some(source) = server::get_block(&position.dimension, BlockPos { y, ..below }) else {
            return false;
        };
        text = source;
    }
    let Some(state) = BlockState::parse(&text) else {
        return false;
    };
    machine::block_in_tag(state.id(), "farmersdelight:heat_sources")
        && state.property("lit").is_none_or(|lit| lit == "true")
}

#[derive(Clone, Debug)]
pub struct CookingRecipe {
    pub ingredients: Vec<IngredientSlot>,
    pub result: ItemStack,
    pub cook_time: u32,
}

impl CookingRecipe {
    fn from_entry(entry: RecipeEntry) -> Option<Self> {
        if entry.inputs.is_empty() || entry.inputs.len() > 6 || entry.outputs.len() != 1 {
            return None;
        }
        // The loaded cooking graph expands item tags into concrete ids.
        if entry.inputs.iter().any(|input| {
            input.count == 0
                || input.items.is_empty()
                || input
                    .items
                    .iter()
                    .any(|id| id.starts_with('#') || id.starts_with('@'))
        }) {
            return None;
        }
        let (id, count) = entry.outputs[0].split_once('*')?;
        let count = count.parse::<u32>().ok()?;
        let (_max_stack, _container) = cooking_items::result_facts(id)?;
        if count == 0 || count > 64 {
            return None;
        }
        Some(Self {
            ingredients: entry.inputs,
            result: ItemStack {
                id: id.into(),
                count,
            },
            cook_time: entry.ticks.max(1),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CookingPotState {
    pub cook_time: u32,
    pub cook_time_total: u32,
}

impl CookingPotState {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8);
        out.extend_from_slice(&self.cook_time.to_le_bytes());
        out.extend_from_slice(&self.cook_time_total.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut state = Self::default();
        if bytes.len() >= 8 {
            state.cook_time = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            state.cook_time_total = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        }
        state
    }

    /// Progresses cooking by 1 tick if ingredients match and heated.
    pub fn tick_step(&mut self, is_heated: bool, has_recipe: bool, ticks: u32) -> bool {
        if is_heated && has_recipe {
            self.cook_time_total = ticks.max(1);
            self.cook_time = self.cook_time.saturating_add(1);
            if self.cook_time >= self.cook_time_total {
                self.cook_time = 0;
                return true; // Finished cooking
            }
        } else if self.cook_time > 0 {
            self.cook_time = self.cook_time.saturating_sub(2);
        }
        false
    }
}

pub fn cooking_pot_screen(state: &CookingPotState) -> ScreenSpec {
    let mut widgets = Vec::new();
    // Title
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: "Cooking Pot".into(),
        value: 0,
        max_value: 0,
    });

    // 6 Meal Ingredient Slots (2 rows of 3)
    for row in 0..2 {
        for col in 0..3 {
            let id = (row * 3 + col) as u32;
            widgets.push(Widget {
                id,
                kind: WidgetKind::Slot,
                x: 44 + col * 18,
                y: 18 + row * 18,
                width: 18,
                height: 18,
                text: format!("Ingredient {}", id + 1),
                value: 0,
                max_value: 64,
            });
        }
    }

    for (id, x, y, text) in [
        (6, 124, 18, "Stored Meal"),
        (7, 92, 55, "Bowl / Container"),
        (8, 124, 55, "Served Meal"),
    ] {
        widgets.push(Widget {
            id,
            kind: WidgetKind::Slot,
            x,
            y,
            width: 18,
            height: 18,
            text: text.into(),
            value: 0,
            max_value: 64,
        });
    }

    // Progress Bar
    widgets.push(Widget {
        id: 101,
        kind: WidgetKind::Flame,
        x: 100,
        y: 35,
        width: 14,
        height: 14,
        text: "Cook Progress".into(),
        value: state.cook_time as u64,
        max_value: state.cook_time_total.max(1) as u64,
    });

    ScreenSpec {
        id: "screen:farmersdelight:cooking_pot".into(),
        title: "Cooking Pot".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub fn cabinet_screen(title: &str) -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: title.into(),
        value: 0,
        max_value: 0,
    });

    // 27 slots (3 rows of 9)
    for row in 0..3 {
        for col in 0..9 {
            let id = (row * 9 + col) as u32;
            widgets.push(Widget {
                id,
                kind: WidgetKind::Slot,
                x: 8 + col * 18,
                y: 18 + row * 18,
                width: 18,
                height: 18,
                text: format!("Slot {}", id + 1),
                value: 0,
                max_value: 64,
            });
        }
    }

    ScreenSpec {
        id: "screen:farmersdelight:cabinet".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub struct FarmersDelight {
    pub pots: HashMap<u32, CookingPotState>,
    recipe_revision: Option<u64>,
    recipes: Vec<CookingRecipe>,
    nourishment_food: HashMap<String, (u64, u32, f32)>,
}

/// Match each occupied slot to a distinct ingredient. Augmenting paths handle
/// overlapping alternatives without depending on recipe or inventory order.
pub fn matches_recipe(inputs: &[Option<ItemStack>], recipe: &CookingRecipe) -> bool {
    let stacks: Vec<_> = inputs.iter().flatten().collect();
    if stacks.len() != recipe.ingredients.len() {
        return false;
    }
    fn assign(
        slot: usize,
        stacks: &[&ItemStack],
        ingredients: &[IngredientSlot],
        owners: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        for (i, ingredient) in ingredients.iter().enumerate() {
            if seen[i]
                || stacks[slot].count < ingredient.count
                || !ingredient.items.contains(&stacks[slot].id)
            {
                continue;
            }
            seen[i] = true;
            if owners[i].is_none_or(|other| assign(other, stacks, ingredients, owners, seen)) {
                owners[i] = Some(slot);
                return true;
            }
        }
        false
    }
    let mut owners = vec![None; recipe.ingredients.len()];
    for slot in 0..stacks.len() {
        if !assign(
            slot,
            &stacks,
            &recipe.ingredients,
            &mut owners,
            &mut vec![false; recipe.ingredients.len()],
        ) {
            return false;
        }
    }
    true
}

fn serve_meal(handle: u32) {
    let Some(meal) = machine::get_slot(handle, 6) else {
        return;
    };
    let Some((max_stack, container)) = cooking_items::result_facts(&meal.id) else {
        return;
    };
    let output = machine::get_slot(handle, 8);
    if output.as_ref().is_some_and(|stack| stack.id != meal.id) {
        return;
    }
    let mut amount = meal
        .count
        .min(max_stack.saturating_sub(output.map_or(0, |stack| stack.count)));
    let mut consumed = Vec::new();
    if let Some(id) = container {
        let Some(stack) = machine::get_slot(handle, 7).filter(|stack| stack.id == id) else {
            return;
        };
        amount = amount.min(stack.count);
        consumed.push(machine::SlotStack {
            slot: 7,
            stack: ItemStack {
                id: id.into(),
                count: amount,
            },
        });
    }
    if amount == 0 {
        return;
    }
    consumed.push(machine::SlotStack {
        slot: 6,
        stack: ItemStack {
            id: meal.id.clone(),
            count: amount,
        },
    });
    let produced = [machine::SlotStack {
        slot: 8,
        stack: ItemStack {
            id: meal.id,
            count: amount,
        },
    }];
    let _ = machine::process_items(handle, &consumed, &produced, false);
}

impl Mod for FarmersDelight {
    fn init() -> Result<(Self, Registration), String> {
        status_effects::register("farmersdelight:nourishment")?;
        Ok((
            FarmersDelight {
                pots: HashMap::new(),
                recipe_revision: None,
                recipes: Vec::new(),
                nourishment_food: HashMap::new(),
            },
            Registration::new().on_tick(),
        ))
    }

    fn tick(&mut self, tick: u64) {
        self.nourishment_food
            .retain(|_, (last_tick, _, _)| tick.saturating_sub(*last_tick) <= 1_200);
        let player = status_effects::player_id();
        let active = status_effects::active();
        let Some(effect) = active
            .iter()
            .find(|effect| effect.id == "farmersdelight:nourishment")
        else {
            self.nourishment_food.remove(&player);
            return;
        };
        let current = (effect.food, effect.saturation);
        let Some((_, previous_food, previous_saturation)) =
            self.nourishment_food.get(&player).copied()
        else {
            self.nourishment_food
                .insert(player, (tick, current.0, current.1));
            return;
        };
        let restored = (
            current.0.max(previous_food),
            current.1.max(previous_saturation),
        );
        if restored != current {
            let _ = status_effects::set_food("farmersdelight:nourishment", restored.0, restored.1);
        }
        self.nourishment_food
            .insert(player, (tick, restored.0, restored.1));
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        let mut types = vec![
            BlockEntityType {
                block: "farmersdelight:cooking_pot".into(),
                slots: 9,
                ticks: true,
            },
            BlockEntityType {
                block: "farmersdelight:skillet".into(),
                slots: 1,
                ticks: true,
            },
            BlockEntityType {
                block: "farmersdelight:cutting_board".into(),
                slots: 1,
                ticks: false,
            },
            BlockEntityType {
                block: "farmersdelight:stove".into(),
                slots: 6,
                ticks: true,
            },
        ];

        // Cabinets and Baskets: 27 slots, non-ticking
        for &block in items::BLOCKS {
            if block.contains("cabinet") || block.contains("basket") {
                types.push(BlockEntityType {
                    block: block.into(),
                    slots: 27,
                    ticks: false,
                });
            }
        }

        types
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        if block.contains("cooking_pot") {
            let state = CookingPotState::decode(saved);
            self.pots.insert(handle, state);

            // Meals are kept at 64 regardless of the served item's stack limit.
            let _ = machine_slot_limits::set_slot_limit(handle, 6, 64);
            let _ = machine::set_player_slot_access(handle, 6, false, false);
            let _ = machine::set_player_slot_access(handle, 8, false, true);
            for slot in 0..9 {
                for face in [
                    Face::Up,
                    Face::Down,
                    Face::North,
                    Face::South,
                    Face::East,
                    Face::West,
                ] {
                    let _ = item_transfer::set_slot_access(handle, slot, face, false, false);
                }
            }
            for slot in 0..6 {
                let _ = item_transfer::set_slot_access(handle, slot, Face::Up, true, false);
            }
            for face in [Face::North, Face::South, Face::East, Face::West] {
                let _ = item_transfer::set_slot_access(handle, 7, face, true, false);
            }
            let _ = item_transfer::set_slot_access(handle, 8, Face::Down, false, true);
        } else if block.contains("cabinet") || block.contains("basket") {
            for slot in 0..27 {
                for face in [
                    Face::Up,
                    Face::Down,
                    Face::North,
                    Face::South,
                    Face::East,
                    Face::West,
                ] {
                    let _ = item_transfer::set_slot_access(handle, slot, face, true, true);
                }
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        if !handles.iter().any(|handle| self.pots.contains_key(handle)) {
            return;
        }
        let revision = recipes::revision();
        if self.recipe_revision != Some(revision) {
            self.recipes = recipes::by_type("farmersdelight:cooking")
                .into_iter()
                .filter_map(CookingRecipe::from_entry)
                // Cooking ingredients consume one item per occupied slot.
                .filter(|recipe| recipe.ingredients.iter().all(|input| input.count == 1))
                .collect();
            self.recipe_revision = Some(revision);
        }
        for &handle in handles {
            let Some(state) = self.pots.get_mut(&handle) else {
                continue;
            };

            // Read 6 ingredient slots
            let mut inputs = Vec::new();
            for slot in 0..6 {
                inputs.push(machine::get_slot(handle, slot));
            }

            let matching_recipe = self
                .recipes
                .iter()
                .find(|recipe| matches_recipe(&inputs, recipe));

            let mut consumed = Vec::new();
            let mut produced = Vec::new();
            if let Some(recipe) = matching_recipe {
                for (slot, stack) in inputs.iter().enumerate() {
                    if let Some(stack) = stack {
                        consumed.push(machine::SlotStack {
                            slot: slot as u32,
                            stack: ItemStack {
                                id: stack.id.clone(),
                                count: 1,
                            },
                        });
                    }
                }
                produced.push(machine::SlotStack {
                    slot: 6,
                    stack: recipe.result.clone(),
                });
            }
            let possible = matching_recipe.is_some()
                && machine::process_items(handle, &consumed, &produced, true);
            let mut next = *state;
            let complete = next.tick_step(
                pot_is_heated(handle),
                possible,
                matching_recipe.map_or(200, |r| r.cook_time),
            );
            if !complete || machine::process_items(handle, &consumed, &produced, false) {
                *state = next;
            }
            // Serving does not require heat or ingredients and may empty an old
            // meal even when the next recipe cannot fit in the reservoir.
            serve_meal(handle);
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.pots
            .get(&handle)
            .map(|p| p.encode())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.pots.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        if let Some(state) = self.pots.get(&handle) {
            vec![state.cook_time as i32, state.cook_time_total as i32]
        } else {
            Vec::new()
        }
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("cooking_pot") {
                server::play_sound(
                    &event.dimension,
                    pos,
                    "minecraft:block.bubble_column.bubble_pop",
                    1.0,
                    1.0,
                );
                let state = CookingPotState::default();
                let _ = ui::open_screen(&cooking_pot_screen(&state));
                return EventOutcome::Consume;
            } else if event.state.contains("cutting_board") {
                server::play_sound(&event.dimension, pos, "minecraft:block.wood.hit", 1.0, 1.2);
                chat::show("🔪 Slicing ingredients on cutting board.");
                return EventOutcome::Consume;
            } else if event.state.contains("cabinet") || event.state.contains("basket") {
                let _ = ui::open_screen(&cabinet_screen("Cabinet"));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        // SUCCESS without healing the wolf or consuming Dog Food.
        dog_food_interaction(&event)
    }
}

horizon_mod_sdk::export_mod!(FarmersDelight);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dog_food_interaction_matches_observed_pass_and_success_results() {
        let event = |target: &str| ItemEvent {
            action: ItemAction::InteractEntity,
            item: "farmersdelight:dog_food".into(),
            player: "player".into(),
            dimension: "minecraft:overworld".into(),
            x: 0,
            y: 0,
            z: 0,
            target: target.into(),
        };
        assert_eq!(
            dog_food_interaction(&event("minecraft:wolf;tamed=true;owner_is_player=true")),
            EventOutcome::Consume
        );
        assert_eq!(
            dog_food_interaction(&event("minecraft:wolf;tamed=false;owner_is_player=false")),
            EventOutcome::Pass
        );
        assert_eq!(
            dog_food_interaction(&event("minecraft:wolf;tamed=true;owner_is_player=false")),
            EventOutcome::Pass
        );
    }

    #[test]
    fn test_cooking_pot_state_encode_decode() {
        let state = CookingPotState {
            cook_time: 150,
            cook_time_total: 200,
        };
        let encoded = state.encode();
        assert_eq!(encoded.len(), 8);
        let decoded = CookingPotState::decode(&encoded);
        assert_eq!(state, decoded);
    }

    #[test]
    fn test_cooking_pot_tick_step() {
        let mut state = CookingPotState::default();
        // Tick without recipe does nothing
        assert!(!state.tick_step(true, false, 200));
        assert_eq!(state.cook_time, 0);

        // Tick with recipe progresses
        for _ in 0..199 {
            assert!(!state.tick_step(true, true, 200));
        }
        assert_eq!(state.cook_time, 199);

        // Tick 200 completes cooking
        assert!(state.tick_step(true, true, 200));
        assert_eq!(state.cook_time, 0);
    }

    #[test]
    fn overlapping_ingredients_match_in_either_order() {
        let recipe = CookingRecipe {
            ingredients: vec![
                IngredientSlot {
                    items: vec!["a".into(), "b".into()],
                    count: 1,
                },
                IngredientSlot {
                    items: vec!["a".into()],
                    count: 1,
                },
            ],
            result: ItemStack {
                id: "meal".into(),
                count: 2,
            },
            cook_time: 100,
        };
        let stack = |id: &str| {
            Some(ItemStack {
                id: id.into(),
                count: 1,
            })
        };
        assert!(matches_recipe(&[stack("a"), stack("b")], &recipe));
        assert!(matches_recipe(&[stack("b"), stack("a")], &recipe));
        assert!(!matches_recipe(&[stack("b"), stack("b")], &recipe));
        assert!(!matches_recipe(
            &[stack("a"), stack("b"), stack("c")],
            &recipe
        ));
        assert!(!matches_recipe(&[stack("a")], &recipe));
        assert!(!matches_recipe(
            &[
                Some(ItemStack {
                    id: "a".into(),
                    count: 0
                }),
                stack("b")
            ],
            &recipe
        ));
    }

    #[test]
    fn recipe_duration_controls_progress() {
        let mut state = CookingPotState::default();
        for _ in 0..99 {
            assert!(!state.tick_step(true, true, 100));
        }
        assert!(state.tick_step(true, true, 100));
        assert_eq!(state.cook_time_total, 100);
    }
}

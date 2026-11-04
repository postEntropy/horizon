//! Crafting and cooking recipes from the pinned data pack: shaped patterns
//! (`ShapedRecipePattern.matches`), shapeless recipes (a bipartite match of
//! ingredients to stacks, as `StackedContents.canCraft`), the special and
//! transmuting crafting recipes (`CustomRecipe` and friends), and
//! furnace-type cooking lookups.
use crate::{
    Result,
    item_data::StackData,
    items::{ItemStack, Items},
    particles::{DYE_FIREWORK_COLORS, FireworkExplosion, FireworkShape},
};
use horizon_world::nbt::Tag;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

/// A set of accepted items (expanded from an item, list or tag), and the
/// items in its `HolderSet` order (`Ingredient.display`'s `TagSlotDisplay`).
#[derive(Clone, Debug)]
pub struct Ingredient(pub HashSet<String>, pub Vec<String>);
impl Ingredient {
    /// The items of a list, in order.
    pub fn of(items: Vec<String>) -> Self {
        Self(items.iter().cloned().collect(), items)
    }
    fn parse(v: &Value, items: &Items) -> Option<Self> {
        let mut list: Vec<String> = Vec::new();
        fn collect(v: &Value, items: &Items, list: &mut Vec<String>) {
            let mut add = |s: &str| {
                let more: Vec<String> = match s.strip_prefix('#') {
                    Some(tag) => items.item_tag_order.get(tag).cloned().unwrap_or_default(),
                    None => vec![s.to_owned()],
                };
                for i in more {
                    if !list.contains(&i) {
                        list.push(i);
                    }
                }
            };
            match v {
                Value::String(s) => add(s),
                Value::Array(a) => {
                    for elem in a {
                        collect(elem, items, list);
                    }
                }
                Value::Object(o) => {
                    if let Some(s) = o.get("item").and_then(Value::as_str) {
                        add(s);
                    }
                    if let Some(t) = o.get("tag").and_then(Value::as_str) {
                        add(&format!("#{t}"));
                    }
                    if let Some(inner) = o.get("items") {
                        collect(inner, items, list);
                    }
                    if let Some(inner) = o.get("children") {
                        collect(inner, items, list);
                    }
                    if let Some(inner) = o.get("base") {
                        collect(inner, items, list);
                    }
                }
                _ => {}
            }
        }
        collect(v, items, &mut list);
        (!list.is_empty()).then(|| Self::of(list))
    }
    pub fn test(&self, stack: &ItemStack) -> bool {
        self.0.contains(&*stack.item)
    }
}
/// Expands the loader ingredient forms used by cooking-pot data. Unknown
/// predicates fail closed instead of silently treating their base as a match.
fn pot_ingredient(
    value: &Value,
    resolve: &impl Fn(&str) -> Vec<String>,
    depth: usize,
) -> Option<Ingredient> {
    if depth > 32 {
        return None;
    }
    let combine = |children: &[Value]| -> Option<Ingredient> {
        let mut all = Vec::new();
        for child in children {
            for id in pot_ingredient(child, resolve, depth + 1)?.1 {
                if !all.contains(&id) {
                    all.push(id);
                }
            }
        }
        Some(Ingredient::of(all))
    };
    match value {
        Value::String(id) => Some(Ingredient::of(resolve(id))),
        Value::Array(children) => combine(children),
        Value::Object(object) => {
            let kind = object
                .get("type")
                .or_else(|| object.get("neoforge:ingredient_type"));
            let kind = match kind {
                Some(value) => Some(value.as_str()?),
                None => None,
            };
            let allowed: &[&str] = match kind {
                Some("neoforge:compound") => &["type", "neoforge:ingredient_type", "children"],
                Some("neoforge:difference") => {
                    &["type", "neoforge:ingredient_type", "base", "subtracted"]
                }
                None => &["item", "tag"],
                _ => return None,
            };
            if object.keys().any(|key| !allowed.contains(&key.as_str()))
                || (object.contains_key("type") && object.contains_key("neoforge:ingredient_type"))
                || (object.contains_key("item") && object.contains_key("tag"))
            {
                return None;
            }
            match kind {
                Some("neoforge:compound") => combine(object.get("children")?.as_array()?),
                Some("neoforge:difference") => {
                    let base = pot_ingredient(object.get("base")?, resolve, depth + 1)?;
                    let excluded = pot_ingredient(object.get("subtracted")?, resolve, depth + 1)?;
                    Some(Ingredient::of(
                        base.1
                            .into_iter()
                            .filter(|id| !excluded.0.contains(id))
                            .collect(),
                    ))
                }
                Some(_) => None,
                None => {
                    if let Some(id) = object.get("item").and_then(Value::as_str) {
                        Some(Ingredient::of(resolve(id)))
                    } else {
                        let tag = object.get("tag")?.as_str()?;
                        Some(Ingredient::of(resolve(&format!("#{tag}"))))
                    }
                }
            }
        }
        _ => None,
    }
}

/// `PlacementInfo`: the ingredients to find and, per recipe slot, the index
/// of its ingredient (`slotsToIngredientIndex`, -1 for an empty slot). Empty
/// when the recipe is impossible to place.
#[derive(Clone, Debug, Default)]
pub struct Placement {
    pub ingredients: Vec<Ingredient>,
    pub slots: Vec<i32>,
    /// A `ShapedRecipe`'s width and height (the others fill the grid).
    pub shaped: Option<(usize, usize)>,
}
impl Placement {
    /// `PlacementInfo.createFromOptionals` (and `create` with no empty slots).
    pub fn of(cells: Vec<Option<Ingredient>>, shaped: Option<(usize, usize)>) -> Self {
        let mut out = Self {
            shaped,
            ..Self::default()
        };
        for cell in cells {
            match cell {
                Some(i) if i.0.is_empty() => return Self::default(),
                Some(i) => {
                    out.slots.push(out.ingredients.len() as i32);
                    out.ingredients.push(i);
                }
                None => out.slots.push(-1),
            }
        }
        out
    }
    pub fn impossible(&self) -> bool {
        self.slots.is_empty()
    }
}
/// A recipe result: `ItemStackTemplate` (`id`, `count`, `components`), with the
/// id optional for the transmuting recipes that keep the input's item.
#[derive(Clone, Debug)]
pub struct Template {
    pub item: Option<Arc<str>>,
    pub count: u32,
    pub damage: u32,
    pub extra: Option<Box<StackData>>,
}
impl Template {
    fn parse(v: &Value) -> Option<Self> {
        if let Some(s) = v.as_str() {
            return Some(Self {
                item: Some(Arc::from(s)),
                count: 1,
                damage: 0,
                extra: None,
            });
        }
        let (damage, extra) = match v.get("components") {
            Some(c) => crate::item_data::decode_components(Some(&json_tag(c))),
            None => (0, None),
        };
        Some(Self {
            item: v["id"]
                .as_str()
                .or_else(|| v["item"].as_str())
                .map(Arc::from),
            count: v["count"].as_u64().unwrap_or(1) as u32,
            damage,
            extra,
        })
    }
    /// `ItemStackTemplate.create`.
    pub fn create(&self) -> ItemStack {
        ItemStack {
            item: self
                .item
                .clone()
                .unwrap_or_else(|| Arc::from("minecraft:air")),
            count: self.count,
            damage: self.damage,
            extra: self.extra.clone(),
        }
    }
    /// `TransmuteRecipe.createWithOriginalComponents`: the input's component
    /// patch on the result item, then the template's own components over it.
    fn over(&self, input: &ItemStack, extra_count: u32) -> ItemStack {
        let mut out = ItemStack {
            item: self.item.clone().unwrap_or_else(|| input.item.clone()),
            count: self.count + extra_count,
            damage: input.damage,
            extra: input.extra.clone(),
        };
        if self.damage > 0 {
            out.damage = self.damage;
        }
        if let Some(over) = &self.extra {
            overlay(out.data_mut(), over);
        }
        out.tidy();
        out
    }
}
/// `DataComponentPatch` application: every component the overlay sets wins.
fn overlay(dst: &mut StackData, src: &StackData) {
    let none = StackData::default();
    macro_rules! field {
        ($($f:ident),*) => {$(
            if src.$f != none.$f {
                dst.$f = src.$f.clone();
            }
        )*};
    }
    field!(
        enchantments,
        stored_enchantments,
        repair_cost,
        custom_name,
        potion,
        trim,
        container,
        charged,
        fireworks,
        map_id,
        writable_pages,
        written,
        banner_patterns,
        base_color,
        ominous_banner,
        ominous_amplifier,
        dyed_color,
        firework_explosion,
        pot_decorations,
        bundle_contents,
        stew_effects,
        map_post_processing
    );
}
/// A JSON recipe value as the NBT a component codec reads.
fn json_tag(v: &Value) -> Tag {
    match v {
        Value::Null => Tag::End,
        Value::Bool(b) => Tag::Byte(i8::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i32::try_from(i).map_or(Tag::Long(i), Tag::Int),
            None => Tag::Double(n.as_f64().unwrap_or(0.0).to_bits()),
        },
        Value::String(s) => Tag::String(s.as_str().into()),
        Value::Array(a) => Tag::List(a.iter().map(json_tag).collect()),
        Value::Object(o) => Tag::compound(o.iter().map(|(k, v)| (k.as_str(), json_tag(v)))),
    }
}
#[derive(Clone, Debug)]
pub enum Recipe {
    Shaped {
        width: usize,
        height: usize,
        cells: Vec<Option<Ingredient>>,
        result: Template,
    },
    Shapeless {
        ingredients: Vec<Ingredient>,
        result: Template,
    },
    /// The recipes with code of their own.
    Special(Special),
}
/// The custom and transmuting crafting recipes.
#[derive(Clone, Debug)]
pub enum Special {
    /// A patterned banner and a blank one of the same colour copy the patterns.
    BannerDuplicate { banner: Ingredient, result: String },
    /// A banner decorates an undecorated shield.
    ShieldDecoration {
        banner: Ingredient,
        target: Ingredient,
        result: String,
    },
    /// `TransmuteRecipe`: one input, 1..8 materials; shulker box and bundle
    /// dyeing, map cloning.
    Transmute {
        input: Ingredient,
        material: Ingredient,
        min: usize,
        max: usize,
        add_count: bool,
        result: Template,
    },
    /// `DyeRecipe`: dyed armour.
    Dye {
        target: Ingredient,
        dye: Ingredient,
        result: Template,
    },
    /// `ImbueRecipe`: tipped arrows.
    Imbue {
        source: Ingredient,
        material: Ingredient,
        result: Template,
    },
    /// `BookCloningRecipe`.
    BookCloning {
        source: Ingredient,
        material: Ingredient,
        min: i32,
        max: i32,
        result: Template,
    },
    /// `FireworkRocketRecipe`.
    FireworkRocket {
        shell: Ingredient,
        fuel: Ingredient,
        star: Ingredient,
        result: Template,
    },
    /// `FireworkStarRecipe`.
    FireworkStar {
        dye: Ingredient,
        fuel: Ingredient,
        trail: Ingredient,
        twinkle: Ingredient,
        shapes: Vec<(FireworkShape, Ingredient)>,
        result: Template,
    },
    /// `FireworkStarFadeRecipe`.
    FireworkStarFade {
        dye: Ingredient,
        target: Ingredient,
        result: Template,
    },
    /// `MapExtendingRecipe`: a map ringed by paper, when its scale is under 4.
    MapExtending {
        map: Ingredient,
        material: Ingredient,
        result: Template,
    },
    /// `DecoratedPotRecipe`: four sherds or bricks.
    DecoratedPot {
        back: Ingredient,
        left: Ingredient,
        right: Ingredient,
        front: Ingredient,
        result: Template,
    },
    /// `RepairItemRecipe`: two damaged items of one kind.
    RepairItem,
}
impl Special {
    /// The recipe-book displays of a learnable (non-custom) crafting recipe:
    /// its layout, its result as the button shows it, and every stack the
    /// result display cycles through when it is more than that one
    /// (`SlotDisplay.resolveForStacks`).
    pub fn book_displays(&self) -> Vec<BookDisplay> {
        use crate::recipe_book::Display;
        let first_of =
            |all: &Vec<ItemStack>, single: ItemStack| all.first().cloned().unwrap_or(single);
        let shown = |display, result, variants| BookDisplay {
            display,
            result,
            variants,
            stacks: Vec::new(),
        };
        match self {
            Self::Transmute {
                input,
                material,
                min,
                max,
                add_count,
                result,
            } => (*min..=*max)
                .map(|n| {
                    let mut ingredients = vec![input.clone()];
                    ingredients.extend(std::iter::repeat_n(material.clone(), n));
                    let mut made = result.create();
                    if *add_count {
                        made.count += n as u32;
                    }
                    // The result keeps the input's item: any item of the ingredient.
                    let all: Vec<ItemStack> = if result.item.is_none() {
                        input
                            .1
                            .iter()
                            .map(|item| {
                                let mut stack = made.clone();
                                stack.item = Arc::from(item.as_str());
                                stack
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    shown(Display::Shapeless(ingredients), first_of(&all, made), all)
                })
                .collect(),
            Self::Dye {
                target,
                dye,
                result,
            } => {
                // DyedSlotDemo: the targets cycle fastest, 16 stacks at most.
                let (targets, dyes) = (&target.1, &dye.1);
                let all: Vec<ItemStack> = (0..targets.len() * dyes.len())
                    .map(|i| {
                        let mut made = result.create();
                        made.item = Arc::from(targets[i % targets.len()].as_str());
                        let name = dyes[i / targets.len()].trim_start_matches("minecraft:");
                        let color = crate::block_entity_render::DYES
                            .iter()
                            .find(|(c, _)| name.strip_suffix("_dye") == Some(c))
                            .map_or(crate::block_entity_render::DYES[0].1, |d| d.1);
                        made.data_mut().dyed_color = Some(color as i32);
                        made
                    })
                    .take(16)
                    .collect();
                let first = result.create();
                vec![shown(
                    Display::Shapeless(vec![target.clone(), dye.clone()]),
                    first_of(&all, first),
                    all,
                )]
            }
            Self::Imbue {
                source,
                material,
                result,
            } => {
                let cells = (0..9)
                    .map(|i| Some(if i == 4 { source } else { material }.clone()))
                    .collect();
                // Any potion's tipped arrow.
                let potions = |item: ItemStack| -> Vec<ItemStack> {
                    crate::brewing::POTIONS
                        .iter()
                        .map(|p| item.clone().with_potion(&format!("minecraft:{}", p.id)))
                        .collect()
                };
                let all = potions(result.create());
                // The source ingredient is shown as a lingering potion of each kind.
                let lingering = source
                    .1
                    .iter()
                    .flat_map(|item| potions(ItemStack::new(item, 1)))
                    .collect();
                vec![BookDisplay {
                    stacks: vec![(4, lingering)],
                    ..shown(
                        Display::Shaped {
                            width: 3,
                            height: 3,
                            cells,
                        },
                        first_of(&all, result.create()),
                        all,
                    )
                }]
            }
            _ => Vec::new(),
        }
    }
}
/// One recipe-book display of a special recipe.
pub struct BookDisplay {
    pub display: crate::recipe_book::Display,
    /// The result as the button shows it (the first of `variants`).
    pub result: ItemStack,
    /// Every stack the result display cycles through; empty for one stack.
    pub variants: Vec<ItemStack>,
    /// Ingredients shown as other stacks than their items, by index in
    /// `Entry::requirements`.
    pub stacks: Vec<(usize, Vec<ItemStack>)>,
}
/// `CraftingInput`: the occupied bounding box of the grid.
pub struct Input<'a> {
    pub width: usize,
    pub height: usize,
    /// Position of the box in the grid.
    pub left: usize,
    pub top: usize,
    cells: Vec<Option<&'a ItemStack>>,
}
impl<'a> Input<'a> {
    /// `CraftingInput.ofPositioned`; None for an empty grid.
    pub fn of(grid: &'a [Option<ItemStack>], size: usize) -> Option<Self> {
        let occupied: Vec<(usize, usize)> = (0..size * size)
            .filter(|&i| grid.get(i).is_some_and(|c| c.is_some()))
            .map(|i| (i % size, i / size))
            .collect();
        let (x0, x1) = (
            occupied.iter().map(|p| p.0).min()?,
            occupied.iter().map(|p| p.0).max()?,
        );
        let (y0, y1) = (
            occupied.iter().map(|p| p.1).min()?,
            occupied.iter().map(|p| p.1).max()?,
        );
        let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
        let cells = (0..width * height)
            .map(|i| grid[(y0 + i / width) * size + x0 + i % width].as_ref())
            .collect();
        Some(Self {
            width,
            height,
            left: x0,
            top: y0,
            cells,
        })
    }
    pub fn get(&self, x: usize, y: usize) -> Option<&'a ItemStack> {
        self.cells.get(y * self.width + x).copied().flatten()
    }
    /// The non-empty stacks in slot order.
    pub fn stacks(&self) -> Vec<&'a ItemStack> {
        self.cells.iter().flatten().copied().collect()
    }
    pub fn count(&self) -> usize {
        self.cells.iter().flatten().count()
    }
}
/// What recipes need to know about an item.
#[derive(Clone, Debug, Default)]
struct Facts {
    /// Index into the dye colours when the item has the `dye` component.
    dye: Option<usize>,
    max_damage: u32,
    max_stack: u32,
    remainder: Option<Arc<str>>,
}
/// The tables recipes consult.
pub struct Context<'a> {
    recipes: &'a Recipes,
    /// A filled map's scale, when its data is known.
    pub map_scale: &'a dyn Fn(i32) -> Option<u8>,
}
impl Context<'_> {
    fn facts(&self, stack: &ItemStack) -> Option<&Facts> {
        self.recipes.facts.get(&stack.item)
    }
    /// `itemStack.has(DataComponents.DYE)`.
    fn dye(&self, stack: &ItemStack) -> Option<usize> {
        self.facts(stack).and_then(|f| f.dye)
    }
    fn max_stack(&self, item: &str) -> u32 {
        self.recipes.facts.get(item).map_or(64, |f| f.max_stack)
    }
}
impl Special {
    /// `Recipe.matches`.
    fn matches(&self, ctx: &Context, input: &Input) -> bool {
        use crate::banner;
        let stacks = input.stacks();
        match self {
            Self::BannerDuplicate { banner: ing, .. } => {
                if stacks.len() != 2 {
                    return false;
                }
                let mut color = None;
                let (mut target, mut source) = (false, false);
                for s in &stacks {
                    let Some(this) = banner::banner_color(&s.item).filter(|_| ing.test(s)) else {
                        return false;
                    };
                    match color {
                        None => color = Some(this),
                        Some(c) if c != this => return false,
                        _ => {}
                    }
                    let layers = banner::layers(s).len();
                    if layers > banner::MAX_LAYERS {
                        return false;
                    }
                    let seen = if layers > 0 { &mut source } else { &mut target };
                    if *seen {
                        return false;
                    }
                    *seen = true;
                }
                source && target
            }
            Self::ShieldDecoration {
                banner: ing,
                target: tgt,
                ..
            } => {
                if stacks.len() != 2 {
                    return false;
                }
                let (mut patterned, mut clear) = (false, false);
                for s in &stacks {
                    if ing.test(s) && banner::banner_color(&s.item).is_some() {
                        if patterned {
                            return false;
                        }
                        patterned = true;
                    } else {
                        if !tgt.test(s) || clear || !banner::layers(s).is_empty() {
                            return false;
                        }
                        clear = true;
                    }
                }
                patterned && clear
            }
            Self::Transmute {
                input: base,
                material,
                min,
                max,
                add_count,
                result,
            } => {
                let n = stacks.len();
                if n < min + 1 || n > max + 1 {
                    return false;
                }
                let mut found: Option<&ItemStack> = None;
                let mut materials = 0;
                for s in &stacks {
                    if base.test(s) {
                        if found.is_some() {
                            return false;
                        }
                        found = Some(s);
                    } else {
                        if !material.test(s) {
                            return false;
                        }
                        materials += 1;
                        if materials > *max {
                            return false;
                        }
                    }
                }
                let Some(found) = found else {
                    return false;
                };
                if materials < *min || materials > *max {
                    return false;
                }
                let count = if *add_count {
                    materials as u32 + result.count
                } else {
                    result.count
                };
                if count != 1 {
                    return true;
                }
                let made = result.over(found, 0);
                made.count <= ctx.max_stack(&made.item) && !made.same_item_same_components(found)
            }
            Self::Dye { target, dye, .. } => {
                if stacks.len() < 2 {
                    return false;
                }
                let (mut has_target, mut has_dyes) = (false, false);
                for s in &stacks {
                    if target.test(s) {
                        if has_target {
                            return false;
                        }
                        has_target = true;
                    } else {
                        if !dye.test(s) || ctx.dye(s).is_none() {
                            return false;
                        }
                        has_dyes = true;
                    }
                }
                has_dyes && has_target
            }
            Self::Imbue {
                source, material, ..
            } => {
                if input.width != 3 || input.height != 3 || stacks.len() != 9 {
                    return false;
                }
                (0..3).all(|y| {
                    (0..3).all(|x| {
                        let ing = if x == 1 && y == 1 { source } else { material };
                        input.get(x, y).is_some_and(|s| ing.test(s))
                    })
                })
            }
            Self::BookCloning {
                source,
                material,
                min,
                max,
                ..
            } => {
                if stacks.len() < 2 {
                    return false;
                }
                let (mut has_material, mut has_source) = (false, false);
                for s in &stacks {
                    if source.test(s) {
                        let Some(book) = s.data().and_then(|d| d.written.as_ref()) else {
                            return false;
                        };
                        let generation = i32::from(book.generation);
                        if generation < *min || generation > *max || has_source {
                            return false;
                        }
                        has_source = true;
                    } else {
                        if !material.test(s) {
                            return false;
                        }
                        has_material = true;
                    }
                }
                has_source && has_material
            }
            Self::FireworkRocket {
                shell, fuel, star, ..
            } => {
                if stacks.len() < 2 {
                    return false;
                }
                let mut has_shell = false;
                let mut fuels = 0;
                for s in &stacks {
                    if shell.test(s) {
                        if has_shell {
                            return false;
                        }
                        has_shell = true;
                    } else if fuel.test(s) {
                        fuels += 1;
                        if fuels > 3 {
                            return false;
                        }
                    } else if !star.test(s) {
                        return false;
                    }
                }
                has_shell && fuels >= 1
            }
            Self::FireworkStar {
                dye,
                fuel,
                trail,
                twinkle,
                shapes,
                ..
            } => {
                if stacks.len() < 2 {
                    return false;
                }
                let (mut has_fuel, mut has_dye, mut has_shape) = (false, false, false);
                let (mut has_trail, mut has_twinkle) = (false, false);
                for s in &stacks {
                    if twinkle.test(s) {
                        if has_twinkle {
                            return false;
                        }
                        has_twinkle = true;
                    } else if trail.test(s) {
                        if has_trail {
                            return false;
                        }
                        has_trail = true;
                    } else if fuel.test(s) {
                        if has_fuel {
                            return false;
                        }
                        has_fuel = true;
                    } else if dye.test(s) && ctx.dye(s).is_some() {
                        has_dye = true;
                    } else {
                        if !shapes.iter().any(|(_, ing)| ing.test(s)) || has_shape {
                            return false;
                        }
                        has_shape = true;
                    }
                }
                has_fuel && has_dye
            }
            Self::FireworkStarFade { dye, target, .. } => {
                if stacks.len() < 2 {
                    return false;
                }
                let (mut has_dye, mut has_target) = (false, false);
                for s in &stacks {
                    if dye.test(s) && ctx.dye(s).is_some() {
                        has_dye = true;
                    } else {
                        if !target.test(s) || has_target {
                            return false;
                        }
                        has_target = true;
                    }
                }
                has_target && has_dye
            }
            Self::MapExtending { map, material, .. } => {
                if input.width != 3 || input.height != 3 || stacks.len() != 9 {
                    return false;
                }
                let ring = (0..3).all(|y| {
                    (0..3).all(|x| {
                        let ing = if x == 1 && y == 1 { map } else { material };
                        input.get(x, y).is_some_and(|s| ing.test(s))
                    })
                });
                if !ring {
                    return false;
                }
                stacks
                    .iter()
                    .find_map(|s| s.data().and_then(|d| d.map_id))
                    .and_then(|id| (ctx.map_scale)(id))
                    .is_some_and(|scale| scale < 4)
            }
            Self::DecoratedPot {
                back,
                left,
                right,
                front,
                ..
            } => {
                input.width == 3
                    && input.height == 3
                    && stacks.len() == 4
                    && input.get(1, 0).is_some_and(|s| back.test(s))
                    && input.get(0, 1).is_some_and(|s| left.test(s))
                    && input.get(2, 1).is_some_and(|s| right.test(s))
                    && input.get(1, 2).is_some_and(|s| front.test(s))
            }
            Self::RepairItem => Self::repair_pair(ctx, &stacks).is_some(),
        }
    }
    /// `RepairItemRecipe.getItemsToCombine`.
    fn repair_pair<'a>(
        ctx: &Context,
        stacks: &[&'a ItemStack],
    ) -> Option<(&'a ItemStack, &'a ItemStack)> {
        let [first, second] = stacks else {
            return None;
        };
        let durable = |s: &ItemStack| ctx.facts(s).is_some_and(|f| f.max_damage > 0);
        (second.item == first.item
            && first.count == 1
            && second.count == 1
            && durable(first)
            && durable(second))
        .then_some((*first, *second))
    }
    /// `Recipe.assemble`; None is the empty stack.
    fn assemble(&self, ctx: &Context, input: &Input) -> Option<ItemStack> {
        use crate::banner;
        let stacks = input.stacks();
        let made = match self {
            Self::BannerDuplicate { result, .. } => {
                let source = stacks
                    .iter()
                    .find(|s| (1..=banner::MAX_LAYERS).contains(&banner::layers(s).len()))?;
                // TransmuteRecipe.createWithOriginalComponents: the source's components.
                ItemStack {
                    item: Arc::from(result.as_str()),
                    count: 1,
                    ..(**source).clone()
                }
            }
            Self::ShieldDecoration {
                banner: ing,
                target,
                result,
            } => {
                let (mut banner_stack, mut shield) = (None, None);
                for s in &stacks {
                    if ing.test(s) && banner::banner_color(&s.item).is_some() {
                        banner_stack = Some(*s);
                    } else if target.test(s) {
                        shield = Some(*s);
                    }
                }
                let (banner_stack, shield) = (banner_stack?, shield?);
                let mut out = ItemStack {
                    item: Arc::from(result.as_str()),
                    count: 1,
                    ..shield.clone()
                };
                let data = out.data_mut();
                data.banner_patterns = banner::layers(banner_stack).to_vec();
                data.base_color = banner::banner_color(&banner_stack.item).map(Arc::from);
                out
            }
            Self::Transmute {
                input: base,
                material,
                add_count,
                result,
                ..
            } => {
                if *add_count {
                    let mut materials = 0;
                    let mut found = None;
                    for s in &stacks {
                        if base.test(s) {
                            found = Some(*s);
                        } else if material.test(s) {
                            materials += 1;
                        }
                    }
                    result.over(found?, materials)
                } else {
                    let s = stacks.iter().find(|s| base.test(s))?;
                    result.over(s, 0)
                }
            }
            Self::Dye {
                target,
                dye,
                result,
            } => {
                let mut dyes = Vec::new();
                let mut target_stack: Option<&ItemStack> = None;
                for s in &stacks {
                    if target.test(s) {
                        if target_stack.is_some() {
                            return None;
                        }
                        target_stack = Some(s);
                    } else {
                        if !dye.test(s) {
                            return None;
                        }
                        // getOrDefault(DYE, WHITE)
                        dyes.push(ctx.dye(s).unwrap_or(0));
                    }
                }
                let target_stack = target_stack?;
                if dyes.is_empty() {
                    return None;
                }
                let current = target_stack.data().and_then(|d| d.dyed_color);
                let mut out = result.over(target_stack, 0);
                out.data_mut().dyed_color = Some(apply_dyes(current, &dyes));
                out
            }
            Self::Imbue { result, .. } => {
                let source = input.get(1, 1)?;
                let mut out = result.create();
                out.data_mut().potion = source.data().and_then(|d| d.potion.clone());
                out
            }
            Self::BookCloning {
                source,
                material,
                result,
                ..
            } => {
                let mut copies = 0u32;
                let mut book: Option<&ItemStack> = None;
                for s in &stacks {
                    if source.test(s) && s.data().is_some_and(|d| d.written.is_some()) {
                        if book.is_some() {
                            return None;
                        }
                        book = Some(s);
                    } else {
                        if !material.test(s) {
                            return None;
                        }
                        copies += 1;
                    }
                }
                let book = book?;
                // count - 1 can be -1 when no material was seen; matches() ensures one.
                let mut out = result.over(book, copies.saturating_sub(1));
                if let Some(w) = out.data_mut().written.as_mut() {
                    w.generation += 1;
                }
                out
            }
            Self::FireworkRocket {
                fuel, star, result, ..
            } => {
                let mut explosions = Vec::new();
                let mut fuels = 0u8;
                for s in &stacks {
                    if fuel.test(s) {
                        fuels += 1;
                    } else if star.test(s)
                        && let Some(e) = s.data().and_then(|d| d.firework_explosion.clone())
                    {
                        explosions.push(e);
                    }
                }
                let mut out = result.create();
                // A rocket's prototype is one flight and no explosions.
                if fuels != 1 || !explosions.is_empty() {
                    out.data_mut().fireworks = Some((fuels, explosions));
                }
                out
            }
            Self::FireworkStar {
                dye,
                trail,
                twinkle,
                shapes,
                result,
                ..
            } => {
                let mut shape = FireworkShape::SmallBall;
                let (mut has_twinkle, mut has_trail) = (false, false);
                let mut colors = Vec::new();
                for s in &stacks {
                    if let Some((found, _)) = shapes.iter().find(|(_, ing)| ing.test(s)) {
                        shape = *found;
                    } else if twinkle.test(s) {
                        has_twinkle = true;
                    } else if trail.test(s) {
                        has_trail = true;
                    } else if dye.test(s) {
                        colors.push(DYE_FIREWORK_COLORS[ctx.dye(s).unwrap_or(0)].1);
                    }
                }
                let mut out = result.create();
                out.data_mut().firework_explosion = Some(FireworkExplosion {
                    shape,
                    colors,
                    fade: Vec::new(),
                    trail: has_trail,
                    twinkle: has_twinkle,
                });
                out
            }
            Self::FireworkStarFade {
                dye,
                target,
                result,
            } => {
                let mut colors = Vec::new();
                let mut target_stack = None;
                for s in &stacks {
                    if dye.test(s) {
                        colors.push(DYE_FIREWORK_COLORS[ctx.dye(s).unwrap_or(0)].1);
                    } else if target.test(s) {
                        target_stack = Some(*s);
                    }
                }
                let target_stack = target_stack?;
                if colors.is_empty() {
                    return None;
                }
                let mut out = result.over(target_stack, 0);
                let explosion = out
                    .data_mut()
                    .firework_explosion
                    .get_or_insert_with(FireworkExplosion::default);
                explosion.fade = colors;
                out
            }
            Self::MapExtending { result, .. } => {
                let map = stacks
                    .iter()
                    .find(|s| s.data().is_some_and(|d| d.map_id.is_some()))?;
                let mut out = result.over(map, 0);
                out.data_mut().map_post_processing = Some(1);
                out
            }
            Self::DecoratedPot { result, .. } => {
                let side = |x, y| {
                    input.get(x, y).map(|s| ItemStack {
                        count: 1,
                        ..s.clone()
                    })
                };
                let mut out = result.create();
                let sides = [side(1, 0), side(0, 1), side(2, 1), side(1, 2)];
                if !crate::item_data::all_bricks(&sides) {
                    out.data_mut().pot_decorations = Some(Box::new(sides));
                }
                out
            }
            Self::RepairItem => {
                let (first, second) = Self::repair_pair(ctx, &stacks)?;
                let max = |s: &ItemStack| ctx.facts(s).map_or(0, |f| f.max_damage) as i64;
                let durability = max(first).max(max(second));
                let remaining = (max(first) - i64::from(first.damage))
                    + (max(second) - i64::from(second.damage))
                    + durability * 5 / 100;
                let mut out = ItemStack::new(&first.item, 1);
                out.damage = (durability - remaining).max(0) as u32;
                // Only the curses of both survive, at the higher level.
                let (a, b) = (
                    first.crafting_enchantments(),
                    second.crafting_enchantments(),
                );
                let mut curses: Vec<(Arc<str>, u32)> = Vec::new();
                for (id, level) in a.iter().chain(b.iter()) {
                    if !ctx.recipes.curses.contains(&**id) {
                        continue;
                    }
                    match curses.iter_mut().find(|(c, _)| c == id) {
                        Some((_, l)) => *l = (*l).max(*level),
                        None => curses.push((id.clone(), *level)),
                    }
                }
                out.set_crafting_enchantments(curses);
                out
            }
        };
        let mut made = made;
        made.tidy();
        // ItemStackTemplate.validate: oversized stacks become empty.
        (made.count > 0 && made.count <= ctx.max_stack(&made.item)).then_some(made)
    }
}
/// `DyedItemColor.applyDyes` over dye indexes.
pub(crate) fn apply_dyes(current: Option<i32>, dyes: &[usize]) -> i32 {
    let (mut r, mut g, mut b, mut intensity, mut n) = (0i32, 0i32, 0i32, 0i32, 0i32);
    let mut add = |rgb: u32| {
        let (red, green, blue) = (
            (rgb >> 16 & 255) as i32,
            (rgb >> 8 & 255) as i32,
            (rgb & 255) as i32,
        );
        intensity += red.max(green.max(blue));
        r += red;
        g += green;
        b += blue;
        n += 1;
    };
    if let Some(c) = current {
        add(c as u32);
    }
    for d in dyes {
        add(crate::block_entity_render::DYES[*d].1);
    }
    let (red, green, blue) = (r / n, g / n, b / n);
    let average = intensity as f32 / n as f32;
    let result = red.max(green.max(blue)) as f32;
    let scale = |c: i32| (c as f32 * average / result) as i32;
    scale(red) << 16 | scale(green) << 8 | scale(blue)
}
#[derive(Clone, Debug)]
pub struct Cooking {
    /// The recipe's file name (its id without namespace).
    pub id: String,
    /// `category` (`food`, `blocks`, `misc`) and `group` of the recipe book.
    pub category: String,
    pub group: String,
    pub kind: String,
    pub ingredient: Ingredient,
    pub result: ItemStack,
    pub experience: f32,
    pub time: u32,
}
#[derive(Clone, Debug)]
pub struct MachineRecipe {
    pub id: String,
    pub kind: String,
    pub inputs: Vec<MachineInput>,
    pub outputs: Vec<MachineOutput>,
    pub ticks: u32,
    pub energy: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MachineResourceKind {
    Fluid,
    Chemical,
}
#[derive(Clone, Debug)]
pub enum MachineInput {
    Item {
        ingredient: Ingredient,
        count: u32,
    },
    Resource {
        kind: MachineResourceKind,
        tank: u32,
        accepted: Vec<String>,
        amount: u32,
    },
}
#[derive(Clone, Debug)]
pub enum MachineOutput {
    Item(ItemStack),
    /// A chanced item output (e.g. Create crushing results).
    ItemWithChance {
        stack: ItemStack,
        chance: f32,
    },
    Resource {
        kind: MachineResourceKind,
        tank: u32,
        id: String,
        amount: u32,
    },
}

fn resource_tags(root: &Path, kind: MachineResourceKind) -> HashMap<String, Vec<String>> {
    fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                json_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "json") {
                out.push(path);
            }
        }
    }
    fn expand(
        id: &str,
        raw: &HashMap<String, Vec<String>>,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        if !seen.insert(id.to_owned()) {
            return;
        }
        if let Some(values) = raw.get(id) {
            for value in values {
                if let Some(tag) = value.strip_prefix('#') {
                    expand(tag, raw, seen, out);
                } else if !out.contains(value) {
                    out.push(value.clone());
                }
            }
        }
    }
    let data = root.join("data");
    let Ok(namespaces) = fs::read_dir(data) else {
        return HashMap::new();
    };
    let mut raw = HashMap::new();
    for ns in namespaces.flatten().filter(|e| e.path().is_dir()) {
        let ns_name = ns.file_name().to_string_lossy().into_owned();
        let base = match kind {
            MachineResourceKind::Fluid => ns.path().join("tags/fluid"),
            MachineResourceKind::Chemical => ns.path().join("tags/mekanism/chemical"),
        };
        let mut files = Vec::new();
        json_files(&base, &mut files);
        for path in files {
            let Ok(relative) = path.strip_prefix(&base) else {
                continue;
            };
            let id = format!(
                "{ns_name}:{}",
                relative
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            let Ok(data) = fs::read(&path) else {
                continue;
            };
            let Ok(value) = serde_json::from_slice::<Value>(&data) else {
                continue;
            };
            let values = value["values"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            raw.insert(id, values);
        }
    }
    let mut expanded = HashMap::new();
    for id in raw.keys() {
        let mut values = Vec::new();
        expand(id, &raw, &mut HashSet::new(), &mut values);
        expanded.insert(id.clone(), values);
    }
    expanded
}

/// Thermal's machine JSON uses `ingredient`/`ingredients`, `result(s)`, and
/// optional fluid objects rather than Minecraft's vanilla machine schema.
fn thermal_machine_recipe(
    id: &str,
    kind: &str,
    value: &Value,
    items: &Items,
    fluid_tags: &HashMap<String, Vec<String>>,
) -> Option<MachineRecipe> {
    if !matches!(
        kind,
        "thermal:crystallizer"
            | "thermal:pulverizer"
            | "thermal:furnace"
            | "thermal:press"
            | "thermal:smelter"
            | "thermal:crucible"
            | "thermal:chiller"
            | "thermal:insolator"
    ) {
        return None;
    }
    let mut inputs = Vec::new();
    let mut input_tank = 0u32;
    let ingredients = value
        .get("ingredients")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| value.get("ingredient").cloned().map(|input| vec![input]))?;
    for input in ingredients {
        let fluid = input
            .get("fluid")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                input
                    .get("fluid_tag")
                    .and_then(Value::as_str)
                    .map(|tag| format!("#{tag}"))
            });
        if let Some(fluid) = fluid {
            let accepted = if let Some(tag) = fluid.strip_prefix('#') {
                fluid_tags.get(tag).cloned().unwrap_or_default()
            } else {
                vec![fluid]
            };
            if accepted.is_empty() {
                return None;
            }
            inputs.push(MachineInput::Resource {
                kind: MachineResourceKind::Fluid,
                tank: input_tank,
                accepted,
                amount: input
                    .get("amount")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, u32::MAX as u64) as u32,
            });
            input_tank = input_tank.saturating_add(1);
            continue;
        }
        let ingredient_value = input.get("value").unwrap_or(&input);
        let Some(ingredient) = Ingredient::parse(ingredient_value, items) else {
            return None;
        };
        inputs.push(MachineInput::Item {
            ingredient,
            count: input
                .get("count")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, u32::MAX as u64) as u32,
        });
    }

    let raw_outputs = value.get("results").or_else(|| value.get("result"))?;
    let outputs: Vec<&Value> = raw_outputs
        .as_array()
        .map(|outputs| outputs.iter().collect())
        .unwrap_or_else(|| vec![raw_outputs]);
    let mut parsed_outputs = Vec::new();
    let mut output_tank = 0u32;
    for output in outputs {
        if let Some(fluid) = output
            .get("fluid")
            .and_then(Value::as_str)
            .or_else(|| output.get("fluid_tag").and_then(Value::as_str))
        {
            parsed_outputs.push(MachineOutput::Resource {
                kind: MachineResourceKind::Fluid,
                tank: output_tank,
                id: fluid.strip_prefix('#').unwrap_or(fluid).to_owned(),
                amount: output
                    .get("amount")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, u32::MAX as u64) as u32,
            });
            output_tank = output_tank.saturating_add(1);
            continue;
        }
        let Some(item) = output
            .get("item")
            .and_then(Value::as_str)
            .or_else(|| output.get("id").and_then(Value::as_str))
        else {
            return None;
        };
        let count = output
            .get("count")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .clamp(1, u32::MAX as u64) as u32;
        let chance = output
            .get("chance")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            .max(0.0);
        let guaranteed = chance.floor().min(u32::MAX as f64) as u32;
        if guaranteed > 0 {
            parsed_outputs.push(MachineOutput::Item(ItemStack::new(
                item,
                count.saturating_mul(guaranteed),
            )));
        }
        let fractional = (chance - f64::from(guaranteed)) as f32;
        if fractional > 0.0 {
            parsed_outputs.push(MachineOutput::ItemWithChance {
                stack: ItemStack::new(item, count),
                chance: fractional,
            });
        }
    }
    if inputs.is_empty() || parsed_outputs.is_empty() {
        return None;
    }
    // `energy_mod` is a multiplier against Thermal's configured machine
    // baseline. The baseline is not in the carried JSON, so don't invent one.
    let energy = value.get("energy").and_then(Value::as_u64).unwrap_or(0);
    Some(MachineRecipe {
        id: id.to_owned(),
        kind: kind.to_owned(),
        inputs,
        outputs: parsed_outputs,
        ticks: 0,
        energy,
    })
}

fn create_milling_recipe(id: &str, value: &Value, items: &Items) -> Option<MachineRecipe> {
    let ingredients = value.get("ingredients")?.as_array()?;
    let inputs = ingredients
        .iter()
        .map(|input| {
            Some(MachineInput::Item {
                ingredient: Ingredient::parse(input, items)?,
                count: input
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, u32::MAX as u64) as u32,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let outputs = value.get("results")?.as_array()?;
    let outputs = outputs
        .iter()
        .map(|output| {
            let item = output
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| output.get("item").and_then(Value::as_str))?;
            let stack = ItemStack::new(
                item,
                output
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, u32::MAX as u64) as u32,
            );
            let chance = output.get("chance").and_then(Value::as_f64).unwrap_or(1.0) as f32;
            Some(if chance < 1.0 {
                MachineOutput::ItemWithChance { stack, chance }
            } else {
                MachineOutput::Item(stack)
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if inputs.is_empty() || outputs.is_empty() {
        return None;
    }
    Some(MachineRecipe {
        id: id.to_owned(),
        kind: "create:milling".into(),
        inputs,
        outputs,
        ticks: value
            .get("processing_time")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .clamp(1, u32::MAX as u64) as u32,
        energy: 0,
    })
}
pub struct Recipes {
    pub crafting: Vec<(String, Recipe)>,
    pub cooking: Vec<Cooking>,
    pub machine: Vec<MachineRecipe>,
    /// `category` and `group` of each crafting recipe, by file name.
    pub meta: HashMap<String, (String, String)>,
    facts: HashMap<Arc<str>, Facts>,
    /// The `#minecraft:curse` enchantments.
    curses: HashSet<String>,
    /// The first cooking recipe of each kind taking each item, built at the
    /// first lookup (furnaces look theirs up every tick).
    cook_index: std::sync::OnceLock<CookIndex>,
}
type CookIndex = HashMap<String, crate::fast_hash::FxHashMap<String, usize>>;
/// `MinMaxBounds.Ints` as (min, max) with the given defaults.
fn bounds(v: Option<&Value>, default: (i64, i64), open: (i64, i64)) -> (i64, i64) {
    match v {
        Some(Value::Number(n)) => {
            let n = n.as_i64().unwrap_or(default.0);
            (n, n)
        }
        Some(o @ Value::Object(_)) => (
            o["min"].as_i64().unwrap_or(open.0),
            o["max"].as_i64().unwrap_or(open.1),
        ),
        _ => default,
    }
}
/// `Util.isSymmetrical` is not needed: both orientations are tried.
/// `ShapedRecipePattern.shrink`: the pattern without empty border rows and columns.
fn shrink(pattern: &[Vec<char>]) -> Vec<Vec<char>> {
    let rows: Vec<&Vec<char>> = pattern
        .iter()
        .skip_while(|r| r.iter().all(|c| *c == ' '))
        .collect();
    let mut rows: Vec<Vec<char>> = rows.into_iter().cloned().collect();
    while rows.last().is_some_and(|r| r.iter().all(|c| *c == ' ')) {
        rows.pop();
    }
    let left = rows
        .iter()
        .filter_map(|r| r.iter().position(|c| *c != ' '))
        .min();
    let right = rows
        .iter()
        .filter_map(|r| r.iter().rposition(|c| *c != ' '))
        .max();
    match (left, right) {
        (Some(l), Some(r)) => rows.iter().map(|row| row[l..=r].to_vec()).collect(),
        _ => Vec::new(),
    }
}
/// A perfect matching of the ingredients to distinct stacks (`StackedContents.canCraft`).
fn matches_all(ingredients: &[Ingredient], stacks: &[&ItemStack]) -> bool {
    fn augment(
        i: usize,
        ingredients: &[Ingredient],
        stacks: &[&ItemStack],
        owner: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        for s in 0..stacks.len() {
            if seen[s] || !ingredients[i].test(stacks[s]) {
                continue;
            }
            seen[s] = true;
            if owner[s].is_none_or(|o| augment(o, ingredients, stacks, owner, seen)) {
                owner[s] = Some(i);
                return true;
            }
        }
        false
    }
    if ingredients.len() != stacks.len() {
        return false;
    }
    let mut owner = vec![None; stacks.len()];
    (0..ingredients.len()).all(|i| {
        let mut seen = vec![false; stacks.len()];
        augment(i, ingredients, stacks, &mut owner, &mut seen)
    })
}
impl Recipes {
    pub fn load(root: &Path, items: &Items) -> Result<Self> {
        let mut crafting = Vec::new();
        let mut cooking = Vec::new();
        let mut machine = Vec::new();
        let mut meta = HashMap::new();
        let json_files = |dir: &Path| -> Result<Vec<PathBuf>> {
            fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
                for entry in fs::read_dir(dir)?.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = walk(&path, out);
                    } else if path.extension().is_some_and(|e| e == "json") {
                        out.push(path);
                    }
                }
                Ok(())
            }
            let mut files = Vec::new();
            walk(dir, &mut files)?;
            files.sort();
            Ok(files)
        };
        let fluid_tags = resource_tags(root, MachineResourceKind::Fluid);
        let chemical_tags = resource_tags(root, MachineResourceKind::Chemical);
        // Vanilla's recipes by bare name, then content mods' as `<ns>:<name>`.
        let mc_dir = root.join("data/minecraft/recipe");
        let mut files: Vec<(String, PathBuf, PathBuf)> = json_files(&mc_dir)?
            .into_iter()
            .map(|p| (String::new(), mc_dir.clone(), p))
            .collect();
        for namespace in crate::mod_content::namespaces(root) {
            for sub in ["recipe", "recipes"] {
                let dir = root.join(format!("data/{namespace}/{sub}"));
                if dir.is_dir() {
                    files.extend(
                        json_files(&dir)?
                            .into_iter()
                            .map(|p| (format!("{namespace}:"), dir.clone(), p)),
                    );
                }
            }
        }
        for (prefix, base_dir, path) in files {
            let rel = path
                .strip_prefix(&base_dir)
                .unwrap_or(&path)
                .with_extension("");
            let stem = rel.to_string_lossy().replace('\\', "/");
            let name = format!("{prefix}{stem}");
            // Vanilla's RecipeManager logs a file that does not parse and
            // goes on ("Couldn't parse data file").
            let v: Value = match serde_json::from_slice(&fs::read(&path)?) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Horizon: recipe {name} not loaded: {e}");
                    continue;
                }
            };
            let text = |k: &str| v[k].as_str().unwrap_or("").to_owned();
            meta.insert(name.clone(), (text("category"), text("group")));
            let ing = |k: &str| Ingredient::parse(&v[k], items);
            let result = || Template::parse(&v["result"]);
            let special = |recipe: Option<Special>| recipe.map(Recipe::Special);
            let rtype = v["type"].as_str().unwrap_or("");
            if rtype == "farmersdelight:cooking" {
                let resolve = |selector: &str| -> Vec<String> {
                    match selector.strip_prefix('#') {
                        Some(tag) => items.item_tag_order.get(tag).cloned().unwrap_or_default(),
                        None if items.items.contains_key(selector) => vec![selector.to_owned()],
                        None => Vec::new(),
                    }
                };
                let inputs = v["ingredients"].as_array().and_then(|values| {
                    if values.is_empty() || values.len() > 6 {
                        return None;
                    }
                    values
                        .iter()
                        .map(|value| {
                            let ingredient = pot_ingredient(value, &resolve, 0)?;
                            (!ingredient.0.is_empty()).then_some(MachineInput::Item {
                                ingredient,
                                count: 1,
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                });
                if let (Some(inputs), Some(output)) = (inputs, result())
                    && let Some(id) = &output.item
                    && items.items.contains_key(id.as_ref())
                    && output.count > 0
                    && output.count <= 64
                {
                    machine.push(MachineRecipe {
                        id: name.clone(),
                        kind: rtype.to_owned(),
                        inputs,
                        outputs: vec![MachineOutput::Item(output.create())],
                        ticks: v["cookingtime"]
                            .as_u64()
                            .unwrap_or(200)
                            .clamp(1, u32::MAX as u64) as u32,
                        energy: 0,
                    });
                } else {
                    log::warn!(
                        "Horizon: cooking-pot recipe {name} has unsupported or unresolved ingredients/result"
                    );
                }
            } else if rtype == "minecraft:smithing_transform" {
                let output = v["result"]["id"].as_str();
                let inputs = ["template", "base", "addition"]
                    .map(&ing)
                    .into_iter()
                    .collect::<Option<Vec<_>>>();
                if let (Some(output), Some(inputs)) = (output, inputs)
                    && items.items.contains_key(output)
                {
                    machine.push(MachineRecipe {
                        id: format!("minecraft:{stem}"),
                        kind: rtype.to_owned(),
                        inputs: inputs
                            .into_iter()
                            .map(|ingredient| MachineInput::Item {
                                ingredient,
                                count: 1,
                            })
                            .collect(),
                        outputs: vec![MachineOutput::Item(ItemStack::new(
                            output,
                            v["result"]["count"].as_u64().unwrap_or(1) as u32,
                        ))],
                        ticks: 0,
                        energy: 0,
                    });
                }
            } else if rtype == "ae2:inscriber" {
                let mode = v["mode"].as_str().unwrap_or("");
                let ingredients = &v["ingredients"];
                let ingredient_keys: &[&str] = match mode {
                    "inscribe" => &["top", "middle"],
                    "press" => &["top", "middle", "bottom"],
                    _ => &[],
                };
                let inputs: Option<Vec<_>> = ingredient_keys
                    .iter()
                    .filter_map(|key| ingredients.get(*key))
                    .map(|ingredient| Ingredient::parse(ingredient, items))
                    .collect();
                let expected_count = ingredient_keys
                    .iter()
                    .filter(|key| ingredients.get(**key).is_some())
                    .count();
                let output = v["result"]["item"]
                    .as_str()
                    .or_else(|| v["result"]["id"].as_str());
                if !mode.is_empty()
                    && ingredients.get("middle").is_some()
                    && expected_count > 0
                    && inputs
                        .as_ref()
                        .is_some_and(|inputs| inputs.len() == expected_count)
                    && output.is_some_and(|item| items.items.contains_key(item))
                    && let (Some(inputs), Some(output)) = (inputs, output)
                {
                    machine.push(MachineRecipe {
                        id: name.clone(),
                        kind: format!("ae2:inscriber:{mode}"),
                        inputs: inputs
                            .into_iter()
                            .map(|ingredient| MachineInput::Item {
                                ingredient,
                                count: 1,
                            })
                            .collect(),
                        outputs: vec![MachineOutput::Item(ItemStack::new(
                            output,
                            v["result"]["count"].as_u64().unwrap_or(1) as u32,
                        ))],
                        ticks: 0,
                        energy: 0,
                    });
                }
            } else if rtype == "avaritia:compressor" {
                let cost = v["cost"].as_u64().unwrap_or(0) as u32;
                let ingredients = v["ingredients"].as_array();
                let result = v["result"]["item"].as_str();
                if cost > 0
                    && ingredients.is_some_and(|ingredients| ingredients.len() == 1)
                    && result.is_some_and(|item| items.items.contains_key(item))
                    && let (Some(ingredient), Some(result)) = (
                        ingredients
                            .and_then(|ingredients| ingredients.first())
                            .and_then(|ingredient| Ingredient::parse(ingredient, items)),
                        result,
                    )
                {
                    machine.push(MachineRecipe {
                        id: name.clone(),
                        kind: rtype.to_owned(),
                        inputs: vec![MachineInput::Item {
                            ingredient,
                            count: cost,
                        }],
                        outputs: vec![MachineOutput::Item(ItemStack::new(
                            result,
                            v["result"]["count"].as_u64().unwrap_or(1) as u32,
                        ))],
                        ticks: 0,
                        energy: 0,
                    });
                }
            } else if rtype.starts_with("thermal:") {
                if let Some(recipe) = thermal_machine_recipe(&name, rtype, &v, items, &fluid_tags) {
                    machine.push(recipe);
                }
            } else if rtype == "create:milling" {
                if let Some(recipe) = create_milling_recipe(&name, &v, items)
                    && recipe.outputs.iter().all(|output| match output {
                        MachineOutput::Item(stack)
                        | MachineOutput::ItemWithChance { stack, .. } => {
                            items.items.contains_key(stack.item.as_ref())
                        }
                        MachineOutput::Resource { .. } => false,
                    })
                {
                    machine.push(recipe);
                }
            } else if rtype == "create:crushing" {
                let mut inputs = Vec::new();
                let mut resource_tanks = HashMap::<MachineResourceKind, u32>::new();
                for input in v["ingredients"].as_array().into_iter().flatten() {
                    if let Some(ingredient) = Ingredient::parse(input, items) {
                        let count = input["count"].as_u64().unwrap_or(1) as u32;
                        inputs.push(MachineInput::Item {
                            ingredient,
                            count: count.max(1),
                        });
                        continue;
                    }
                    let fluid = input["fluid"]
                        .as_str()
                        .map(str::to_owned)
                        .or_else(|| input["fluid_tag"].as_str().map(|tag| format!("#{tag}")));
                    if let Some(fluid) = fluid {
                        let tank = resource_tanks
                            .entry(MachineResourceKind::Fluid)
                            .or_default();
                        let accepted = if let Some(tag) = fluid.strip_prefix('#') {
                            fluid_tags.get(tag).cloned().unwrap_or_default()
                        } else {
                            vec![fluid]
                        };
                        inputs.push(MachineInput::Resource {
                            kind: MachineResourceKind::Fluid,
                            tank: *tank,
                            accepted,
                            amount: input["amount"].as_u64().unwrap_or(1).max(1) as u32,
                        });
                        *tank += 1;
                    }
                }
                let outputs_json = v["results"].as_array().or_else(|| v["result"].as_array());
                let mut outputs = Vec::new();
                for output in outputs_json.into_iter().flatten() {
                    let Some(item) = output["item"].as_str().or_else(|| output["id"].as_str())
                    else {
                        continue;
                    };
                    if !items.items.contains_key(item) {
                        continue;
                    }
                    let stack =
                        ItemStack::new(item, output["count"].as_u64().unwrap_or(1).max(1) as u32);
                    let chance = output["chance"].as_f64().unwrap_or(1.0) as f32;
                    if chance < 1.0 {
                        outputs.push(MachineOutput::ItemWithChance { stack, chance });
                    } else {
                        outputs.push(MachineOutput::Item(stack));
                    }
                }
                if !inputs.is_empty() && !outputs.is_empty() {
                    machine.push(MachineRecipe {
                        id: name.clone(),
                        kind: rtype.to_owned(),
                        inputs,
                        outputs,
                        ticks: v["processingTime"]
                            .as_u64()
                            .or_else(|| v["processing_time"].as_u64())
                            .unwrap_or(100) as u32,
                        energy: v["energy"].as_u64().unwrap_or(0),
                    });
                }
            } else if rtype.starts_with("mekanism:") {
                type InputSpec = (&'static str, Option<MachineResourceKind>);
                let variants: Vec<Vec<InputSpec>> = match rtype {
                    "mekanism:combining" => vec![vec![("main_input", None), ("extra_input", None)]],
                    "mekanism:metallurgic_infusing"
                    | "mekanism:injecting"
                    | "mekanism:purifying"
                    | "mekanism:dissolution"
                    | "mekanism:nucleosynthesizing"
                    | "mekanism:compressing" => vec![vec![
                        ("item_input", None),
                        ("chemical_input", Some(MachineResourceKind::Chemical)),
                    ]],
                    "mekanism:reaction" => vec![vec![
                        ("item_input", None),
                        ("chemical_input", Some(MachineResourceKind::Chemical)),
                        ("fluid_input", Some(MachineResourceKind::Fluid)),
                    ]],
                    "mekanism:washing" => vec![vec![
                        ("chemical_input", Some(MachineResourceKind::Chemical)),
                        ("fluid_input", Some(MachineResourceKind::Fluid)),
                    ]],
                    "mekanism:chemical_infusing" | "mekanism:pigment_mixing" => vec![vec![
                        ("left_input", Some(MachineResourceKind::Chemical)),
                        ("right_input", Some(MachineResourceKind::Chemical)),
                    ]],
                    "mekanism:painting" => vec![vec![
                        ("item_input", None),
                        ("chemical_input", Some(MachineResourceKind::Chemical)),
                    ]],
                    "mekanism:rotary" => vec![
                        vec![("chemical_input", Some(MachineResourceKind::Chemical))],
                        vec![("fluid_input", Some(MachineResourceKind::Fluid))],
                    ],
                    "mekanism:separating" | "mekanism:evaporating" => {
                        vec![vec![("input", Some(MachineResourceKind::Fluid))]]
                    }
                    "mekanism:crystallizing" | "mekanism:centrifuging" | "mekanism:activating" => {
                        vec![vec![("input", Some(MachineResourceKind::Chemical))]]
                    }
                    _ => vec![vec![("input", None)]],
                };
                for variant in variants {
                    let rotary_fluid_mode = variant
                        .first()
                        .is_some_and(|(_, kind)| *kind == Some(MachineResourceKind::Fluid));
                    let mut inputs = Vec::new();
                    let mut resource_input_tanks = HashMap::<MachineResourceKind, u32>::new();
                    for (field, resource_kind) in variant {
                        let value = &v[field];
                        if let Some(kind) = resource_kind {
                            let selector = value["tag"]
                                .as_str()
                                .map(|tag| (tag, true))
                                .or_else(|| {
                                    value["chemical"]
                                        .as_str()
                                        .or_else(|| value["fluid"].as_str())
                                        .map(|id| (id, false))
                                })
                                .or_else(|| value["id"].as_str().map(|id| (id, false)));
                            let Some((selector, is_tag)) = selector else {
                                continue;
                            };
                            let tags = match kind {
                                MachineResourceKind::Fluid => &fluid_tags,
                                MachineResourceKind::Chemical => &chemical_tags,
                            };
                            let accepted = if is_tag {
                                tags.get(selector)
                                    .cloned()
                                    .unwrap_or_else(|| vec![selector.to_owned()])
                            } else {
                                vec![selector.to_owned()]
                            };
                            let tank = resource_input_tanks.entry(kind).or_default();
                            let amount = value["amount"].as_u64().unwrap_or(1) as u32;
                            inputs.push(MachineInput::Resource {
                                kind,
                                tank: *tank,
                                accepted,
                                amount: amount.max(1),
                            });
                            *tank += 1;
                        } else if let Some(ingredient) = Ingredient::parse(value, items) {
                            let count = value["count"].as_u64().unwrap_or(1) as u32;
                            inputs.push(MachineInput::Item {
                                ingredient,
                                count: count.max(1),
                            });
                        }
                    }
                    let mut outputs = Vec::new();
                    let mut output_resource_tanks = resource_input_tanks.clone();
                    let mut add_item_output = |field: &str| {
                        let value = &v[field];
                        if let Some(item) = value["id"].as_str()
                            && items.items.contains_key(item)
                        {
                            let count = value["count"].as_u64().unwrap_or(1) as u32;
                            outputs.push(MachineOutput::Item(ItemStack::new(item, count.max(1))));
                        }
                    };
                    let item_output_fields: &[&str] = match rtype {
                        "mekanism:enriching" | "mekanism:crushing" | "mekanism:combining" => {
                            &["output"]
                        }
                        "mekanism:sawing" => &["main_output", "secondary_output"],
                        "mekanism:reaction" => &["item_output"],
                        "mekanism:crystallizing" => &["output"],
                        "mekanism:painting"
                        | "mekanism:metallurgic_infusing"
                        | "mekanism:injecting"
                        | "mekanism:purifying"
                        | "mekanism:nucleosynthesizing"
                        | "mekanism:compressing" => &["output"],
                        _ => &[],
                    };
                    for field in item_output_fields {
                        add_item_output(field);
                    }
                    let mut add_resource_output = |field: &str, kind: MachineResourceKind| {
                        let value = &v[field];
                        if let Some(id) = value["id"]
                            .as_str()
                            .or_else(|| value["chemical"].as_str())
                            .or_else(|| value["fluid"].as_str())
                        {
                            let tank = output_resource_tanks.entry(kind).or_default();
                            let amount = value["amount"].as_u64().unwrap_or(1) as u32;
                            outputs.push(MachineOutput::Resource {
                                kind,
                                tank: *tank,
                                id: id.to_owned(),
                                amount: amount.max(1),
                            });
                            *tank += 1;
                        }
                    };
                    match rtype {
                        "mekanism:chemical_conversion"
                        | "mekanism:oxidizing"
                        | "mekanism:pigment_extracting"
                        | "mekanism:activating"
                        | "mekanism:centrifuging"
                        | "mekanism:chemical_infusing"
                        | "mekanism:pigment_mixing" => {
                            add_resource_output("output", MachineResourceKind::Chemical);
                        }
                        "mekanism:evaporating" => {
                            add_resource_output("output", MachineResourceKind::Fluid);
                        }
                        "mekanism:metallurgic_infusing"
                        | "mekanism:injecting"
                        | "mekanism:purifying"
                        | "mekanism:dissolution"
                        | "mekanism:nucleosynthesizing"
                        | "mekanism:compressing"
                        | "mekanism:washing" => {
                            if item_output_fields.is_empty() {
                                add_resource_output("output", MachineResourceKind::Chemical);
                            }
                        }
                        "mekanism:reaction" => {
                            add_resource_output("chemical_output", MachineResourceKind::Chemical);
                        }
                        "mekanism:separating" => {
                            add_resource_output(
                                "left_chemical_output",
                                MachineResourceKind::Chemical,
                            );
                            add_resource_output(
                                "right_chemical_output",
                                MachineResourceKind::Chemical,
                            );
                        }
                        "mekanism:rotary" => {
                            if rotary_fluid_mode {
                                add_resource_output(
                                    "chemical_output",
                                    MachineResourceKind::Chemical,
                                );
                            } else {
                                add_resource_output("fluid_output", MachineResourceKind::Fluid);
                            }
                        }
                        _ => {}
                    }
                    if !inputs.is_empty() && !outputs.is_empty() {
                        machine.push(MachineRecipe {
                            id: name.clone(),
                            kind: rtype.to_owned(),
                            inputs,
                            outputs,
                            ticks: v["duration"].as_u64().unwrap_or(200) as u32,
                            energy: v["energy_required"].as_u64().unwrap_or(0),
                        });
                    }
                }
            }
            let parsed: Option<Recipe> = match rtype {
                "minecraft:crafting_shaped" | "crafting_shaped" | "mekanism:mek_data"
                    if v.get("pattern").is_some() && v.get("key").is_some() =>
                {
                    let pattern: Vec<Vec<char>> = v["pattern"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.as_str().map(|s| s.chars().collect()))
                        .collect();
                    let pattern = shrink(&pattern);
                    let height = pattern.len();
                    let width = pattern.first().map_or(0, Vec::len);
                    let mut cells = Vec::with_capacity(width * height);
                    let mut ok = height > 0;
                    for row in &pattern {
                        for c in row {
                            if *c == ' ' {
                                cells.push(None);
                            } else {
                                match Ingredient::parse(&v["key"][c.to_string()], items) {
                                    Some(i) => cells.push(Some(i)),
                                    None => ok = false,
                                }
                            }
                        }
                    }
                    match (ok, result()) {
                        (true, Some(result)) => Some(Recipe::Shaped {
                            width,
                            height,
                            cells,
                            result,
                        }),
                        _ => None,
                    }
                }
                "minecraft:crafting_shapeless" | "crafting_shapeless"
                    if v.get("ingredients").is_some() =>
                {
                    let ingredients: Option<Vec<_>> = v["ingredients"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|i| Ingredient::parse(i, items))
                        .collect();
                    match (ingredients, result()) {
                        (Some(ingredients), Some(result)) => Some(Recipe::Shapeless {
                            ingredients,
                            result,
                        }),
                        _ => None,
                    }
                }
                _ if v.get("pattern").is_some()
                    && v.get("key").is_some()
                    && v.get("result").is_some() =>
                {
                    let pattern: Vec<Vec<char>> = v["pattern"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.as_str().map(|s| s.chars().collect()))
                        .collect();
                    let pattern = shrink(&pattern);
                    let height = pattern.len();
                    let width = pattern.first().map_or(0, Vec::len);
                    let mut cells = Vec::with_capacity(width * height);
                    let mut ok = height > 0 && width <= 3 && height <= 3;
                    for row in &pattern {
                        for c in row {
                            if *c == ' ' {
                                cells.push(None);
                            } else {
                                match Ingredient::parse(&v["key"][c.to_string()], items) {
                                    Some(i) => cells.push(Some(i)),
                                    None => ok = false,
                                }
                            }
                        }
                    }
                    match (ok, result()) {
                        (true, Some(result)) => Some(Recipe::Shaped {
                            width,
                            height,
                            cells,
                            result,
                        }),
                        _ => None,
                    }
                }
                _ if rtype.contains("shapeless")
                    && v.get("ingredients").is_some()
                    && v.get("result").is_some() =>
                {
                    let ingredients: Option<Vec<_>> = v["ingredients"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|i| Ingredient::parse(i, items))
                        .collect();
                    match (ingredients, result()) {
                        (Some(ingredients), Some(result)) if ingredients.len() <= 9 => {
                            Some(Recipe::Shapeless {
                                ingredients,
                                result,
                            })
                        }
                        _ => None,
                    }
                }
                "minecraft:crafting_special_bannerduplicate" => special(
                    ing("banner")
                        .zip(v["result"]["id"].as_str())
                        .map(|(banner, id)| Special::BannerDuplicate {
                            banner,
                            result: id.to_owned(),
                        }),
                ),
                "minecraft:crafting_special_shielddecoration" => special(
                    ing("banner")
                        .zip(ing("target"))
                        .zip(v["result"]["id"].as_str())
                        .map(|((banner, target), id)| Special::ShieldDecoration {
                            banner,
                            target,
                            result: id.to_owned(),
                        }),
                ),
                "minecraft:crafting_transmute" => special((|| {
                    let (min, max) = bounds(v.get("material_count"), (1, 1), (1, 8));
                    Some(Special::Transmute {
                        input: ing("input")?,
                        material: ing("material")?,
                        min: min.clamp(1, 8) as usize,
                        max: max.clamp(1, 8) as usize,
                        add_count: v["add_material_count_to_result"].as_bool().unwrap_or(false),
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_dye" => special((|| {
                    Some(Special::Dye {
                        target: ing("target")?,
                        dye: ing("dye")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_imbue" => special((|| {
                    Some(Special::Imbue {
                        source: ing("source")?,
                        material: ing("material")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_bookcloning" => special((|| {
                    let (min, max) = bounds(v.get("allowed_generations"), (0, 1), (0, 2));
                    Some(Special::BookCloning {
                        source: ing("source")?,
                        material: ing("material")?,
                        min: min as i32,
                        max: max as i32,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_firework_rocket" => special((|| {
                    Some(Special::FireworkRocket {
                        shell: ing("shell")?,
                        fuel: ing("fuel")?,
                        star: ing("star")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_firework_star" => special((|| {
                    let mut shapes = Vec::new();
                    for (shape, value) in v["shapes"].as_object()? {
                        shapes.push((
                            FireworkShape::from_name(shape),
                            Ingredient::parse(value, items)?,
                        ));
                    }
                    Some(Special::FireworkStar {
                        dye: ing("dye")?,
                        fuel: ing("fuel")?,
                        trail: ing("trail")?,
                        twinkle: ing("twinkle")?,
                        shapes,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_firework_star_fade" => special((|| {
                    Some(Special::FireworkStarFade {
                        dye: ing("dye")?,
                        target: ing("target")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_mapextending" => special((|| {
                    Some(Special::MapExtending {
                        map: ing("map")?,
                        material: ing("material")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_decorated_pot" => special((|| {
                    Some(Special::DecoratedPot {
                        back: ing("back")?,
                        left: ing("left")?,
                        right: ing("right")?,
                        front: ing("front")?,
                        result: result()?,
                    })
                })()),
                "minecraft:crafting_special_repairitem" => special(Some(Special::RepairItem)),
                kind @ ("minecraft:smelting"
                | "minecraft:blasting"
                | "minecraft:smoking"
                | "minecraft:campfire_cooking") => {
                    if let (Some(ingredient), Some(result)) = (ing("ingredient"), result()) {
                        cooking.push(Cooking {
                            id: name.clone(),
                            category: text("category"),
                            group: text("group"),
                            kind: kind.to_owned(),
                            ingredient,
                            result: result.create(),
                            experience: v["experience"].as_f64().unwrap_or(0.0) as f32,
                            time: v["cookingtime"].as_u64().unwrap_or(200) as u32,
                        });
                    }
                    None
                }
                _ => None,
            };
            if let Some(recipe) = parsed {
                crafting.push((name, recipe));
            }
        }
        let facts = items
            .items
            .iter()
            .map(|(id, info)| {
                (
                    id.clone(),
                    Facts {
                        dye: info.dye.as_deref().and_then(|d| {
                            crate::block_entity_render::DYES
                                .iter()
                                .position(|(name, _)| *name == d)
                        }),
                        max_damage: info.max_damage,
                        max_stack: info.max_stack,
                        remainder: info.crafting_remainder.as_deref().map(Arc::from),
                    },
                )
            })
            .collect();
        let curses = crate::items::load_tags(root, "enchantment")?
            .remove("minecraft:curse")
            .unwrap_or_default();
        Ok(Self {
            crafting,
            cooking,
            machine,
            meta,
            facts,
            curses,
            cook_index: Default::default(),
        })
    }
    /// Match a `size`x`size` grid (row-major) against the crafting recipes.
    pub fn craft(&self, grid: &[Option<ItemStack>], size: usize) -> Option<ItemStack> {
        self.find(grid, size).map(|(_, result)| result)
    }
    /// `craft` with the matched recipe's id (the file name without namespace).
    pub fn find(&self, grid: &[Option<ItemStack>], size: usize) -> Option<(&str, ItemStack)> {
        self.find_with(grid, size, &|_| None)
    }
    /// `RecipeManager.getRecipeFor(CRAFTING, ...)` with a lookup for the scale
    /// of a filled map (map extending reads the map's saved data).
    pub fn find_with(
        &self,
        grid: &[Option<ItemStack>],
        size: usize,
        map_scale: &dyn Fn(i32) -> Option<u8>,
    ) -> Option<(&str, ItemStack)> {
        let input = Input::of(grid, size)?;
        let ctx = Context {
            recipes: self,
            map_scale,
        };
        for (name, recipe) in &self.crafting {
            if let Some(result) = Self::assemble_if_matches(&ctx, &input, recipe) {
                return result.map(|r| (name.as_str(), r));
            }
        }
        None
    }
    /// `Recipe.matches` then `assemble` for one recipe: None when it does
    /// not match, Some(None) when it matches with nothing to make.
    fn assemble_if_matches(
        ctx: &Context,
        input: &Input,
        recipe: &Recipe,
    ) -> Option<Option<ItemStack>> {
        match recipe {
            Recipe::Special(special) => special
                .matches(ctx, input)
                .then(|| special.assemble(ctx, input)),
            Recipe::Shaped {
                width,
                height,
                cells,
                result,
            } => {
                if *width != input.width
                    || *height != input.height
                    || cells.iter().flatten().count() != input.count()
                {
                    return None;
                }
                let matches = |mirror: bool| {
                    (0..*height).all(|y| {
                        (0..*width).all(|x| {
                            let rx = if mirror { width - 1 - x } else { x };
                            match (&cells[y * width + rx], input.get(x, y)) {
                                (None, None) => true,
                                (Some(i), Some(s)) => i.test(s),
                                _ => false,
                            }
                        })
                    })
                };
                (matches(false) || matches(true)).then(|| Some(result.create()))
            }
            Recipe::Shapeless {
                ingredients,
                result,
            } => matches_all(ingredients, &input.stacks()).then(|| Some(result.create())),
        }
    }
    /// `recipe.matches(craftSlots.asCraftInput())` for one recipe by id
    /// (`minecraft:name`).
    pub fn recipe_matches(&self, id: &str, grid: &[Option<ItemStack>], size: usize) -> bool {
        let name = id.trim_start_matches("minecraft:");
        let Some((_, recipe)) = self.crafting.iter().find(|(n, _)| n == name) else {
            return false;
        };
        let Some(input) = Input::of(grid, size) else {
            return false;
        };
        let ctx = Context {
            recipes: self,
            map_scale: &|_| None,
        };
        Self::assemble_if_matches(&ctx, &input, recipe).is_some()
    }
    /// `Recipe.placementInfo` of a crafting or cooking recipe by id; None for
    /// an unknown id, an empty placement when it is impossible to place.
    pub fn placement(&self, id: &str) -> Option<Placement> {
        let name = id.trim_start_matches("minecraft:");
        if let Some(c) = self.cooking.iter().find(|c| c.id == name) {
            return Some(Placement::of(vec![Some(c.ingredient.clone())], None));
        }
        let (_, recipe) = self.crafting.iter().find(|(n, _)| n == name)?;
        Some(match recipe {
            Recipe::Shaped {
                width,
                height,
                cells,
                ..
            } => Placement::of(cells.clone(), Some((*width, *height))),
            Recipe::Shapeless { ingredients, .. } => {
                Placement::of(ingredients.iter().cloned().map(Some).collect(), None)
            }
            Recipe::Special(Special::Transmute {
                input,
                material,
                max,
                ..
            }) => {
                let mut list = vec![Some(input.clone())];
                list.extend(std::iter::repeat_n(Some(material.clone()), *max));
                Placement::of(list, None)
            }
            Recipe::Special(Special::Dye { target, dye, .. }) => {
                Placement::of(vec![Some(target.clone()), Some(dye.clone())], None)
            }
            Recipe::Special(Special::Imbue {
                source, material, ..
            }) => Placement::of(
                (0..9)
                    .map(|i| Some(if i == 4 { source } else { material }.clone()))
                    .collect(),
                None,
            ),
            // CustomRecipe.placementInfo: NOT_PLACEABLE.
            Recipe::Special(_) => Placement::default(),
        })
    }
    /// `Recipe.getRemainingItems` placed on the `size`x`size` grid: an item's
    /// crafting remainder (buckets, bottles), the patterned banner a duplicate
    /// keeps and the written book a clone keeps.
    pub fn remainders(
        &self,
        recipe: &str,
        grid: &[Option<ItemStack>],
        size: usize,
    ) -> Vec<Option<ItemStack>> {
        let mut out = vec![None; size * size];
        let Some(input) = Input::of(grid, size) else {
            return out;
        };
        let kind = self
            .crafting
            .iter()
            .find(|(n, _)| n == recipe)
            .map(|(_, r)| r);
        for (i, cell) in input.cells.iter().enumerate() {
            let Some(s) = cell else { continue };
            let at = (input.top + i / input.width) * size + input.left + i % input.width;
            let rest = self
                .facts
                .get(&s.item)
                .and_then(|f| f.remainder.as_deref())
                .map(|r| ItemStack::new(r, 1));
            if rest.is_some() {
                out[at] = rest;
                continue;
            }
            let copy = ItemStack {
                count: 1,
                ..(*s).clone()
            };
            match kind {
                Some(Recipe::Special(Special::BannerDuplicate { .. }))
                    if !crate::banner::layers(s).is_empty() =>
                {
                    out[at] = Some(copy);
                }
                // BookCloningRecipe stops at the first copied book.
                Some(Recipe::Special(Special::BookCloning { .. }))
                    if s.data().is_some_and(|d| d.written.is_some()) =>
                {
                    out[at] = Some(copy);
                    break;
                }
                _ => {}
            }
        }
        out
    }
    pub fn cook(&self, kind: &str, input: &ItemStack) -> Option<&Cooking> {
        let index = self.cook_index.get_or_init(|| {
            let mut index = CookIndex::new();
            for (i, c) in self.cooking.iter().enumerate() {
                let of_kind = index.entry(c.kind.clone()).or_default();
                for item in &c.ingredient.0 {
                    of_kind.entry(item.clone()).or_insert(i);
                }
            }
            index
        });
        let i = *index.get(kind)?.get(&*input.item)?;
        self.cooking.get(i)
    }
    /// `cook` by scanning every recipe, as it was before the index.
    #[cfg(test)]
    fn cook_scan(&self, kind: &str, input: &ItemStack) -> Option<&Cooking> {
        self.cooking
            .iter()
            .find(|c| c.kind == kind && c.ingredient.test(input))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_milling_parses_processing_time_and_chanced_result_ids() {
        use serde_json::json;
        use std::collections::{HashMap, HashSet};

        let items = crate::items::Items::for_tests(HashMap::from([(
            "forge:gravel".into(),
            HashSet::from(["minecraft:gravel".into()]),
        )]));
        let milling = json!({
            "type":"create:milling",
            "ingredients":[{"tag":"forge:gravel"}],
            "processing_time":100,
            "results":[
                {"id":"minecraft:coal", "chance":0.35},
                {"id":"minecraft:raw_iron", "chance":0.35},
                {"id":"create:raw_zinc", "chance":0.2},
                {"id":"minecraft:sand", "chance":0.4}
            ]
        });
        let parsed =
            super::create_milling_recipe("ftbstoneblock:milling/gravel", &milling, &items).unwrap();
        assert_eq!(parsed.kind, "create:milling");
        assert_eq!(parsed.ticks, 100);
        assert_eq!(parsed.inputs.len(), 1);
        assert!(matches!(
            &parsed.inputs[0],
            super::MachineInput::Item { ingredient, count: 1 }
                if ingredient.1 == ["minecraft:gravel"]
        ));
        let expected = [
            ("minecraft:coal", 0.35),
            ("minecraft:raw_iron", 0.35),
            ("create:raw_zinc", 0.2),
            ("minecraft:sand", 0.4),
        ];
        assert_eq!(parsed.outputs.len(), expected.len());
        for (output, (item, chance)) in parsed.outputs.iter().zip(expected) {
            assert!(matches!(
                output,
                super::MachineOutput::ItemWithChance { stack, chance: actual }
                    if stack.item.as_ref() == item && stack.count == 1 && (*actual - chance).abs() < 0.0001
            ));
        }
    }

    #[test]
    fn thermal_smelter_and_crucible_recipes_parse_item_and_fluid_resources() {
        use serde_json::json;
        use std::collections::{HashMap, HashSet};

        let items = crate::items::Items::for_tests(HashMap::from([
            (
                "forge:ingots/copper".into(),
                HashSet::from(["minecraft:copper_ingot".into()]),
            ),
            (
                "forge:ingots/tin".into(),
                HashSet::from(["thermal:tin_ingot".into()]),
            ),
        ]));
        let smelter = json!({
            "type": "thermal:smelter",
            "ingredients": [
                {"value": [{"tag":"forge:ingots/copper"}], "count": 3},
                {"value": [{"tag":"forge:ingots/tin"}], "count": 1}
            ],
            "result": [{"item":"thermal:bronze_ingot", "count":4}],
            "energy": 6400
        });
        let parsed = super::thermal_machine_recipe(
            "thermal:smelter_alloy_bronze",
            "thermal:smelter",
            &smelter,
            &items,
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(parsed.energy, 6400);
        assert_eq!(parsed.inputs.len(), 2);
        match &parsed.inputs[0] {
            super::MachineInput::Item { ingredient, count } => {
                assert_eq!(*count, 3);
                assert_eq!(ingredient.1, ["minecraft:copper_ingot"]);
            }
            _ => panic!("expected the copper ingredient"),
        }
        assert!(matches!(
            &parsed.outputs[0],
            super::MachineOutput::Item(stack)
                if stack.item.as_ref() == "thermal:bronze_ingot" && stack.count == 4
        ));

        let default_energy = json!({
            "type":"thermal:pulverizer",
            "ingredient":{"tag":"forge:ingots/copper"},
            "result":[{"item":"thermal:copper_dust"}],
            "energy_mod":0.5
        });
        let parsed = super::thermal_machine_recipe(
            "thermal:pulverizer_copper_ingot_to_dust",
            "thermal:pulverizer",
            &default_energy,
            &items,
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(
            parsed.energy, 0,
            "unresolved Thermal baseline must not be guessed"
        );

        let crucible = json!({
            "type":"thermal:crucible",
            "ingredient":{"item":"minecraft:cobblestone"},
            "result":[{"fluid":"minecraft:lava", "amount":1000}],
            "energy":200000
        });
        let parsed = super::thermal_machine_recipe(
            "thermal:crucible_cobblestone_to_lava",
            "thermal:crucible",
            &crucible,
            &items,
            &HashMap::new(),
        )
        .unwrap();
        assert!(matches!(
            &parsed.inputs[0],
            super::MachineInput::Item { ingredient, .. }
                if ingredient.1 == ["minecraft:cobblestone"]
        ));
        assert!(matches!(
            &parsed.outputs[0],
            super::MachineOutput::Resource { id, amount, .. }
                if id == "minecraft:lava" && *amount == 1000
        ));
    }

    #[test]
    fn pot_ingredients_expand_unions_and_exclusions_without_ignoring_predicates() {
        use serde_json::json;
        let resolve = |id: &str| match id {
            "#vegetables" => vec!["carrot".into(), "melon".into()],
            "#empty" => Vec::new(),
            id => vec![id.to_owned()],
        };
        let difference = json!({"type":"neoforge:difference", "base":{"tag":"vegetables"},"subtracted":{"item":"melon"}});
        assert_eq!(
            super::pot_ingredient(&difference, &resolve, 0).unwrap().1,
            ["carrot"]
        );
        let union = json!({"type":"neoforge:compound","children":[difference,{"item":"fish"},{"tag":"empty"}]});
        assert_eq!(
            super::pot_ingredient(&union, &resolve, 0).unwrap().1,
            ["carrot", "fish"]
        );
        assert!(
            super::pot_ingredient(
                &json!({"type":"unknown:predicate","base":{"item":"carrot"}}),
                &resolve,
                0
            )
            .is_none()
        );
        assert!(super::pot_ingredient(&union, &resolve, 33).is_none());
    }
    use super::*;
    use crate::banner::BannerLayer;

    fn ingredient(items: &[&str]) -> Ingredient {
        Ingredient::of(items.iter().map(|s| (*s).to_owned()).collect())
    }
    fn patterned(item: &str) -> ItemStack {
        let mut s = ItemStack::new(item, 1);
        s.data_mut().banner_patterns = vec![BannerLayer::new("minecraft:cross", "red")];
        s
    }
    /// About vanilla's count of cooking recipes: four kinds, ingredients of
    /// one to twelve items, overlapping (several recipes take some items).
    fn cooking_book() -> Recipes {
        let kinds = ["smelting", "blasting", "smoking", "campfire_cooking"];
        let cooking = (0..320usize)
            .map(|i| Cooking {
                id: format!("r{i}"),
                category: "misc".into(),
                group: String::new(),
                kind: kinds[i % 4].into(),
                ingredient: ingredient(
                    &(0..1 + i % 12)
                        .map(|k| format!("minecraft:item_{}", (i * 7 + k * 13) % 400))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                ),
                result: ItemStack::new("minecraft:stone", 1),
                experience: 0.1,
                time: 200,
            })
            .collect();
        Recipes {
            crafting: Vec::new(),
            cooking,
            machine: Vec::new(),
            meta: HashMap::new(),
            facts: HashMap::new(),
            curses: HashSet::new(),
            cook_index: Default::default(),
        }
    }
    #[test]
    fn cook_finds_the_first_recipe_the_scan_finds() {
        let recipes = cooking_book();
        for kind in [
            "smelting",
            "blasting",
            "smoking",
            "campfire_cooking",
            "other",
        ] {
            for n in 0..420 {
                let stack = ItemStack::new(&format!("minecraft:item_{n}"), 1);
                let a = recipes.cook(kind, &stack).map(|c| &c.id);
                let b = recipes.cook_scan(kind, &stack).map(|c| &c.id);
                assert_eq!(a, b, "{kind} item_{n}");
            }
        }
    }
    /// A furnace's recipe lookup, once per lit furnace per tick.
    #[test]
    #[ignore]
    fn cook_cost() {
        let recipes = cooking_book();
        let stacks: Vec<ItemStack> = (0..1000)
            .map(|n| ItemStack::new(&format!("minecraft:item_{}", n % 400), 1))
            .collect();
        let t = std::time::Instant::now();
        let a = stacks
            .iter()
            .filter(|s| recipes.cook_scan("smelting", s).cloned().is_some())
            .count();
        let before = t.elapsed().as_secs_f64() * 1e3;
        recipes.cook("smelting", &stacks[0]);
        let t = std::time::Instant::now();
        let b = stacks
            .iter()
            .filter(|s| recipes.cook("smelting", s).is_some())
            .count();
        let after = t.elapsed().as_secs_f64() * 1e3;
        assert_eq!(a, b);
        println!("1000 furnace lookups (scan and clone -> index): {before:.3} ms -> {after:.3} ms");
    }
    fn run(special: &Special, cells: &[ItemStack]) -> Option<ItemStack> {
        let recipes = Recipes {
            crafting: Vec::new(),
            cooking: Vec::new(),
            machine: Vec::new(),
            meta: HashMap::new(),
            facts: HashMap::new(),
            curses: HashSet::new(),
            cook_index: Default::default(),
        };
        let ctx = Context {
            recipes: &recipes,
            map_scale: &|_| None,
        };
        let mut grid: Vec<Option<ItemStack>> = cells.iter().cloned().map(Some).collect();
        grid.resize(9, None);
        let input = Input::of(&grid, 3)?;
        special
            .matches(&ctx, &input)
            .then(|| special.assemble(&ctx, &input))
            .flatten()
    }
    #[test]
    fn banner_duplicate_copies_the_patterns_of_the_same_colour() {
        let recipe = Special::BannerDuplicate {
            banner: ingredient(&["minecraft:white_banner"]),
            result: "minecraft:white_banner".into(),
        };
        let source = patterned("minecraft:white_banner");
        let blank = ItemStack::new("minecraft:white_banner", 1);
        let out = run(&recipe, &[source.clone(), blank.clone()]).expect("copy");
        assert_eq!(out.data().unwrap().banner_patterns.len(), 1);
        // Two blanks, or two patterned banners, do not match.
        assert!(run(&recipe, &[blank.clone(), blank.clone()]).is_none());
        assert!(run(&recipe, &[source.clone(), source.clone()]).is_none());
        // Another colour is not in the recipe's ingredient.
        let other = ItemStack::new("minecraft:red_banner", 1);
        assert!(run(&recipe, &[source, other]).is_none());
    }
    #[test]
    fn shield_decoration_needs_a_bare_shield() {
        let recipe = Special::ShieldDecoration {
            banner: ingredient(&["minecraft:red_banner"]),
            target: ingredient(&["minecraft:shield"]),
            result: "minecraft:shield".into(),
        };
        let banner = patterned("minecraft:red_banner");
        let shield = ItemStack::new("minecraft:shield", 1);
        let out = run(&recipe, &[banner.clone(), shield]).expect("decorated");
        let data = out.data().unwrap();
        assert_eq!(data.base_color.as_deref(), Some("red"));
        assert_eq!(data.banner_patterns.len(), 1);
        // A shield that already has patterns cannot be decorated again.
        assert!(run(&recipe, &[banner, out]).is_none());
    }
    #[test]
    fn shapeless_matching_assigns_each_stack_one_ingredient() {
        let any = ingredient(&["a", "b"]);
        let only_a = ingredient(&["a"]);
        let (a, b) = (ItemStack::new("a", 1), ItemStack::new("b", 1));
        // Greedy matching would give `a` to the first ingredient and fail.
        assert!(matches_all(&[any.clone(), only_a.clone()], &[&a, &b]));
        assert!(!matches_all(&[only_a.clone(), only_a], &[&a, &b]));
        assert!(!matches_all(&[any], &[&a, &b]));
    }
    #[test]
    fn dyes_blend_like_dyed_item_color() {
        // One red dye on an undyed item gives the dye's own colour.
        assert_eq!(apply_dyes(None, &[14]), 0xB02E26);
    }
}

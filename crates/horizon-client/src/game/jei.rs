//! Built-in JEI / REI item and recipe browser sidebar overlay for `Screen::Inventory`
//! and `Screen::Creative`.
use super::{Game, Screen};
use horizon_client::{
    crafting::{Ingredient, Recipe},
    creative::Creative,
    hud::{HudBatch, rgba},
    items::ItemStack,
    stations::Smithing,
};
use std::{sync::Arc, time::Instant};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecipeMode {
    Recipes,
    Uses,
}

#[derive(Clone, Debug)]
enum JeiRecipe {
    Crafting {
        id: String,
        shaped: Option<(usize, usize)>,
        cells: Vec<Option<Ingredient>>,
        result: ItemStack,
    },
    Cooking {
        kind: String,
        ingredient: Ingredient,
        result: ItemStack,
        time: u32,
        xp: f32,
    },
    Stonecutting {
        input: Ingredient,
        result: ItemStack,
    },
    Smithing {
        template: Ingredient,
        base: Ingredient,
        addition: Ingredient,
        result: ItemStack,
    },
}

#[derive(Clone, Debug)]
pub(super) struct JeiModal {
    pub item: String,
    pub mode: RecipeMode,
    pub page: usize,
    recipes: Vec<JeiRecipe>,
}

pub(super) struct JeiState {
    pub search: String,
    pub focused: bool,
    pub page: usize,
    pub modal: Option<JeiModal>,
    cached_query: Option<String>,
    cached_items: Vec<ItemStack>,
    started: Instant,
}

impl Default for JeiState {
    fn default() -> Self {
        Self {
            search: String::new(),
            focused: false,
            page: 0,
            modal: None,
            cached_query: None,
            cached_items: Vec::new(),
            started: Instant::now(),
        }
    }
}

struct PanelLayout {
    x0: f32,
    y0: f32,
    width: f32,
    cols: usize,
    rows: usize,
    grid_x: f32,
    grid_y: f32,
    search_x: f32,
    search_y: f32,
    search_w: f32,
}

impl Game<'_> {
    fn jei_layout(&self, batch: &HudBatch) -> Option<PanelLayout> {
        let (ox, _oy) = self.inventory_origin(batch);
        let win_w = if self.screen == Screen::Creative {
            Creative::WIDTH
        } else {
            match self.station.as_ref().map(|s| s.kind) {
                Some(super::stations::Kind::Merchant) => 226.0,
                Some(super::stations::Kind::Beacon) => 230.0,
                _ => 176.0,
            }
        };
        let x0 = (ox + win_w + 8.0).floor();
        let avail_w = batch.width - x0 - 4.0;
        let cols = ((avail_w / 18.0).floor() as usize).clamp(0, 14);
        if cols < 1 {
            return None;
        }
        let width = cols as f32 * 18.0 + 4.0;
        let y0 = 6.0;
        let avail_h = batch.height - y0 - 40.0;
        let rows = ((avail_h / 18.0).floor() as usize).clamp(3, 22);
        let grid_x = x0 + 2.0;
        let grid_y = y0 + 18.0;
        let search_x = x0 + 2.0;
        let search_y = grid_y + rows as f32 * 18.0 + 4.0;
        let search_w = cols as f32 * 18.0;
        Some(PanelLayout {
            x0,
            y0,
            width,
            cols,
            rows,
            grid_x,
            grid_y,
            search_x,
            search_y,
            search_w,
        })
    }

    fn ensure_jei_items(&mut self) {
        if self.jei.cached_query.as_deref() == Some(self.jei.search.as_str())
            && !self.jei.cached_items.is_empty()
        {
            return;
        }
        let query = self.jei.search.trim().to_ascii_lowercase();
        let mut all = Vec::new();
        let mut seen: std::collections::HashSet<Arc<str>> = std::collections::HashSet::new();
        for tab in &self.creative_screen.tabs {
            for stack in &tab.items {
                if seen.insert(stack.item.clone()) {
                    all.push(stack.clone());
                }
            }
        }
        let mut extra_ids: Vec<&Arc<str>> = self
            .res
            .items()
            .items
            .keys()
            .filter(|id| id.as_ref() != "minecraft:air" && !seen.contains(*id))
            .collect();
        extra_ids.sort();
        for id in extra_ids {
            seen.insert(id.clone());
            all.push(ItemStack::new(&**id, 1));
        }

        if query.is_empty() {
            self.jei.cached_items = all;
            self.jei.cached_query = Some(self.jei.search.clone());
            return;
        }

        let mut mod_filters = Vec::new();
        let mut tag_filters = Vec::new();
        let mut text_terms = Vec::new();
        for tok in query.split_whitespace() {
            if let Some(m) = tok.strip_prefix('@') {
                if !m.is_empty() {
                    mod_filters.push(m);
                }
            } else if let Some(t) = tok.strip_prefix('#') {
                if !t.is_empty() {
                    tag_filters.push(t);
                }
            } else {
                text_terms.push(tok);
            }
        }

        let names = &self.pack.names;
        let item_tags = &self.res.items().item_tag_order;
        let filtered: Vec<ItemStack> = all
            .into_iter()
            .filter(|stack| {
                let id = stack.item.as_ref();
                let ns = id.split_once(':').map_or("minecraft", |(n, _)| n);
                for mf in &mod_filters {
                    let pretty = ns.replace('_', " ");
                    if !ns.to_ascii_lowercase().contains(mf)
                        && !pretty.to_ascii_lowercase().contains(mf)
                    {
                        return false;
                    }
                }
                for tf in &tag_filters {
                    let matches_tag = item_tags.iter().any(|(tag_name, members)| {
                        tag_name.to_ascii_lowercase().contains(tf)
                            && members.iter().any(|m| m == id)
                    });
                    if !matches_tag {
                        return false;
                    }
                }
                if !text_terms.is_empty() {
                    let disp = names.item(&stack.item).to_ascii_lowercase();
                    let id_low = id.to_ascii_lowercase();
                    for term in &text_terms {
                        if !disp.contains(term) && !id_low.contains(term) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();

        self.jei.cached_items = filtered;
        self.jei.cached_query = Some(self.jei.search.clone());
    }

    fn collect_jei_recipes(&self, target_item: &str, mode: RecipeMode) -> Vec<JeiRecipe> {
        let mut out = Vec::new();
        let recipes = self.res.recipes();
        for (id, r) in &recipes.crafting {
            match r {
                Recipe::Shaped {
                    width,
                    height,
                    cells,
                    result,
                } => {
                    let res = result.create();
                    let matches = match mode {
                        RecipeMode::Recipes => &*res.item == target_item,
                        RecipeMode::Uses => cells
                            .iter()
                            .flatten()
                            .any(|ing| ing.0.contains(target_item)),
                    };
                    if matches && &*res.item != "minecraft:air" {
                        out.push(JeiRecipe::Crafting {
                            id: id.clone(),
                            shaped: Some((*width, *height)),
                            cells: cells.clone(),
                            result: res,
                        });
                    }
                }
                Recipe::Shapeless {
                    ingredients,
                    result,
                } => {
                    let res = result.create();
                    let matches = match mode {
                        RecipeMode::Recipes => &*res.item == target_item,
                        RecipeMode::Uses => {
                            ingredients.iter().any(|ing| ing.0.contains(target_item))
                        }
                    };
                    if matches && &*res.item != "minecraft:air" {
                        out.push(JeiRecipe::Crafting {
                            id: id.clone(),
                            shaped: None,
                            cells: ingredients.iter().cloned().map(Some).collect(),
                            result: res,
                        });
                    }
                }
                Recipe::Special(_) => {}
            }
        }
        for cook in &recipes.cooking {
            let matches = match mode {
                RecipeMode::Recipes => &*cook.result.item == target_item,
                RecipeMode::Uses => cook.ingredient.0.contains(target_item),
            };
            if matches && &*cook.result.item != "minecraft:air" {
                out.push(JeiRecipe::Cooking {
                    kind: cook.kind.clone(),
                    ingredient: cook.ingredient.clone(),
                    result: cook.result.clone(),
                    time: cook.time,
                    xp: cook.experience,
                });
            }
        }
        let st = &self.res.stations().recipes;
        for sc in &st.stonecutting {
            let matches = match mode {
                RecipeMode::Recipes => &*sc.result.item == target_item,
                RecipeMode::Uses => sc.input.0.contains(target_item),
            };
            if matches && &*sc.result.item != "minecraft:air" {
                out.push(JeiRecipe::Stonecutting {
                    input: sc.input.clone(),
                    result: sc.result.clone(),
                });
            }
        }
        for sm in &st.smithing {
            if let Smithing::Transform {
                template,
                base,
                addition,
                result,
            } = sm
            {
                let matches = match mode {
                    RecipeMode::Recipes => result == target_item,
                    RecipeMode::Uses => {
                        template.0.contains(target_item)
                            || base.0.contains(target_item)
                            || addition.0.contains(target_item)
                    }
                };
                if matches && result != "minecraft:air" {
                    out.push(JeiRecipe::Smithing {
                        template: template.clone(),
                        base: base.clone(),
                        addition: addition.clone(),
                        result: ItemStack::new(result, 1),
                    });
                }
            }
        }
        out
    }

    pub(super) fn open_jei_modal(&mut self, item: &str, mode: RecipeMode) -> bool {
        if item.is_empty() || item == "minecraft:air" {
            return false;
        }
        let recipes = self.collect_jei_recipes(item, mode);
        self.jei.modal = Some(JeiModal {
            item: item.to_owned(),
            mode,
            page: 0,
            recipes,
        });
        true
    }

    fn cycle_ingredient(&self, ing: &Ingredient) -> Option<ItemStack> {
        if ing.1.is_empty() {
            return None;
        }
        let secs = self.jei.started.elapsed().as_secs() as usize;
        let idx = secs % ing.1.len();
        Some(ItemStack::new(&ing.1[idx], 1))
    }

    pub(super) fn draw_jei(&mut self, batch: &mut HudBatch) {
        let Some(layout) = self.jei_layout(batch) else {
            return;
        };
        self.ensure_jei_items();
        let per_page = (layout.cols * layout.rows).max(1);
        let total = self.jei.cached_items.len();
        let total_pages = total.div_ceil(per_page).max(1);
        if self.jei.page >= total_pages {
            self.jei.page = total_pages - 1;
        }

        // Panel backdrop
        let panel_h = (layout.search_y + 18.0 - layout.y0).floor();
        batch.fill(
            &self.gui.atlas,
            layout.x0,
            layout.y0,
            layout.width,
            panel_h,
            rgba(18, 18, 22, 185),
        );

        // Header: [<]  JEI (1/N)  [>]
        let header_y = layout.y0 + 4.0;
        let prev_x = layout.x0 + 4.0;
        let next_x = layout.x0 + layout.width - 18.0;
        let (cx, cy) = self.cursor;
        let prev_hov =
            cx >= prev_x && cx < prev_x + 14.0 && cy >= layout.y0 + 2.0 && cy < layout.y0 + 16.0;
        let next_hov =
            cx >= next_x && cx < next_x + 14.0 && cy >= layout.y0 + 2.0 && cy < layout.y0 + 16.0;
        batch.fill(
            &self.gui.atlas,
            prev_x,
            layout.y0 + 2.0,
            14.0,
            13.0,
            if prev_hov {
                rgba(90, 90, 110, 220)
            } else {
                rgba(45, 45, 55, 200)
            },
        );
        batch.text(&self.gui.atlas, "<", prev_x + 4.0, header_y, u32::MAX, true);
        batch.fill(
            &self.gui.atlas,
            next_x,
            layout.y0 + 2.0,
            14.0,
            13.0,
            if next_hov {
                rgba(90, 90, 110, 220)
            } else {
                rgba(45, 45, 55, 200)
            },
        );
        batch.text(&self.gui.atlas, ">", next_x + 4.0, header_y, u32::MAX, true);

        let page_label = format!("JEI {}/{}", self.jei.page + 1, total_pages);
        let lw = self.gui.atlas.font.width(&page_label);
        let lx = (layout.x0 + (layout.width - lw) * 0.5).floor();
        batch.text(&self.gui.atlas, &page_label, lx, header_y, u32::MAX, true);

        // Item grid
        let start = self.jei.page * per_page;
        let end = (start + per_page).min(total);
        let page_slice: Vec<ItemStack> = self.jei.cached_items[start..end].to_vec();
        let mut hovered_jei: Option<ItemStack> = None;
        let modal_open = self.jei.modal.is_some();

        for (idx, stack) in page_slice.into_iter().enumerate() {
            let r = idx / layout.cols;
            let c = idx % layout.cols;
            let x = layout.grid_x + c as f32 * 18.0;
            let y = layout.grid_y + r as f32 * 18.0;
            self.draw_item(batch, &stack, x, y);
            if !modal_open && cx >= x && cx < x + 16.0 && cy >= y && cy < y + 16.0 {
                batch.fill(&self.gui.atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 110));
                hovered_jei = Some(stack);
            }
        }

        // Bottom search box
        let border_col = if self.jei.focused {
            rgba(255, 215, 70, 255)
        } else {
            rgba(110, 110, 125, 220)
        };
        batch.fill(
            &self.gui.atlas,
            layout.search_x,
            layout.search_y,
            layout.search_w,
            14.0,
            border_col,
        );
        batch.fill(
            &self.gui.atlas,
            layout.search_x + 1.0,
            layout.search_y + 1.0,
            layout.search_w - 2.0,
            12.0,
            rgba(10, 10, 14, 240),
        );
        if self.jei.search.is_empty() && !self.jei.focused {
            batch.text(
                &self.gui.atlas,
                "Buscar (@mod, #tag)...",
                layout.search_x + 4.0,
                layout.search_y + 3.0,
                rgba(145, 145, 155, 255),
                false,
            );
        } else {
            let cursor_char = if self.jei.focused
                && (self.jei.started.elapsed().as_millis() / 500).is_multiple_of(2)
            {
                "_"
            } else {
                ""
            };
            let shown = format!("{}{cursor_char}", self.jei.search);
            batch.text(
                &self.gui.atlas,
                &shown,
                layout.search_x + 4.0,
                layout.search_y + 3.0,
                u32::MAX,
                true,
            );
        }

        if let Some(stack) = hovered_jei {
            let mut lines = self.stack_tooltip(&stack);
            if self.creative {
                lines.push((
                    "[Clique Esq: Pegar | Shift+Esq: 64 | R: Receita | U: Usos]".into(),
                    rgba(160, 160, 175, 255),
                ));
            } else {
                lines.push((
                    "[Clique Esq / R: Receita | Clique Dir / U: Usos]".into(),
                    rgba(160, 160, 175, 255),
                ));
            }
            self.draw_tooltip_lines(batch, &lines, cx, cy);
        }

        if self.jei.modal.is_some() {
            self.draw_jei_modal(batch);
        }
    }

    fn modal_rect(batch: &HudBatch) -> (f32, f32, f32, f32) {
        let (mw, mh) = (196.0, 132.0);
        let mx = ((batch.width - mw) * 0.5).floor();
        let my = ((batch.height - mh) * 0.5).floor();
        (mx, my, mw, mh)
    }

    fn draw_jei_modal(&mut self, batch: &mut HudBatch) {
        let Some(modal) = self.jei.modal.clone() else {
            return;
        };
        let (mx, my, mw, mh) = Self::modal_rect(batch);
        let (cx, cy) = self.cursor;

        // Dim background + bordered modal box
        batch.fill(
            &self.gui.atlas,
            0.0,
            0.0,
            batch.width,
            batch.height,
            rgba(0, 0, 0, 140),
        );
        batch.fill(
            &self.gui.atlas,
            mx - 1.0,
            my - 1.0,
            mw + 2.0,
            mh + 2.0,
            rgba(220, 220, 230, 255),
        );
        batch.fill(&self.gui.atlas, mx, my, mw, mh, rgba(38, 38, 46, 250));

        let item_title = self.pack.names.item(&modal.item);
        let prefix = match modal.mode {
            RecipeMode::Recipes => "Receita",
            RecipeMode::Uses => "Usos",
        };
        let header = format!("{prefix}: {item_title}");
        batch.text(
            &self.gui.atlas,
            &header,
            mx + 6.0,
            my + 6.0,
            rgba(255, 230, 110, 255),
            true,
        );

        // Close button [X]
        let close_x = mx + mw - 18.0;
        let close_y = my + 4.0;
        let close_hov =
            cx >= close_x && cx < close_x + 14.0 && cy >= close_y && cy < close_y + 13.0;
        batch.fill(
            &self.gui.atlas,
            close_x,
            close_y,
            14.0,
            13.0,
            if close_hov {
                rgba(190, 60, 60, 240)
            } else {
                rgba(90, 40, 40, 220)
            },
        );
        batch.text(
            &self.gui.atlas,
            "X",
            close_x + 4.0,
            close_y + 2.0,
            u32::MAX,
            true,
        );

        if modal.recipes.is_empty() {
            let msg = match modal.mode {
                RecipeMode::Recipes => "Nenhuma receita encontrada para este item.",
                RecipeMode::Uses => "Nenhum uso encontrado para este item.",
            };
            batch.text(
                &self.gui.atlas,
                msg,
                mx + 12.0,
                my + 60.0,
                rgba(190, 190, 200, 255),
                false,
            );
            return;
        }

        let total = modal.recipes.len();
        let page = modal.page.min(total - 1);
        let recipe = &modal.recipes[page];

        // Subheader with recipe type and [<] page/total [>]
        let kind_label = match recipe {
            JeiRecipe::Crafting {
                shaped: Some(_), ..
            } => "Bancada (Shaped)",
            JeiRecipe::Crafting { shaped: None, .. } => "Bancada (Shapeless)",
            JeiRecipe::Cooking { kind, .. } => match kind.as_str() {
                "minecraft:blasting" => "Alto-Forno (Blasting)",
                "minecraft:smoking" => "Defumador (Smoking)",
                "minecraft:campfire_cooking" => "Fogueira (Campfire)",
                _ => "Fornalha (Smelting)",
            },
            JeiRecipe::Stonecutting { .. } => "Cortador de Pedras",
            JeiRecipe::Smithing { .. } => "Mesa de Ferraria",
        };
        batch.text(
            &self.gui.atlas,
            kind_label,
            mx + 8.0,
            my + 22.0,
            rgba(180, 210, 255, 255),
            false,
        );

        let nav_y = my + 20.0;
        let prev_x = mx + mw - 66.0;
        let next_x = mx + mw - 18.0;
        batch.fill(
            &self.gui.atlas,
            prev_x,
            nav_y,
            14.0,
            13.0,
            rgba(65, 65, 80, 230),
        );
        batch.text(
            &self.gui.atlas,
            "<",
            prev_x + 4.0,
            nav_y + 2.0,
            u32::MAX,
            true,
        );
        let pg_str = format!("{}/{}", page + 1, total);
        batch.text(
            &self.gui.atlas,
            &pg_str,
            prev_x + 18.0,
            nav_y + 2.0,
            u32::MAX,
            false,
        );
        batch.fill(
            &self.gui.atlas,
            next_x,
            nav_y,
            14.0,
            13.0,
            rgba(65, 65, 80, 230),
        );
        batch.text(
            &self.gui.atlas,
            ">",
            next_x + 4.0,
            nav_y + 2.0,
            u32::MAX,
            true,
        );

        let mut hovered_slot: Option<ItemStack> = None;
        let mut draw_slot =
            |game: &mut Self, batch: &mut HudBatch, sx: f32, sy: f32, st: Option<ItemStack>| {
                batch.fill(
                    &game.gui.atlas,
                    sx - 1.0,
                    sy - 1.0,
                    18.0,
                    18.0,
                    rgba(20, 20, 26, 255),
                );
                batch.fill(
                    &game.gui.atlas,
                    sx,
                    sy,
                    16.0,
                    16.0,
                    rgba(139, 139, 139, 255),
                );
                if let Some(stack) = st {
                    game.draw_item(batch, &stack, sx, sy);
                    if cx >= sx && cx < sx + 16.0 && cy >= sy && cy < sy + 16.0 {
                        batch.fill(
                            &game.gui.atlas,
                            sx,
                            sy,
                            16.0,
                            16.0,
                            rgba(255, 255, 255, 110),
                        );
                        hovered_slot = Some(stack);
                    }
                }
            };

        let gx = mx + 26.0;
        let gy = my + 44.0;
        match recipe {
            JeiRecipe::Crafting {
                id,
                shaped,
                cells,
                result,
            } => {
                let (w, h) = shaped.unwrap_or((3, 3));
                for r in 0..3 {
                    for c in 0..3 {
                        let sx = gx + c as f32 * 18.0;
                        let sy = gy + r as f32 * 18.0;
                        let ing = if shaped.is_some() {
                            if r < h && c < w {
                                cells.get(r * w + c).and_then(|x| x.as_ref())
                            } else {
                                None
                            }
                        } else {
                            cells.get(r * 3 + c).and_then(|x| x.as_ref())
                        };
                        let stack = ing.and_then(|i| self.cycle_ingredient(i));
                        draw_slot(self, batch, sx, sy, stack);
                    }
                }
                batch.text(&self.gui.atlas, "->", gx + 64.0, gy + 22.0, u32::MAX, true);
                draw_slot(self, batch, gx + 88.0, gy + 18.0, Some(result.clone()));
                batch.text(
                    &self.gui.atlas,
                    id,
                    mx + 8.0,
                    my + mh - 14.0,
                    rgba(140, 140, 155, 255),
                    false,
                );
            }
            JeiRecipe::Cooking {
                ingredient,
                result,
                time,
                xp,
                ..
            } => {
                let stack = self.cycle_ingredient(ingredient);
                draw_slot(self, batch, gx + 18.0, gy + 18.0, stack);
                batch.text(
                    &self.gui.atlas,
                    "->",
                    gx + 46.0,
                    gy + 22.0,
                    rgba(255, 180, 70, 255),
                    true,
                );
                draw_slot(self, batch, gx + 72.0, gy + 18.0, Some(result.clone()));
                let info = format!("Tempo: {:.1}s   XP: {:.1}", *time as f32 / 20.0, xp);
                batch.text(
                    &self.gui.atlas,
                    &info,
                    mx + 12.0,
                    my + mh - 16.0,
                    rgba(180, 180, 195, 255),
                    false,
                );
            }
            JeiRecipe::Stonecutting { input, result } => {
                let stack = self.cycle_ingredient(input);
                draw_slot(self, batch, gx + 18.0, gy + 18.0, stack);
                batch.text(&self.gui.atlas, "->", gx + 46.0, gy + 22.0, u32::MAX, true);
                draw_slot(self, batch, gx + 72.0, gy + 18.0, Some(result.clone()));
            }
            JeiRecipe::Smithing {
                template,
                base,
                addition,
                result,
            } => {
                let s0 = self.cycle_ingredient(template);
                let s1 = self.cycle_ingredient(base);
                let s2 = self.cycle_ingredient(addition);
                draw_slot(self, batch, gx, gy + 18.0, s0);
                draw_slot(self, batch, gx + 18.0, gy + 18.0, s1);
                draw_slot(self, batch, gx + 36.0, gy + 18.0, s2);
                batch.text(&self.gui.atlas, "->", gx + 60.0, gy + 22.0, u32::MAX, true);
                draw_slot(self, batch, gx + 82.0, gy + 18.0, Some(result.clone()));
            }
        }

        if let Some(stack) = hovered_slot {
            let mut lines = self.stack_tooltip(&stack);
            lines.push((
                "[Clique Esq / R: Receita | Clique Dir / U: Usos]".into(),
                rgba(160, 160, 175, 255),
            ));
            self.draw_tooltip_lines(batch, &lines, cx, cy);
        }
    }

    fn give_creative_jei_item(&mut self, item_id: &str, full_stack: bool) {
        let max_stack = self
            .res
            .items()
            .get(item_id)
            .map_or(64, |info| info.max_stack);
        let count = if full_stack { max_stack } else { 1 };
        let given = ItemStack::new(item_id, count);

        // Try merging into existing stack in hotbar (0..9) then main inventory (9..36)
        for i in (0..9).chain(9..36) {
            if let Some(existing) = &mut self.inventory[i]
                && existing.same_item_same_components(&given)
                && existing.count < max_stack
            {
                let add = (max_stack - existing.count).min(count);
                existing.count += add;
                return;
            }
        }
        // Otherwise place into first empty slot in hotbar (0..9) then main inventory (9..36)
        for i in (0..9).chain(9..36) {
            if self.inventory[i].is_none() {
                self.inventory[i] = Some(given);
                return;
            }
        }
        // Fallback: put on cursor
        self.carried = Some(given);
    }

    pub(super) fn jei_click(&mut self, right: bool) -> bool {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            return false;
        }
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return false;
        };
        let batch = HudBatch::new(size);
        let (cx, cy) = self.cursor;

        // 1. Handle clicks when Recipe Modal is open
        if let Some(modal) = self.jei.modal.clone() {
            let (mx, my, mw, mh) = Self::modal_rect(&batch);
            if cx < mx || cx >= mx + mw || cy < my || cy >= my + mh {
                // Clicked outside modal -> close modal
                self.jei.modal = None;
                return true;
            }
            let close_x = mx + mw - 18.0;
            let close_y = my + 4.0;
            if cx >= close_x && cx < close_x + 14.0 && cy >= close_y && cy < close_y + 13.0 {
                self.jei.modal = None;
                return true;
            }
            if !modal.recipes.is_empty() {
                let total = modal.recipes.len();
                let page = modal.page.min(total - 1);
                let nav_y = my + 20.0;
                let prev_x = mx + mw - 66.0;
                let next_x = mx + mw - 18.0;
                if cx >= prev_x && cx < prev_x + 14.0 && cy >= nav_y && cy < nav_y + 13.0 {
                    if let Some(m) = &mut self.jei.modal {
                        m.page = if page == 0 { total - 1 } else { page - 1 };
                    }
                    return true;
                }
                if cx >= next_x && cx < next_x + 14.0 && cy >= nav_y && cy < nav_y + 13.0 {
                    if let Some(m) = &mut self.jei.modal {
                        m.page = (page + 1) % total;
                    }
                    return true;
                }
                if let Some(clicked_item) = self.hovered_modal_item(&batch, &modal.recipes[page]) {
                    let mode = if right {
                        RecipeMode::Uses
                    } else {
                        RecipeMode::Recipes
                    };
                    self.open_jei_modal(&clicked_item.item, mode);
                    return true;
                }
            }
            return true;
        }

        // 2. Check JEI panel bounds
        let Some(layout) = self.jei_layout(&batch) else {
            self.jei.focused = false;
            return false;
        };

        // Search box click
        if cx >= layout.search_x
            && cx < layout.search_x + layout.search_w
            && cy >= layout.search_y
            && cy < layout.search_y + 14.0
        {
            self.jei.focused = true;
            if right {
                self.jei.search.clear();
                self.jei.cached_query = None;
                self.jei.page = 0;
            }
            return true;
        }
        self.jei.focused = false;

        let panel_h = (layout.search_y + 18.0 - layout.y0).floor();
        if cx < layout.x0
            || cx >= layout.x0 + layout.width
            || cy < layout.y0
            || cy >= layout.y0 + panel_h
        {
            return false;
        }

        self.ensure_jei_items();
        let per_page = (layout.cols * layout.rows).max(1);
        let total = self.jei.cached_items.len();
        let total_pages = total.div_ceil(per_page).max(1);

        // Header `<` and `>` buttons
        let prev_x = layout.x0 + 4.0;
        let next_x = layout.x0 + layout.width - 18.0;
        if cx >= prev_x && cx < prev_x + 14.0 && cy >= layout.y0 + 2.0 && cy < layout.y0 + 16.0 {
            self.jei.page = if self.jei.page == 0 {
                total_pages - 1
            } else {
                self.jei.page - 1
            };
            return true;
        }
        if cx >= next_x && cx < next_x + 14.0 && cy >= layout.y0 + 2.0 && cy < layout.y0 + 16.0 {
            self.jei.page = (self.jei.page + 1) % total_pages;
            return true;
        }

        // Grid item click
        if cx >= layout.grid_x && cy >= layout.grid_y {
            let c = ((cx - layout.grid_x) / 18.0).floor() as usize;
            let r = ((cy - layout.grid_y) / 18.0).floor() as usize;
            if c < layout.cols && r < layout.rows {
                let idx = self.jei.page * per_page + r * layout.cols + c;
                if let Some(stack) = self.jei.cached_items.get(idx).cloned() {
                    if self.creative && !right {
                        let shift = self.has_shift_down();
                        self.give_creative_jei_item(&stack.item, shift);
                    } else {
                        let mode = if right && !self.creative {
                            RecipeMode::Uses
                        } else {
                            RecipeMode::Recipes
                        };
                        self.open_jei_modal(&stack.item, mode);
                    }
                    return true;
                }
            }
        }
        true
    }

    fn hovered_modal_item(&self, batch: &HudBatch, recipe: &JeiRecipe) -> Option<ItemStack> {
        let (mx, my, _, _) = Self::modal_rect(batch);
        let (cx, cy) = self.cursor;
        let gx = mx + 26.0;
        let gy = my + 44.0;
        let hit = |sx: f32, sy: f32| cx >= sx && cx < sx + 16.0 && cy >= sy && cy < sy + 16.0;

        match recipe {
            JeiRecipe::Crafting {
                shaped,
                cells,
                result,
                ..
            } => {
                let (w, h) = shaped.unwrap_or((3, 3));
                for r in 0..3 {
                    for c in 0..3 {
                        let sx = gx + c as f32 * 18.0;
                        let sy = gy + r as f32 * 18.0;
                        if hit(sx, sy) {
                            let ing = if shaped.is_some() {
                                if r < h && c < w {
                                    cells.get(r * w + c).and_then(|x| x.as_ref())
                                } else {
                                    None
                                }
                            } else {
                                cells.get(r * 3 + c).and_then(|x| x.as_ref())
                            };
                            return ing.and_then(|i| self.cycle_ingredient(i));
                        }
                    }
                }
                if hit(gx + 88.0, gy + 18.0) {
                    return Some(result.clone());
                }
            }
            JeiRecipe::Cooking {
                ingredient, result, ..
            }
            | JeiRecipe::Stonecutting {
                input: ingredient,
                result,
            } => {
                if hit(gx + 18.0, gy + 18.0) {
                    return self.cycle_ingredient(ingredient);
                }
                if hit(gx + 72.0, gy + 18.0) {
                    return Some(result.clone());
                }
            }
            JeiRecipe::Smithing {
                template,
                base,
                addition,
                result,
            } => {
                if hit(gx, gy + 18.0) {
                    return self.cycle_ingredient(template);
                }
                if hit(gx + 18.0, gy + 18.0) {
                    return self.cycle_ingredient(base);
                }
                if hit(gx + 36.0, gy + 18.0) {
                    return self.cycle_ingredient(addition);
                }
                if hit(gx + 82.0, gy + 18.0) {
                    return Some(result.clone());
                }
            }
        }
        None
    }

    pub(super) fn jei_scroll(&mut self, delta: f32) -> bool {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            return false;
        }
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return false;
        };
        let batch = HudBatch::new(size);
        let (cx, cy) = self.cursor;

        if let Some(modal) = &mut self.jei.modal {
            let total = modal.recipes.len();
            if total > 1 {
                if delta < 0.0 {
                    modal.page = (modal.page + 1) % total;
                } else {
                    modal.page = if modal.page == 0 {
                        total - 1
                    } else {
                        modal.page - 1
                    };
                }
            }
            return true;
        }

        let Some(layout) = self.jei_layout(&batch) else {
            return false;
        };
        let panel_h = (layout.search_y + 18.0 - layout.y0).floor();
        if cx < layout.x0
            || cx >= layout.x0 + layout.width
            || cy < layout.y0
            || cy >= layout.y0 + panel_h
        {
            return false;
        }
        self.ensure_jei_items();
        let per_page = (layout.cols * layout.rows).max(1);
        let total_pages = self.jei.cached_items.len().div_ceil(per_page).max(1);
        if total_pages > 1 {
            if delta < 0.0 {
                self.jei.page = (self.jei.page + 1) % total_pages;
            } else {
                self.jei.page = if self.jei.page == 0 {
                    total_pages - 1
                } else {
                    self.jei.page - 1
                };
            }
        }
        true
    }

    fn hovered_item_for_jei(&mut self, batch: &HudBatch) -> Option<ItemStack> {
        let (cx, cy) = self.cursor;
        if let Some(modal) = self.jei.modal.clone() {
            if !modal.recipes.is_empty() {
                let page = modal.page.min(modal.recipes.len() - 1);
                if let Some(st) = self.hovered_modal_item(batch, &modal.recipes[page]) {
                    return Some(st);
                }
            }
            return None;
        }
        if let Some(layout) = self.jei_layout(batch) {
            self.ensure_jei_items();
            let per_page = (layout.cols * layout.rows).max(1);
            if cx >= layout.grid_x && cy >= layout.grid_y {
                let c = ((cx - layout.grid_x) / 18.0).floor() as usize;
                let r = ((cy - layout.grid_y) / 18.0).floor() as usize;
                if c < layout.cols && r < layout.rows {
                    let idx = self.jei.page * per_page + r * layout.cols + c;
                    if let Some(st) = self.jei.cached_items.get(idx) {
                        return Some(st.clone());
                    }
                }
            }
        }
        if self.screen == Screen::Creative && !self.creative_screen.tab().inventory {
            let (ox, oy) = Self::creative_origin(batch);
            let items = self.creative_screen.items(&self.pack.names);
            let first = self.creative_screen.first_row(&self.pack.names);
            for r in 0..5 {
                for c in 0..9 {
                    let (x, y) = (ox + 9.0 + c as f32 * 18.0, oy + 18.0 + r as f32 * 18.0);
                    if cx >= x && cx < x + 16.0 && cy >= y && cy < y + 16.0 {
                        return items.get((first + r) * 9 + c).cloned();
                    }
                }
            }
        }
        self.slot_at(batch, cx, cy).and_then(|s| self.slot_view(s))
    }

    pub(super) fn jei_key(&mut self, event: &KeyEvent) -> bool {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative)
            || event.state != ElementState::Pressed
        {
            return false;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return false;
        };

        // 1. If JEI search box is focused, handle typing / backspace / Escape
        if self.jei.focused {
            match code {
                KeyCode::Escape | KeyCode::Enter => {
                    self.jei.focused = false;
                    return true;
                }
                KeyCode::Backspace => {
                    self.jei.search.pop();
                    self.jei.cached_query = None;
                    self.jei.page = 0;
                    return true;
                }
                _ => {
                    if let Some(text) = &event.text {
                        let mut changed = false;
                        for ch in text.chars() {
                            if !ch.is_control() && self.jei.search.len() < 48 {
                                self.jei.search.push(ch);
                                changed = true;
                            }
                        }
                        if changed {
                            self.jei.cached_query = None;
                            self.jei.page = 0;
                        }
                    }
                    return true;
                }
            }
        }

        // 2. If Recipe Modal is open, Escape closes it (instead of closing the inventory)
        if self.jei.modal.is_some() && code == KeyCode::Escape {
            self.jei.modal = None;
            return true;
        }

        // 3. `R` (Recipes) or `U` (Uses) over any hovered item (when Creative search tab isn't typing)
        let creative_search_active =
            self.screen == Screen::Creative && self.creative_screen.tab().search;
        if !creative_search_active && matches!(code, KeyCode::KeyR | KeyCode::KeyU) {
            let Some(size) = self
                .renderer
                .as_ref()
                .map(|r| [r.config.width, r.config.height])
            else {
                return false;
            };
            let batch = HudBatch::new(size);
            if let Some(hovered) = self.hovered_item_for_jei(&batch) {
                let mode = if code == KeyCode::KeyU {
                    RecipeMode::Uses
                } else {
                    RecipeMode::Recipes
                };
                return self.open_jei_modal(&hovered.item, mode);
            }
        }
        false
    }
}

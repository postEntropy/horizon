//! LaserIO: laser nodes and connectors that move items, fluids, energy and
//! chemicals between inventories with cards, filters and redstone control.
//!
//! A node's channels and the cards in its screen are what the port keeps; the
//! transfer itself runs in the network, which the sandbox cannot join.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const NODE_SCREEN: &str = "screen:laserio:node";
const CHANNELS: u32 = 8;

const MODES: [&str; 4] = ["Insert", "Extract", "Sensor", "Stock"];

const CARDS: [(&str, &str); 4] = [
    ("item", "Item"),
    ("fluid", "Fluid"),
    ("energy", "Energy"),
    ("chemical", "Chemical"),
];

const FILTERS: [(&str, &str); 5] = [
    ("basic", "Basic filter"),
    ("count", "Count filter"),
    ("mod", "Mod filter"),
    ("tag", "Tag filter"),
    ("nbt", "NBT filter"),
];

fn slot(id: u32, x: i32, y: i32, text: &str) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Slot,
        x,
        y,
        width: 18,
        height: 18,
        text: text.into(),
        value: 0,
        max_value: 1,
    }
}

fn node_screen(advanced: bool) -> ScreenSpec {
    let mut widgets = Vec::new();
    for channel in 0..CHANNELS {
        let column = channel % 4;
        let row = channel / 4;
        widgets.push(slot(
            channel,
            8 + column as i32 * 30,
            18 + row as i32 * 24,
            &format!("#{}", channel + 1),
        ));
        widgets.push(Widget {
            id: 100 + channel,
            kind: WidgetKind::Button,
            x: 26 + column as i32 * 30,
            y: 24 + row as i32 * 24,
            width: 10,
            height: 10,
            text: "…".into(),
            value: 0,
            max_value: 1,
        });
    }
    widgets.push(Widget {
        id: 200,
        kind: WidgetKind::Label,
        x: 8,
        y: 78,
        width: 160,
        height: 10,
        text: format!(
            "{} node · {CHANNELS} channels",
            if advanced { "Advanced" } else { "Basic" }
        ),
        value: 0,
        max_value: 0,
    });
    widgets.push(Widget {
        id: 201,
        kind: WidgetKind::Label,
        x: 8,
        y: 92,
        width: 160,
        height: 10,
        text: "Drop a card on a channel to set its mode.".into(),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: NODE_SCREEN.into(),
        title: if advanced {
            "Advanced Laser Node".into()
        } else {
            "Laser Node".into()
        },
        width: 176,
        height: 120,
        widgets,
    }
}

fn path_of(item: &str) -> &str {
    item.split([':', '[']).nth(1).unwrap_or(item)
}

struct LaserIo;

impl Mod for LaserIo {
    fn init() -> Result<(Self, Registration), String> {
        Ok((LaserIo, Registration::new().command("laserio")))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 4,
                ticks: true,
            })
            .collect()
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new()
            .block("laserio:laser_node")
            .block("laserio:laser_connector")
            .block("laserio:laser_connector_advanced")
            .item("laserio:laser_wrench")
            .item("laserio:overclocker_card")
            .item("laserio:overclocker_node")
            .item("laserio:logic_chip")
            .item("laserio:logic_chip_raw");
        for (filter, _) in FILTERS {
            hooks = hooks.item(&format!("laserio:filter_{filter}"));
        }
        for (resource, _) in CARDS {
            hooks = hooks.item(&format!("laserio:card_{resource}"));
            hooks = hooks.item(&format!("laserio:card_{resource}_extract"));
            hooks = hooks.item(&format!("laserio:card_{resource}_sensor"));
            hooks = hooks.item(&format!("laserio:card_{resource}_stock"));
        }
        hooks
            .item("laserio:card_redstone")
            .item("laserio:card_cloner")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = event.state.split([':', '[']).nth(1).unwrap_or(&event.state);
        let advanced = path == "laser_connector_advanced";
        if !matches!(
            path,
            "laser_node" | "laser_connector" | "laser_connector_advanced"
        ) {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&node_screen(advanced));
        EventOutcome::Consume
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != NODE_SCREEN || slot >= CHANNELS {
            return;
        }
        chat::show(&format!(
            "🔦 Channel {}: drop a card here — Insert, Extract, Sensor or Stock.",
            slot + 1
        ));
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if path == "laser_wrench" {
            chat::show(
                "🔧 Laser wrench: right-click a node to connect it, sneak-right-click to reset it.",
            );
            return EventOutcome::Consume;
        }
        if let Some(kind) = path.strip_prefix("filter_") {
            let name = FILTERS
                .iter()
                .find(|(filter, _)| *filter == kind)
                .map(|(_, name)| *name)
                .unwrap_or("Filter");
            chat::show(&format!(
                "🔦 {name}: right-click to open it and set what passes."
            ));
            return EventOutcome::Consume;
        }
        if let Some(rest) = path.strip_prefix("card_") {
            let (resource, mode) = match rest.strip_suffix("_extract") {
                Some(resource) => (resource, "Extract"),
                None => match rest.strip_suffix("_sensor") {
                    Some(resource) => (resource, "Sensor"),
                    None => match rest.strip_suffix("_stock") {
                        Some(resource) => (resource, "Stock"),
                        None => (rest, "Insert"),
                    },
                },
            };
            let name = CARDS
                .iter()
                .find(|(card, _)| *card == resource)
                .map(|(_, name)| *name)
                .unwrap_or("Redstone");
            chat::show(&format!(
                "🔦 {name} Card · {mode}: put it in a node channel to move {name} that way."
            ));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "laserio" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "LaserIO: {CHANNELS} channels per node, modes {}; cards for {}; filters: {}.",
            MODES.join(" / "),
            CARDS.map(|(_, name)| name).join(", "),
            FILTERS.map(|(_, name)| name).join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(LaserIo);

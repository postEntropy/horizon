//! Iron's Gems 'n Jewelry: the Jewelcrafting Station cuts gems and sets them
//! into rings and necklaces; the piece's actions fire on hit, use or wear.
//!
//! The station keeps its three slots and the guide lists the eight action
//! families the jewelry can carry.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const STATION_SCREEN: &str = "screen:irons_jewelry:station";
const GUIDE_SCREEN: &str = "screen:irons_jewelry:guide";

const GEMS: [(&str, &str); 7] = [
    ("garnet", "Garnet"),
    ("moonstone", "Moonstone"),
    ("onyx", "Onyx"),
    ("peridot", "Peridot"),
    ("ruby", "Ruby"),
    ("sapphire", "Sapphire"),
    ("topaz", "Topaz"),
];

const ACTIONS: [(&str, &str); 8] = [
    ("apply_damage", "Apply damage to a target"),
    ("apply_effect", "Apply a potion effect"),
    ("apply_freeze", "Freeze a target"),
    ("create_items", "Create items"),
    ("explode", "Explode"),
    ("heal", "Heal the wearer"),
    ("ignite", "Set a target alight"),
    ("knockback", "Knock a target back"),
];

fn label(id: u32, y: i32, text: String) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text,
        value: 0,
        max_value: 0,
    }
}

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
        max_value: 64,
    }
}

fn station_screen() -> ScreenSpec {
    ScreenSpec {
        id: STATION_SCREEN.into(),
        title: "Jewelcrafting Station".into(),
        width: 176,
        height: 130,
        widgets: vec![
            slot(0, 26, 17, "Gem"),
            slot(1, 80, 17, "Pattern"),
            slot(2, 134, 17, "Jewelry"),
            label(3, 44, "Cut the gem, then set it into a band.".into()),
            label(
                4,
                58,
                "Sockets decide which actions the piece fires.".into(),
            ),
        ],
    }
}

fn guide_screen() -> ScreenSpec {
    let mut widgets = vec![
        label(0, 18, "Jewelcrafting and You".into()),
        label(
            1,
            34,
            format!("Gems: {}", GEMS.map(|(_, name)| name).join(", ")),
        ),
    ];
    for (index, (_, text)) in ACTIONS.iter().enumerate() {
        widgets.push(label(
            2 + index as u32,
            52 + index as i32 * 12,
            format!("• {text}"),
        ));
    }
    ScreenSpec {
        id: GUIDE_SCREEN.into(),
        title: "Jewelcrafting and You".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

struct IronsJewelry;

fn path_of(item: &str) -> &str {
    item.split([':', '[']).nth(1).unwrap_or(item)
}

impl Mod for IronsJewelry {
    fn init() -> Result<(Self, Registration), String> {
        Ok((IronsJewelry, Registration::new().command("jewelry")))
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
        let mut hooks = LogicHooks::new().block("irons_jewelry:jewelcrafting_station");
        for (gem, _) in GEMS {
            hooks = hooks.item(&format!("irons_jewelry:{gem}"));
        }
        hooks
            .item("irons_jewelry:ring")
            .item("irons_jewelry:necklace")
            .item("irons_jewelry:jewelcrafting_guide")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&station_screen());
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = path_of(&event.item);
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        if path == "jewelcrafting_guide" {
            let _ = ui::open_screen(&guide_screen());
            return EventOutcome::Consume;
        }
        if path == "ring" || path == "necklace" {
            let _ = ui::open_screen(&guide_screen());
            chat::show(&format!(
                "💍 {}: socketed actions fire when worn or used.",
                if path == "ring" { "Ring" } else { "Necklace" }
            ));
            return EventOutcome::Consume;
        }
        let Some((_, name)) = GEMS.iter().find(|(gem, _)| *gem == path) else {
            return EventOutcome::Pass;
        };
        chat::show(&format!(
            "💎 {name}: cut it at the Jewelcrafting Station to set it into jewelry."
        ));
        EventOutcome::Consume
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != STATION_SCREEN {
            return;
        }
        let what = match slot {
            0 => "the gem to cut",
            1 => "the pattern the piece follows",
            _ => "the finished jewelry",
        };
        chat::show(&format!("💎 The station keeps {what}."));
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "jewelry" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Gems 'n Jewelry: {} gems ({}), {} action families ({}).",
            GEMS.len(),
            GEMS.map(|(_, name)| name).join(", "),
            ACTIONS.len(),
            ACTIONS.map(|(action, _)| action).join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(IronsJewelry);

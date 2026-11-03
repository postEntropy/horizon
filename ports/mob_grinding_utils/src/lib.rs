//! Mob Grinding Utils: absorption hopper, mob fans, mob mashers, and cursed earth.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub fn absorption_hopper_screen(xp_fluid: u64) -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: "Absorption Hopper".into(),
        value: 0,
        max_value: 0,
    });

    // 16 item slots (4x4)
    for row in 0..4 {
        for col in 0..4 {
            let id = (row * 4 + col) as u32;
            widgets.push(Widget {
                id: id + 1,
                kind: WidgetKind::Slot,
                x: 20 + (col as i32) * 18,
                y: 18 + (row as i32) * 18,
                width: 18,
                height: 18,
                text: format!("Slot {}", id + 1),
                value: 0,
                max_value: 64,
            });
        }
    }

    // Fluid XP tank gauge
    widgets.push(Widget {
        id: 17,
        kind: WidgetKind::FluidGauge,
        x: 120,
        y: 18,
        width: 20,
        height: 72,
        text: "Liquid XP".into(),
        value: xp_fluid,
        max_value: 16_000,
    });

    ScreenSpec {
        id: "screen:mob_grinding_utils:absorption_hopper".into(),
        title: "Absorption Hopper".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub fn saw_screen() -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: "Mob Masher Upgrades".into(),
        value: 0,
        max_value: 0,
    });

    let upgrade_names = [
        "Sharpness",
        "Looting",
        "Fire Aspect",
        "Smite",
        "Bane of Arthropods",
        "Beheading",
    ];

    for (i, name) in upgrade_names.iter().enumerate() {
        widgets.push(Widget {
            id: (i + 1) as u32,
            kind: WidgetKind::Slot,
            x: 20 + ((i % 3) as i32) * 45,
            y: 25 + ((i / 3) as i32) * 35,
            width: 18,
            height: 18,
            text: (*name).into(),
            value: 0,
            max_value: 10,
        });
    }

    ScreenSpec {
        id: "screen:mob_grinding_utils:saw".into(),
        title: "Mob Masher".into(),
        width: 176,
        height: 130,
        widgets,
    }
}

pub fn fan_screen() -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: "Mob Fan Modifiers".into(),
        value: 0,
        max_value: 0,
    });

    let upgrade_names = ["Width Modifier", "Height Modifier", "Speed Modifier"];
    for (i, name) in upgrade_names.iter().enumerate() {
        widgets.push(Widget {
            id: (i + 1) as u32,
            kind: WidgetKind::Slot,
            x: 35 + (i as i32) * 40,
            y: 30,
            width: 18,
            height: 18,
            text: (*name).into(),
            value: 0,
            max_value: 3,
        });
    }

    ScreenSpec {
        id: "screen:mob_grinding_utils:fan".into(),
        title: "Mob Fan".into(),
        width: 176,
        height: 100,
        widgets,
    }
}

pub struct MobGrindingUtils;

impl Mod for MobGrindingUtils {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MobGrindingUtils, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![
            BlockEntityType {
                block: "mob_grinding_utils:absorption_hopper".into(),
                slots: 16,
                ticks: true,
            },
            BlockEntityType {
                block: "mob_grinding_utils:saw".into(),
                slots: 6,
                ticks: true,
            },
            BlockEntityType {
                block: "mob_grinding_utils:fan".into(),
                slots: 3,
                ticks: true,
            },
            BlockEntityType {
                block: "mob_grinding_utils:xpsolidifier".into(),
                slots: 4,
                ticks: true,
            },
            BlockEntityType {
                block: "mob_grinding_utils:tank".into(),
                slots: 0,
                ticks: false,
            },
            BlockEntityType {
                block: "mob_grinding_utils:jumbo_tank".into(),
                slots: 0,
                ticks: false,
            },
        ]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        if block.contains("absorption_hopper") {
            // Slots 0..16: hoppers can extract from bottom and sides
            for slot in 0..16 {
                let _ = item_transfer::set_slot_access(handle, slot, Face::Down, false, true);
                for face in [Face::North, Face::South, Face::East, Face::West] {
                    let _ = item_transfer::set_slot_access(handle, slot, face, false, true);
                }
            }
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.item.contains("chicken_feed")
            && (event.action == ItemAction::UseOnBlock || event.action == ItemAction::UseInAir)
        {
            chat::show("🐔 Mob Grinding Utils: Fed GM Chicken Feed to mob!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("absorption_hopper") {
                let _ = ui::open_screen(&absorption_hopper_screen(10_000));
                chat::show("🧲 Absorption Hopper: Vacuuming items and experience (XP: 10,000 mB).");
                return EventOutcome::Consume;
            } else if event.state.contains("saw") {
                let _ = ui::open_screen(&saw_screen());
                chat::show("⚙️ Mob Masher: Armed with sharpness upgrades.");
                return EventOutcome::Consume;
            } else if event.state.contains("fan") {
                let _ = ui::open_screen(&fan_screen());
                chat::show("💨 Mob Fan: Air displacement active.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(MobGrindingUtils);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screens_generation() {
        let hopper = absorption_hopper_screen(8_000);
        assert_eq!(hopper.id, "screen:mob_grinding_utils:absorption_hopper");
        assert_eq!(hopper.widgets.len(), 18); // 1 title + 16 slots + 1 fluid gauge

        let saw = saw_screen();
        assert_eq!(saw.id, "screen:mob_grinding_utils:saw");
        assert_eq!(saw.widgets.len(), 7); // 1 title + 6 upgrade slots

        let fan = fan_screen();
        assert_eq!(fan.id, "screen:mob_grinding_utils:fan");
        assert_eq!(fan.widgets.len(), 4); // 1 title + 3 modifier slots
    }
}

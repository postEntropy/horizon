//! Mining Gadgets: the mining gadget's laser, its upgrades and the modification
//! table that fits them.
mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    capabilities::EnergyView,
    item_data,
    machine,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const TABLE_SCREEN: &str = "screen:mininggadgets:table";
const BASE_BATTERY_CAPACITY: u64 = 1_000_000;
const BATTERY_CAPACITY_COMPONENT: &str = "mininggadgets:forge_energy_max_energy";

const GADGETS: [(&str, &str); 3] = [
    ("mininggadget_simple", "Simple"),
    ("mininggadget", "Mining"),
    ("mininggadget_fancy", "Fancy"),
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

fn table_screen() -> ScreenSpec {
    let mut widgets = vec![slot(0, 80, 17, "Gadget")];
    for index in 0..8u32 {
        widgets.push(slot(1 + index, 8 + index as i32 * 18, 48, "Upgrade"));
    }
    widgets.push(Widget {
        id: 9,
        kind: WidgetKind::Label,
        x: 8,
        y: 72,
        width: 160,
        height: 10,
        text: "Put the gadget here, then the upgrades.".into(),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: TABLE_SCREEN.into(),
        title: "Modification Table".into(),
        width: 176,
        height: 100,
        widgets,
    }
}

struct MiningGadgets;

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

fn battery_capacity(item: &str) -> Option<u64> {
    match item {
        "mininggadgets:upgrade_battery_1" => Some(2_000_000),
        "mininggadgets:upgrade_battery_2" => Some(5_000_000),
        "mininggadgets:upgrade_battery_3" => Some(10_000_000),
        _ => None,
    }
}

impl Mod for MiningGadgets {
    fn init() -> Result<(Self, Registration), String> {
        let energy = EnergyView {
            stored: 0,
            capacity: BASE_BATTERY_CAPACITY,
            max_receive: 10_000_000,
            max_extract: 10_000_000,
        };
        for (gadget, _) in GADGETS {
            let id = format!("mininggadgets:{gadget}");
            let _ = item_data::register_energy_item_capacity_component(
                &id,
                energy,
                BATTERY_CAPACITY_COMPONENT,
            );
        }
        Ok((MiningGadgets, Registration::new().command("mininggadgets")))
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
        let mut hooks = LogicHooks::new()
            .block("mininggadgets:modificationtable")
            .block("mininggadgets:minerslight")
            .block("mininggadgets:renderblock");
        for (gadget, _) in GADGETS {
            hooks = hooks.item(&format!("mininggadgets:{gadget}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("mininggadgets:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        match path_of(&event.state) {
            "modificationtable" => {
                let _ = ui::open_screen(&table_screen());
            }
            "minerslight" => {
                chat::show("💡 Miner's Light: the light block the gadget's light upgrade places.");
            }
            "renderblock" => {
                chat::show("✨ Render block: draws the gadget's mining beam.");
            }
            _ => return EventOutcome::Pass,
        }
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if let Some((_, name)) = GADGETS.iter().find(|(gadget, _)| *gadget == path) {
            chat::show(&format!(
                "⛏️ {name} Mining Gadget: mines in a line through the block you aim at; eight upgrade slots in the Modification Table."
            ));
            return EventOutcome::Consume;
        }
        let Some(rest) = path.strip_prefix("upgrade_") else {
            return EventOutcome::Pass;
        };
        let (kind, level) = match rest.rsplit_once('_') {
            Some((kind, level)) if level.chars().all(|c| c.is_ascii_digit()) => {
                (kind, level.parse::<u32>().unwrap_or(0))
            }
            _ => (rest, 0),
        };
        let effect = match kind {
            "battery" => "adds energy buffer to the gadget".to_string(),
            "efficiency" => format!("mines {}× faster", level.max(1)),
            "fortune" => format!("fortune {level} on the blocks it breaks"),
            "freezing" => "freezes water and lava as it mines".to_string(),
            "light_placer" => "places a Miner's Light where it mines".to_string(),
            "magnet" => "pulls the drops to you".to_string(),
            "range" => format!("extends the beam by {} blocks", level.max(1) * 4),
            "silk" => "silk touch on the blocks it breaks".to_string(),
            "size" => format!("mines a {}×{} area", 1 + level * 2, 1 + level * 2),
            "void_junk" => "voids the junk it mines".to_string(),
            "empty" => "an empty upgrade slot".to_string(),
            _ => return EventOutcome::Pass,
        };
        chat::show(&format!("⛏️ Upgrade: {effect}."));
        EventOutcome::Consume
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let Some(gadget) = machine::get_slot(handle, 0) else {
                continue;
            };
            if !GADGETS.iter().any(|(id, _)| path_of(&gadget.id) == *id)
                || gadget.count != 1
            {
                continue;
            }
            let Some(current) = item_data::get_slot_energy(handle, 0) else {
                // Invalid capacity or stored-energy data cannot be upgraded or
                // mutated by this tick.
                continue;
            };
            if current.capacity != BASE_BATTERY_CAPACITY {
                // The first installed battery tier stays authoritative. Any
                // later upgrades remain in the table's inventory slots.
                continue;
            }
            for slot in 1..9 {
                let Some(upgrade) = machine::get_slot(handle, slot) else {
                    continue;
                };
                if upgrade.count != 1 {
                    continue;
                }
                let Some(capacity) = battery_capacity(&upgrade.id) else {
                    continue;
                };
                if !item_data::set_slot_energy_capacity(handle, 0, capacity) {
                    break;
                }
                // The original installation exposes one FE on the upgraded
                // item. Keep that observed initial state; all existing energy
                // remains untouched by the capacity write.
                if item_data::receive_slot_energy(handle, 0, 1, false) == 1 {
                    let _ = machine::set_slot(handle, slot, None);
                }
                break;
            }
        }
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "mininggadgets" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "Mining Gadgets: three gadget tiers; upgrades for battery, efficiency, fortune, freezing, light, magnet, range, silk and area size."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(MiningGadgets);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_upgrades_declare_observed_capacities() {
        assert_eq!(battery_capacity("mininggadgets:upgrade_battery_1"), Some(2_000_000));
        assert_eq!(battery_capacity("mininggadgets:upgrade_battery_2"), Some(5_000_000));
        assert_eq!(battery_capacity("mininggadgets:upgrade_battery_3"), Some(10_000_000));
        assert_eq!(battery_capacity("mininggadgets:upgrade_battery_creative"), None);
    }
}

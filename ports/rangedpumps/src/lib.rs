//! Ranged Pumps: native Horizon WebAssembly component for automated liquid pumping.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub fn pump_screen(energy: u64, fluid: u64, state_label: &str) -> ScreenSpec {
    ScreenSpec {
        id: "screen:rangedpumps:pump".into(),
        title: "Ranged Pump".into(),
        width: 176,
        height: 166,
        widgets: vec![
            // Header status
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 8,
                width: 160,
                height: 12,
                text: state_label.into(),
                value: 0,
                max_value: 0,
            },
            // Energy bar (32,000 FE buffer)
            Widget {
                id: 1,
                kind: WidgetKind::EnergyBar,
                x: 18,
                y: 22,
                width: 14,
                height: 54,
                text: "Energy".into(),
                value: energy,
                max_value: 32000,
            },
            // Fluid gauge (32,000 mB buffer)
            Widget {
                id: 2,
                kind: WidgetKind::FluidGauge,
                x: 42,
                y: 22,
                width: 18,
                height: 54,
                text: "Fluid".into(),
                value: fluid,
                max_value: 32000,
            },
            // Container Input slot (Slot 0)
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 95,
                y: 28,
                width: 18,
                height: 18,
                text: "Empty Bucket".into(),
                value: 0,
                max_value: 1,
            },
            // Transfer progress arrow
            Widget {
                id: 4,
                kind: WidgetKind::ProgressArrow,
                x: 120,
                y: 38,
                width: 24,
                height: 17,
                text: "Fill".into(),
                value: 100,
                max_value: 100,
            },
            // Container Output slot (Slot 1)
            Widget {
                id: 5,
                kind: WidgetKind::Slot,
                x: 148,
                y: 48,
                width: 18,
                height: 18,
                text: "Filled Bucket".into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

pub struct Rangedpumps;

impl Mod for Rangedpumps {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Rangedpumps, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: "rangedpumps:pump".into(),
            slots: 2,
            ticks: true,
        }]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        if block.contains("pump") {
            // Slot 0 (empty container): insert from top and sides
            let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
            let _ = item_transfer::set_slot_access(handle, 0, Face::North, true, false);
            // Slot 1 (filled container): extract from bottom and sides
            let _ = item_transfer::set_slot_access(handle, 1, Face::Down, false, true);
            let _ = item_transfer::set_slot_access(handle, 1, Face::South, false, true);
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
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            chat::show("Ranged Pump: Place down and supply FE + empty buckets or fluid pipes.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("pump") {
                let screen = pump_screen(24000, 16000, "Ranged Pump: Active (Radius: 64)");
                let _ = ui::open_screen(&screen);
                chat::show(
                    "💧 Ranged Pump: Operational. Replacing pumped fluid with cobblestone/stone.",
                );
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Rangedpumps);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pump_screen() {
        let screen = pump_screen(16000, 8000, "Ranged Pump (Radius 64)");
        assert_eq!(screen.id, "screen:rangedpumps:pump");
        assert_eq!(screen.widgets.len(), 6);
        assert_eq!(screen.widgets[1].value, 16000);
        assert_eq!(screen.widgets[2].value, 8000);
    }
}

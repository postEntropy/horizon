//! Treetap: native Horizon WebAssembly component.
//! Provides tree tapping mechanics: taps extract sap/syrup from trees over time,
//! collectors accumulate fluid, and containers (buckets/bottles) are filled automatically.
pub mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct CollectorMachine {
    pub sap_amount: u32,
    pub max_sap: u32,
    pub timer: u32,
}

impl CollectorMachine {
    pub fn new() -> Self {
        Self {
            sap_amount: 0,
            max_sap: 1000,
            timer: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.timer += 1;
        if self.timer >= 100 {
            self.timer = 0;
            if self.sap_amount < self.max_sap {
                self.sap_amount = (self.sap_amount + 100).min(self.max_sap);
                return true;
            }
        }
        false
    }

    pub fn extract(&mut self, amount: u32) -> u32 {
        let extracted = amount.min(self.sap_amount);
        self.sap_amount -= extracted;
        extracted
    }
}

pub fn collector_screen(title: &str, sap: u32, max_sap: u32) -> ScreenSpec {
    ScreenSpec {
        id: "treetap:collector".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets: vec![
            // Slot 0: Input container (empty bucket/bottle)
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 44,
                y: 35,
                width: 18,
                height: 18,
                text: "Empty Container".into(),
                value: 0,
                max_value: 16,
            },
            // Slot 1: Output container (filled bucket/bottle)
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 116,
                y: 35,
                width: 18,
                height: 18,
                text: "Filled Sap".into(),
                value: 0,
                max_value: 16,
            },
            // Sap Level Gauge
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 75,
                y: 35,
                width: 24,
                height: 17,
                text: format!("{}/{} mB Sap", sap, max_sap),
                value: sap as u64,
                max_value: max_sap as u64,
            },
        ],
    }
}

pub struct Treetap {
    pub machines: HashMap<u32, CollectorMachine>,
}

impl Default for Treetap {
    fn default() -> Self {
        Self {
            machines: HashMap::new(),
        }
    }
}

impl Mod for Treetap {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Treetap::default(), Registration::new()))
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

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        let mut machine = CollectorMachine::new();
        if saved.len() >= 4 {
            machine.sap_amount = u32::from_le_bytes(saved[..4].try_into().unwrap_or_default());
        }
        self.machines.insert(handle, machine);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.machines
            .get(&handle)
            .map(|m| m.sap_amount.to_le_bytes().to_vec())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.machines.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for handle in handles {
            if let Some(m) = self.machines.get_mut(handle) {
                m.tick();
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            let (sap, max) = (250, 1000);
            let title = if event.state.contains("ceramic") {
                "Ceramic Sap Collector"
            } else if event.state.contains("tap") {
                "Tree Tap"
            } else {
                "Sap Collector"
            };
            let _ = ui::open_screen(&collector_screen(title, sap, max));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Treetap);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_machine_simulation() {
        let mut m = CollectorMachine::new();
        assert_eq!(m.sap_amount, 0);

        // Run 99 ticks -> no sap yet
        for _ in 0..99 {
            assert!(!m.tick());
        }
        assert_eq!(m.sap_amount, 0);

        // 100th tick -> collects 100 mB
        assert!(m.tick());
        assert_eq!(m.sap_amount, 100);

        // Extract 50 mB
        let extracted = m.extract(50);
        assert_eq!(extracted, 50);
        assert_eq!(m.sap_amount, 50);

        // Extract rest
        let extracted_all = m.extract(200);
        assert_eq!(extracted_all, 50);
        assert_eq!(m.sap_amount, 0);
    }

    #[test]
    fn test_collector_screen_widgets() {
        let screen = collector_screen("Sap Collector", 300, 1000);
        assert_eq!(screen.id, "treetap:collector");
        assert_eq!(screen.widgets.len(), 3);
        assert_eq!(screen.widgets[0].x, 44);
        assert_eq!(screen.widgets[1].x, 116);
        assert_eq!(screen.widgets[2].value, 300);
        assert_eq!(screen.widgets[2].max_value, 1000);
    }

    #[test]
    fn test_block_entities_registered() {
        let mut tt = Treetap::default();
        let types = tt.block_entity_types();
        assert_eq!(types.len(), 4);
        for t in &types {
            assert_eq!(t.slots, 4);
            assert!(t.ticks);
        }
    }
}

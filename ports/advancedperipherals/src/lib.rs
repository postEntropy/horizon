//! Advanced Peripherals: Chat Box, Player Detector, Environment Detector, and bridges.
pub mod items;

use std::collections::HashMap;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeripheralKind {
    ChatBox,
    PlayerDetector,
    EnvironmentDetector,
    EnergyDetector,
    BlockReader,
    GeoScanner,
    InventoryManager,
    Bridge,
    Generic,
}

impl PeripheralKind {
    pub fn from_name(name: &str) -> Self {
        if name.contains("chat_box") {
            PeripheralKind::ChatBox
        } else if name.contains("player_detector") {
            PeripheralKind::PlayerDetector
        } else if name.contains("environment_detector") {
            PeripheralKind::EnvironmentDetector
        } else if name.contains("energy_detector") {
            PeripheralKind::EnergyDetector
        } else if name.contains("block_reader") {
            PeripheralKind::BlockReader
        } else if name.contains("geo_scanner") {
            PeripheralKind::GeoScanner
        } else if name.contains("inventory_manager") {
            PeripheralKind::InventoryManager
        } else if name.contains("bridge") {
            PeripheralKind::Bridge
        } else {
            PeripheralKind::Generic
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            PeripheralKind::ChatBox => "Chat Box",
            PeripheralKind::PlayerDetector => "Player Detector",
            PeripheralKind::EnvironmentDetector => "Environment Detector",
            PeripheralKind::EnergyDetector => "Energy Detector",
            PeripheralKind::BlockReader => "Block Reader",
            PeripheralKind::GeoScanner => "Geo Scanner",
            PeripheralKind::InventoryManager => "Inventory Manager",
            PeripheralKind::Bridge => "Network Bridge",
            PeripheralKind::Generic => "Peripheral",
        }
    }

    pub fn default_range(self) -> u32 {
        match self {
            PeripheralKind::PlayerDetector => 16,
            PeripheralKind::GeoScanner => 8,
            PeripheralKind::ChatBox => 64,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeripheralState {
    pub kind: PeripheralKind,
    pub range: u32,
    pub connected: bool,
    pub detected_count: u32,
    pub tick_counter: u32,
}

impl PeripheralState {
    pub fn new(kind: PeripheralKind) -> Self {
        let range = kind.default_range();
        Self {
            kind,
            range,
            connected: true,
            detected_count: 0,
            tick_counter: 0,
        }
    }

    /// Checks whether coordinates (x, y, z) are within range of detector (dx, dy, dz).
    pub fn is_in_range(&self, detector: (i32, i32, i32), target: (i32, i32, i32)) -> bool {
        let dx = (detector.0 - target.0).abs() as u32;
        let dy = (detector.1 - target.1).abs() as u32;
        let dz = (detector.2 - target.2).abs() as u32;
        dx <= self.range && dy <= self.range && dz <= self.range
    }

    pub fn tick(&mut self) -> bool {
        self.tick_counter += 1;
        if self.tick_counter >= 20 {
            self.tick_counter = 0;
            return true;
        }
        false
    }
}

pub fn make_peripheral_screen(state: &PeripheralState) -> ScreenSpec {
    let title = state.kind.display_name();
    let mut widgets = Vec::new();

    // Slot 0: Memory card / upgrade slot
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Slot,
        x: 80,
        y: 20,
        width: 18,
        height: 18,
        text: "Memory Card".into(),
        value: 0,
        max_value: 1,
    });

    // Label 1: Status
    widgets.push(Widget {
        id: 1,
        kind: WidgetKind::Label,
        x: 20,
        y: 45,
        width: 136,
        height: 14,
        text: if state.connected {
            "Status: Online (Connected)".into()
        } else {
            "Status: Standalone".into()
        },
        value: 0,
        max_value: 0,
    });

    // Label 2: Range / Telemetry info
    widgets.push(Widget {
        id: 2,
        kind: WidgetKind::Label,
        x: 20,
        y: 62,
        width: 136,
        height: 14,
        text: match state.kind {
            PeripheralKind::PlayerDetector => format!("Detection Range: {} blocks", state.range),
            PeripheralKind::ChatBox => "Chat Range: Unlimited / Global".into(),
            PeripheralKind::GeoScanner => format!("Scan Radius: {} blocks", state.range),
            _ => "Ready for ComputerCraft commands".into(),
        },
        value: 0,
        max_value: 0,
    });

    ScreenSpec {
        id: format!("advancedperipherals:{}", title.to_ascii_lowercase().replace(' ', "_")),
        title: title.into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub struct AdvancedPeripherals {
    pub peripherals: HashMap<u32, PeripheralState>,
}

impl Default for AdvancedPeripherals {
    fn default() -> Self {
        Self {
            peripherals: HashMap::new(),
        }
    }
}

impl Mod for AdvancedPeripherals {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AdvancedPeripherals::default(), Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 2,
                ticks: true,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let kind = PeripheralKind::from_name(block);
        let mut state = PeripheralState::new(kind);
        if saved.len() >= 4 {
            state.range = u32::from_le_bytes(saved[..4].try_into().unwrap_or_default());
        }
        self.peripherals.insert(handle, state);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(state) = self.peripherals.get(&handle) {
            state.range.to_le_bytes().to_vec()
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.peripherals.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for handle in handles {
            if let Some(p) = self.peripherals.get_mut(handle) {
                p.tick();
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
            let kind = PeripheralKind::from_name(&event.state);
            let state = PeripheralState::new(kind);
            let screen = make_peripheral_screen(&state);
            let _ = ui::open_screen(&screen);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AdvancedPeripherals);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peripheral_range_check() {
        let detector = PeripheralState::new(PeripheralKind::PlayerDetector);
        let center = (100, 64, 100);

        // Within 16 blocks
        assert!(detector.is_in_range(center, (110, 64, 105)));
        assert!(detector.is_in_range(center, (100, 70, 100)));

        // Beyond 16 blocks
        assert!(!detector.is_in_range(center, (120, 64, 100)));
        assert!(!detector.is_in_range(center, (100, 64, 117)));
    }

    #[test]
    fn test_peripheral_screen_widgets() {
        let chat_box = PeripheralState::new(PeripheralKind::ChatBox);
        let screen = make_peripheral_screen(&chat_box);
        assert_eq!(screen.id, "advancedperipherals:chat_box");
        assert_eq!(screen.title, "Chat Box");
        assert_eq!(screen.widgets.len(), 3);
        assert_eq!(screen.widgets[0].text, "Memory Card");
        assert_eq!(screen.widgets[1].kind, WidgetKind::Label);
    }

    #[test]
    fn test_block_entities_registered() {
        let mut ap = AdvancedPeripherals::default();
        let types = ap.block_entity_types();
        assert_eq!(types.len(), items::BLOCK_ENTITIES.len());
        for t in &types {
            assert_eq!(t.slots, 2);
            assert!(t.ticks);
        }
    }
}

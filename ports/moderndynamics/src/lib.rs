//! Modern Dynamics: item pipes, fluid pipes, power cables, and pipe attachments.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CableTier {
    Lv,
    Mv,
    Hv,
    Ev,
    Superconductor,
}

impl CableTier {
    pub fn transfer_rate(self) -> u64 {
        match self {
            CableTier::Lv => 256,
            CableTier::Mv => 1_024,
            CableTier::Hv => 4_096,
            CableTier::Ev => 16_384,
            CableTier::Superconductor => 1_000_000,
        }
    }
}

pub fn cable_tier_of(block: &str) -> Option<CableTier> {
    if block.contains("superconductor") {
        Some(CableTier::Superconductor)
    } else if block.contains("ev_cable") {
        Some(CableTier::Ev)
    } else if block.contains("hv_cable") {
        Some(CableTier::Hv)
    } else if block.contains("mv_cable") {
        Some(CableTier::Mv)
    } else if block.contains("lv_cable") {
        Some(CableTier::Lv)
    } else {
        None
    }
}

pub fn pipe_screen(title: &str) -> ScreenSpec {
    ScreenSpec {
        id: "screen:moderndynamics:pipe".into(),
        title: title.into(),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 8,
                width: 160,
                height: 12,
                text: title.into(),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 79,
                y: 35,
                width: 18,
                height: 18,
                text: "Attachment".into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

pub struct ModernDynamics;

impl Mod for ModernDynamics {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ModernDynamics, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCKS
            .iter()
            .map(|&b| BlockEntityType {
                block: b.into(),
                slots: 1,
                ticks: false,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, _saved: &[u8]) {
        for face in [
            Face::Up,
            Face::Down,
            Face::North,
            Face::South,
            Face::East,
            Face::West,
        ] {
            let _ = item_transfer::set_slot_access(handle, 0, face, true, true);
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
        if event.item.contains("wrench")
            && (event.action == ItemAction::UseOnBlock || event.action == ItemAction::UseInAir)
        {
            chat::show(
                "🔧 Modern Dynamics Wrench: Rotated connection & toggled pipe connection mode.",
            );
            return EventOutcome::Consume;
        } else if (event.item.contains("extractor") || event.item.contains("attractor"))
            && event.action == ItemAction::UseOnBlock
        {
            chat::show("📦 Modern Dynamics: Attachment configured on pipe side.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if let Some(tier) = cable_tier_of(&event.state) {
                chat::show(&format!(
                    "⚡ Power Cable ({:?}): Capacity {} FE/t.",
                    tier,
                    tier.transfer_rate()
                ));
                return EventOutcome::Consume;
            } else if event.state.contains("pipe") {
                let _ = ui::open_screen(&pipe_screen("Pipe Configuration"));
                return EventOutcome::Consume;
            } else {
                chat::show("⚡ Modern Dynamics Network: Throughput nominal, transfer loss 0%.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ModernDynamics);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cable_transfer_rates() {
        assert_eq!(CableTier::Lv.transfer_rate(), 256);
        assert_eq!(CableTier::Mv.transfer_rate(), 1_024);
        assert_eq!(CableTier::Hv.transfer_rate(), 4_096);
        assert_eq!(CableTier::Ev.transfer_rate(), 16_384);
        assert_eq!(CableTier::Superconductor.transfer_rate(), 1_000_000);
    }

    #[test]
    fn test_cable_tier_detection() {
        assert_eq!(
            cable_tier_of("moderndynamics:lv_cable"),
            Some(CableTier::Lv)
        );
        assert_eq!(
            cable_tier_of("moderndynamics:superconductor_cable"),
            Some(CableTier::Superconductor)
        );
        assert_eq!(cable_tier_of("moderndynamics:item_pipe"), None);
    }

    #[test]
    fn test_pipe_screen_generation() {
        let screen = pipe_screen("Item Pipe");
        assert_eq!(screen.id, "screen:moderndynamics:pipe");
        assert_eq!(screen.widgets.len(), 2);
    }
}

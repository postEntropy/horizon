//! Ars Technica: mechanical automation, precise spell relays, source motors, and transmutation turrets.
pub mod items;

use horizon_mod_sdk::active_machines::ActiveMachines;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum RelayMode {
    Send = 0,
    Receive = 1,
    Direct = 2,
}

impl RelayMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => RelayMode::Send,
            1 => RelayMode::Receive,
            _ => RelayMode::Direct,
        }
    }

    pub fn next(self) -> Self {
        match self {
            RelayMode::Send => RelayMode::Receive,
            RelayMode::Receive => RelayMode::Direct,
            RelayMode::Direct => RelayMode::Send,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RelayMode::Send => "Send",
            RelayMode::Receive => "Receive",
            RelayMode::Direct => "Direct",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreciseRelayMachine {
    pub source: u32,
    pub max_source: u32,
    pub mode: RelayMode,
    pub target_coords: Option<[i32; 3]>,
    pub transfer_rate: u32,
}

impl Default for PreciseRelayMachine {
    fn default() -> Self {
        Self {
            source: 0,
            max_source: 10_000,
            mode: RelayMode::Send,
            target_coords: None,
            transfer_rate: 500,
        }
    }
}

impl PreciseRelayMachine {
    pub fn tick(&mut self) -> bool {
        if self.mode == RelayMode::Send && self.target_coords.is_some() && self.source > 0 {
            // Emits source to target link
            let transfer = self.source.min(self.transfer_rate);
            self.source -= transfer;
            return true;
        }
        false
    }

    pub fn add_source(&mut self, amount: u32) -> u32 {
        let space = self.max_source.saturating_sub(self.source);
        let accepted = amount.min(space);
        self.source += accepted;
        accepted
    }
}

#[derive(Clone, Debug)]
pub struct SourceMotorMachine {
    pub source_buffer: u32,
    pub max_source: u32,
    pub speed_rpm: u32,
    pub running: bool,
}

impl Default for SourceMotorMachine {
    fn default() -> Self {
        Self {
            source_buffer: 0,
            max_source: 5_000,
            speed_rpm: 64,
            running: false,
        }
    }
}

impl SourceMotorMachine {
    pub fn tick(&mut self) -> bool {
        if self.source_buffer >= 5 {
            self.source_buffer -= 5;
            self.running = true;
            true
        } else {
            self.running = false;
            false
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransmutationTurretMachine {
    pub source_cost: u32,
    pub cooldown: u32,
    pub charge: u32,
}

impl Default for TransmutationTurretMachine {
    fn default() -> Self {
        Self {
            source_cost: 100,
            cooldown: 0,
            charge: 0,
        }
    }
}

impl TransmutationTurretMachine {
    pub fn tick(&mut self) -> bool {
        if self.cooldown > 0 {
            self.cooldown -= 1;
            true
        } else {
            false
        }
    }

    pub fn transmutate(&mut self, source_available: u32) -> Option<&'static str> {
        if self.cooldown == 0 && source_available >= self.source_cost {
            self.cooldown = 20;
            Some("Transmuted spell energy pulse fired!")
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum Machine {
    Relay(PreciseRelayMachine),
    Motor(SourceMotorMachine),
    Turret(TransmutationTurretMachine),
}

pub fn relay_screen(relay: &PreciseRelayMachine) -> ScreenSpec {
    let target_str = match relay.target_coords {
        Some([x, y, z]) => format!("Target: ({x}, {y}, {z})"),
        None => "Target: Unlinked".into(),
    };
    ScreenSpec {
        id: "screen:ars_technica:precise_relay".into(),
        title: "Precise Spell Relay".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 18,
                width: 160,
                height: 12,
                text: format!("Mode: {}", relay.mode.name()),
                value: relay.mode as u64,
                max_value: 2,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 34,
                width: 160,
                height: 12,
                text: target_str,
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 52,
                width: 160,
                height: 14,
                text: format!("{}/{} Source", relay.source, relay.max_source),
                value: relay.source as u64,
                max_value: relay.max_source as u64,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Button,
                x: 8,
                y: 72,
                width: 160,
                height: 18,
                text: "Toggle Mode (Runic Spanner)".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

#[derive(Default)]
pub struct ArsTechnica {
    pub machines: ActiveMachines<Machine>,
    pub selected_coord: Option<[i32; 3]>,
}

impl Mod for ArsTechnica {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            ArsTechnica::default(),
            Registration::new().command("arstechnica"),
        ))
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
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let machine = if block.contains("source_motor") {
            let mut motor = SourceMotorMachine::default();
            if saved.len() >= 4 {
                motor.source_buffer =
                    u32::from_le_bytes(saved[0..4].try_into().unwrap_or_default());
            }
            Machine::Motor(motor)
        } else if block.contains("transmutation_turret") {
            let mut turret = TransmutationTurretMachine::default();
            if saved.len() >= 4 {
                turret.cooldown = u32::from_le_bytes(saved[0..4].try_into().unwrap_or_default());
            }
            Machine::Turret(turret)
        } else {
            let mut relay = PreciseRelayMachine::default();
            if saved.len() >= 5 {
                relay.source = u32::from_le_bytes(saved[0..4].try_into().unwrap_or_default());
                relay.mode = RelayMode::from_u8(saved[4]);
            }
            Machine::Relay(relay)
        };
        self.machines.insert(handle, machine);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        match self.machines.get(&handle) {
            Some(Machine::Relay(relay)) => {
                let mut data = relay.source.to_le_bytes().to_vec();
                data.push(relay.mode as u8);
                data
            }
            Some(Machine::Motor(motor)) => motor.source_buffer.to_le_bytes().to_vec(),
            Some(Machine::Turret(turret)) => turret.cooldown.to_le_bytes().to_vec(),
            None => Vec::new(),
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.machines.remove(&handle);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        self.machines
            .for_each_active(handles, |machine| match machine {
                Machine::Relay(relay) => {
                    relay.tick();
                }
                Machine::Motor(motor) => {
                    motor.tick();
                }
                Machine::Turret(turret) => {
                    turret.tick();
                }
            });
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.state.contains("precise_relay") {
            if event.action == BlockAction::UseItemOn && event.state.contains("runic_spanner") {
                chat::show("🔧 Runic Spanner: Calibrating Precise Relay mode & frequencies.");
                return EventOutcome::Consume;
            }
            if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn
            {
                let relay = match self.machines.values().find_map(|m| match m {
                    Machine::Relay(r) => Some(r),
                    _ => None,
                }) {
                    Some(r) => r.clone(),
                    None => PreciseRelayMachine::default(),
                };
                let _ = ui::open_screen(&relay_screen(&relay));
                return EventOutcome::Consume;
            }
        } else if event.state.contains("source_motor") {
            if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn
            {
                chat::show("⚙️ Source Motor: Rotational kinetic power generator ready.");
                return EventOutcome::Consume;
            }
        } else if event.state.contains("transmutation_turret")
            && (event.action == BlockAction::UseWithoutItem
                || event.action == BlockAction::UseItemOn)
        {
            chat::show("🔮 Transmutation Turret: Focus aligned for alchemical projection.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "arstechnica" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            ["status"] => {
                let relays = self
                    .machines
                    .values()
                    .filter(|m| matches!(m, Machine::Relay(_)))
                    .count();
                let motors = self
                    .machines
                    .values()
                    .filter(|m| matches!(m, Machine::Motor(_)))
                    .count();
                let turrets = self
                    .machines
                    .values()
                    .filter(|m| matches!(m, Machine::Turret(_)))
                    .count();
                Ok(Some(format!(
                    "Ars Technica Network: {relays} Relays, {motors} Motors, {turrets} Turrets active."
                )))
            }
            _ => Ok(Some("Usage: /arstechnica status".into())),
        }
    }
}

horizon_mod_sdk::export_mod!(ArsTechnica);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_received_handles_tick_and_removed_handles_stay_absent() {
        let mut port = ArsTechnica::default();
        port.create_block_entity(7, "ars_technica:source_motor", &50u32.to_le_bytes());
        port.create_block_entity(2, "ars_technica:source_motor", &80u32.to_le_bytes());
        let sleeping = port.save_block_entity(7);
        for _ in 0..3 {
            port.tick_block_entities(&[2, 999]);
        }
        assert_eq!(port.save_block_entity(7), sleeping);
        assert_eq!(port.save_block_entity(2), 65u32.to_le_bytes());
        port.tick_block_entities(&[]);
        assert_eq!(port.save_block_entity(2), 65u32.to_le_bytes());
        port.tick_block_entities(&[7]);
        assert_eq!(port.save_block_entity(7), 45u32.to_le_bytes());
        port.remove_block_entity(2);
        port.tick_block_entities(&[2]);
        assert!(!port.machines.contains_key(&2));
    }

    #[test]
    fn test_precise_relay_simulation() {
        let mut relay = PreciseRelayMachine::default();
        assert_eq!(relay.source, 0);
        let accepted = relay.add_source(2500);
        assert_eq!(accepted, 2500);
        assert_eq!(relay.source, 2500);

        // Without target, does not send
        assert!(!relay.tick());
        assert_eq!(relay.source, 2500);

        // With target in send mode, transfers rate
        relay.target_coords = Some([10, 64, 20]);
        assert!(relay.tick());
        assert_eq!(relay.source, 2000);

        // Mode rotation
        assert_eq!(relay.mode, RelayMode::Send);
        relay.mode = relay.mode.next();
        assert_eq!(relay.mode, RelayMode::Receive);
        relay.mode = relay.mode.next();
        assert_eq!(relay.mode, RelayMode::Direct);
        relay.mode = relay.mode.next();
        assert_eq!(relay.mode, RelayMode::Send);
    }

    #[test]
    fn test_source_motor_simulation() {
        let mut motor = SourceMotorMachine::default();
        assert!(!motor.running);
        assert!(!motor.tick());

        motor.source_buffer = 15;
        assert!(motor.tick());
        assert_eq!(motor.source_buffer, 10);
        assert!(motor.running);
    }

    #[test]
    fn test_transmutation_turret() {
        let mut turret = TransmutationTurretMachine::default();
        assert!(turret.transmutate(50).is_none());
        assert_eq!(
            turret.transmutate(100),
            Some("Transmuted spell energy pulse fired!")
        );
        assert_eq!(turret.cooldown, 20);

        // During cooldown, cannot fire
        assert!(turret.transmutate(100).is_none());
        assert!(turret.tick());
        assert_eq!(turret.cooldown, 19);
    }
}

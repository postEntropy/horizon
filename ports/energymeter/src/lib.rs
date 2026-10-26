//! Energy Meter: reads the energy buffer of the block it faces and shows the
//! rate and the running total, with a Monitor for the larger display.
//!
//! Every reading comes from the `capabilities` API, so the meter shows real
//! numbers whenever the measured block declares its buffer.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, capabilities, chat,
    hud::{self, Corner, Line},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const METER_SCREEN: &str = "screen:energymeter:meter";
const TICKS_PER_READ: u64 = 20;

/// The six blocks around the meter.
const NEIGHBOURS: [(i32, i32, i32); 6] = [
    (0, 1, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
    (1, 0, 0),
    (-1, 0, 0),
];

#[derive(Clone, Copy, Default)]
struct Reading {
    stored: u64,
    capacity: u64,
    total: u64,
    rate: u64,
}

struct EnergyMeter {
    /// `dimension:x,y,z` of each meter, with its last reading.
    meters: HashMap<String, Reading>,
}

fn position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

/// The buffer of the block around `x, y, z`, nearest first.
fn read_around(dimension: &str, x: i32, y: i32, z: i32) -> Option<(u64, u64)> {
    let own = position(dimension, x, y, z);
    if let Some(view) = capabilities::get_energy(&own) {
        return Some((view.stored, view.capacity));
    }
    for (dx, dy, dz) in NEIGHBOURS {
        let key = position(dimension, x + dx, y + dy, z + dz);
        if let Some(view) = capabilities::get_energy(&key) {
            return Some((view.stored, view.capacity));
        }
    }
    None
}

fn meter_screen(reading: Reading) -> ScreenSpec {
    ScreenSpec {
        id: METER_SCREEN.into(),
        title: "Energy Meter".into(),
        width: 176,
        height: 130,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "FE".into(),
                value: reading.stored,
                max_value: reading.capacity.max(1),
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 32,
                y: 18,
                width: 136,
                height: 10,
                text: format!("{}/{} FE", reading.stored, reading.capacity),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 32,
                y: 32,
                width: 136,
                height: 10,
                text: format!("Rate: {} FE/t", reading.rate),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Label,
                x: 32,
                y: 46,
                width: 136,
                height: 10,
                text: format!("Transferred: {} FE", reading.total),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Button,
                x: 8,
                y: 80,
                width: 60,
                height: 18,
                text: "Reset total".into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

impl Mod for EnergyMeter {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            EnergyMeter {
                meters: HashMap::new(),
            },
            Registration::new().on_tick().command("energymeter"),
        ))
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

    fn tick(&mut self, tick: u64) {
        if !tick.is_multiple_of(TICKS_PER_READ) || self.meters.is_empty() {
            return;
        }
        let mut panel = Vec::new();
        for (key, reading) in &mut self.meters {
            let Some((stored, capacity)) = read_at(key) else {
                reading.rate = 0;
                continue;
            };
            let delta = stored.saturating_sub(reading.stored);
            reading.rate = delta / TICKS_PER_READ;
            reading.total = reading.total.saturating_add(delta);
            reading.stored = stored;
            reading.capacity = capacity;
            panel.push(format!("{} FE/t · {} FE", reading.rate, reading.total));
        }
        if let Some(first) = panel.first() {
            hud::set_panel(
                Corner::TopRight,
                &[
                    Line {
                        text: "⚡ Energy Meter".into(),
                        color: 0xFFD166,
                    },
                    Line {
                        text: first.clone(),
                        color: 0x55FF55,
                    },
                ],
            );
        }
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let key = position(&event.dimension, event.x, event.y, event.z);
        let reading = self.meters.entry(key.clone()).or_default();
        if let Some((stored, capacity)) = read_at(&key) {
            reading.stored = stored;
            reading.capacity = capacity;
        }
        let reading = *reading;
        let _ = ui::open_screen(&meter_screen(reading));
        EventOutcome::Consume
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != METER_SCREEN || button != 4 {
            return;
        }
        for reading in self.meters.values_mut() {
            reading.total = 0;
        }
        chat::show("⚡ Energy Meter: total reset.");
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "energymeter" {
            return Err(format!("Unknown command /{name}"));
        }
        let Some((_, reading)) = self.meters.iter().next() else {
            return Ok(Some(
                "Energy Meter: right-click a meter to read the buffer of the block it faces."
                    .into(),
            ));
        };
        Ok(Some(format!(
            "⚡ Energy Meter: {} FE/t · {} FE transferred · {} meters reading.",
            reading.rate,
            reading.total,
            self.meters.len()
        )))
    }
}

/// The buffer at `key`, or around it when the meter sits next to the buffer.
fn read_at(key: &str) -> Option<(u64, u64)> {
    let (dimension, coords) = key.split_once(':')?;
    let mut numbers = coords.split(',');
    let x = numbers.next()?.parse().ok()?;
    let y = numbers.next()?.parse().ok()?;
    let z = numbers.next()?.parse().ok()?;
    read_around(dimension, x, y, z)
}

horizon_mod_sdk::export_mod!(EnergyMeter);

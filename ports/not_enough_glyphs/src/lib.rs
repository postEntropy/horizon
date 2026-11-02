//! Not Enough Glyphs: extra contingency, utility, and metamagic glyphs.
//!
//! Provides Arcane Contingency mechanics: arm a contingency glyph that
//! triggers automatically on specific damage types or critical events.
pub mod items;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind, chat, vanilla_hooks,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContingencyType {
    Damage,
    Fire,
    Fall,
    Health,
    Death,
    Blink,
    Time,
}

impl ContingencyType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Damage => "General Damage Warding",
            Self::Fire => "Fire & Lava Immunity Ward",
            Self::Fall => "Impact & Feather Fall Ward",
            Self::Health => "Critical Health Recovery Ward",
            Self::Death => "Totem of Immortality Ward",
            Self::Blink => "Emergency Teleport Ward",
            Self::Time => "Chrono-Stasis Ward",
        }
    }
}

pub fn parse_glyph_contingency(item_id: &str) -> Option<ContingencyType> {
    let path = item_id.split([':', '[']).nth(1).unwrap_or(item_id);
    match path {
        "glyph_contingency_fire" => Some(ContingencyType::Fire),
        "glyph_contingency_fall" => Some(ContingencyType::Fall),
        "glyph_contingency_health" | "glyph_contingency_heal" => Some(ContingencyType::Health),
        "glyph_contingency_death" => Some(ContingencyType::Death),
        "glyph_contingency_blink" => Some(ContingencyType::Blink),
        "glyph_contingency_time" => Some(ContingencyType::Time),
        item if item.contains("glyph_contingency") => Some(ContingencyType::Damage),
        _ => None,
    }
}

pub fn should_contingency_trigger(contingency: ContingencyType, event_source: &str) -> bool {
    match contingency {
        ContingencyType::Damage => true,
        ContingencyType::Fire => {
            event_source.contains("fire")
                || event_source.contains("lava")
                || event_source.contains("burning")
                || event_source.contains("hot_floor")
        }
        ContingencyType::Fall => event_source.contains("fall") || event_source.contains("fly_into_wall"),
        ContingencyType::Health | ContingencyType::Death => true,
        ContingencyType::Blink => true,
        ContingencyType::Time => true,
    }
}

pub struct NotEnoughGlyphs {
    pub active_contingency: Option<ContingencyType>,
}

impl NotEnoughGlyphs {
    pub fn new() -> Self {
        Self {
            active_contingency: None,
        }
    }
}

impl Default for NotEnoughGlyphs {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for NotEnoughGlyphs {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("entity-damage");
        Ok((
            NotEnoughGlyphs::new(),
            Registration::new().command("contingency"),
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            if let Some(contingency) = parse_glyph_contingency(&event.item) {
                self.active_contingency = Some(contingency);
                chat::show(&format!(
                    "📜 Arcane Contingency armed: [{}] — will trigger automatically!",
                    contingency.name()
                ));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage && event.subject.contains("player") {
            if let Some(contingency) = self.active_contingency {
                if should_contingency_trigger(contingency, &event.context) {
                    self.active_contingency = None;
                    chat::show(&format!(
                        "⚡ Contingency triggered! [{}] absorbed harm from {}!",
                        contingency.name(),
                        event.context
                    ));
                    return EventOutcome::Pass;
                }
            }
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "contingency" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["status"] => match self.active_contingency {
                Some(c) => Ok(Some(format!("Active Contingency: {}", c.name()))),
                None => Ok(Some("No active contingency armed.".into())),
            },
            ["clear"] => {
                self.active_contingency = None;
                Ok(Some("Active contingency cleared.".into()))
            }
            _ => Err("Usage: /contingency [status | clear]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(NotEnoughGlyphs);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_glyph_contingency() {
        assert_eq!(
            parse_glyph_contingency("not_enough_glyphs:glyph_contingency_fire"),
            Some(ContingencyType::Fire)
        );
        assert_eq!(
            parse_glyph_contingency("not_enough_glyphs:glyph_contingency_fall"),
            Some(ContingencyType::Fall)
        );
        assert_eq!(
            parse_glyph_contingency("not_enough_glyphs:glyph_contingency_blink"),
            Some(ContingencyType::Blink)
        );
        assert_eq!(
            parse_glyph_contingency("not_enough_glyphs:glyph_contingency_time"),
            Some(ContingencyType::Time)
        );
        assert_eq!(
            parse_glyph_contingency("not_enough_glyphs:spell_binder"),
            None
        );
    }

    #[test]
    fn test_trigger_matching() {
        assert!(should_contingency_trigger(
            ContingencyType::Fire,
            "minecraft:in_fire"
        ));
        assert!(should_contingency_trigger(
            ContingencyType::Fire,
            "minecraft:lava"
        ));
        assert!(!should_contingency_trigger(
            ContingencyType::Fire,
            "minecraft:fall"
        ));

        assert!(should_contingency_trigger(
            ContingencyType::Fall,
            "minecraft:fall"
        ));
        assert!(!should_contingency_trigger(
            ContingencyType::Fall,
            "minecraft:arrow"
        ));

        assert!(should_contingency_trigger(
            ContingencyType::Damage,
            "minecraft:mob_attack"
        ));
    }
}

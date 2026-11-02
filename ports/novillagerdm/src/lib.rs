//! No Villager Death Messages: suppresses death broadcast messages for villagers and wandering traders.
use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration, ServerEvent, ServerEventKind, config, server,
    storage,
};

pub fn is_suppressible_death(
    raw_subject: &str,
    suppress_villagers: bool,
    suppress_traders: bool,
) -> bool {
    let subject = raw_subject.strip_prefix("minecraft:").unwrap_or(raw_subject);
    if subject == "villager" {
        suppress_villagers
    } else if subject == "wandering_trader" {
        suppress_traders
    } else {
        false
    }
}

pub fn format_death_location(subject: &str, dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{subject}@{dimension}:{x},{y},{z}")
}

pub struct NoVillagerDmMod {
    pub suppressed: u64,
}

impl NoVillagerDmMod {
    pub fn new() -> Self {
        Self { suppressed: 0 }
    }

    fn read_count() -> u64 {
        storage::get("suppressed_count")
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }
}

impl Default for NoVillagerDmMod {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for NoVillagerDmMod {
    fn init() -> Result<(Self, Registration), String> {
        let suppressed = Self::read_count();
        let _ = server::set_rule("novillagerdm:suppressed_count", &suppressed.to_string());
        Ok((Self { suppressed }, Registration::new().command("novillagerdm")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().server_event(ServerEventKind::EntityDeath)
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        let suppress_villagers = config::get_bool("suppress_villager_deaths").unwrap_or(true);
        let suppress_traders = config::get_bool("suppress_wandering_traders").unwrap_or(true);

        if is_suppressible_death(&event.subject, suppress_villagers, suppress_traders) {
            self.suppressed += 1;
            let _ = storage::set("suppressed_count", self.suppressed.to_string().as_bytes());
            let last = format_death_location(&event.subject, &event.dimension, event.x, event.y, event.z);
            let _ = storage::set("last_villager_death", last.as_bytes());
            let _ = server::set_rule(
                "novillagerdm:suppressed_count",
                &self.suppressed.to_string(),
            );
        }
        EventOutcome::Pass
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["status"] => {
                let enabled = config::get_bool("suppress_villager_deaths").unwrap_or(true);
                let traders = config::get_bool("suppress_wandering_traders").unwrap_or(true);
                let last = storage::get("last_villager_death")
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_else(|| "none".into());
                Ok(Some(format!(
                    "NoVillagerDM: enabled={enabled} traders={traders} suppressed={} last={last}",
                    self.suppressed
                )))
            }
            ["toggle"] => {
                let next = !config::get_bool("suppress_villager_deaths").unwrap_or(true);
                config::set_bool("suppress_villager_deaths", next)?;
                Ok(Some(format!("NoVillagerDM: suppress_villager_deaths={next}")))
            }
            _ => Err("Usage: /novillagerdm [status | toggle]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(NoVillagerDmMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_suppressible_villager() {
        assert!(is_suppressible_death("minecraft:villager", true, true));
        assert!(is_suppressible_death("villager", true, false));
        assert!(!is_suppressible_death("villager", false, true));
    }

    #[test]
    fn test_is_suppressible_wandering_trader() {
        assert!(is_suppressible_death("minecraft:wandering_trader", true, true));
        assert!(is_suppressible_death("wandering_trader", false, true));
        assert!(!is_suppressible_death("wandering_trader", true, false));
    }

    #[test]
    fn test_other_entities_not_suppressed() {
        assert!(!is_suppressible_death("minecraft:player", true, true));
        assert!(!is_suppressible_death("minecraft:zombie", true, true));
        assert!(!is_suppressible_death("minecraft:iron_golem", true, true));
    }

    #[test]
    fn test_format_death_location() {
        let formatted = format_death_location("villager", "minecraft:overworld", 100, 64, -200);
        assert_eq!(formatted, "villager@minecraft:overworld:100,64,-200");
    }
}

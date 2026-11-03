//! InControl: Custom mob spawning rules, light levels, and dimension filters.
pub mod items;

use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration, VanillaHookEvent, VanillaHookKind, vanilla_hooks,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnResult {
    Allow,
    Deny,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRule {
    pub mob: Option<String>,
    pub dimension: Option<String>,
    pub min_y: Option<i32>,
    pub max_y: Option<i32>,
    pub min_light: Option<u8>,
    pub max_light: Option<u8>,
    pub result: SpawnResult,
}

impl SpawnRule {
    pub fn matches(&self, mob: &str, dim: &str, y: i32, light: u8) -> bool {
        if let Some(ref m) = self.mob {
            if m != mob && !mob.contains(m) {
                return false;
            }
        }
        if let Some(ref d) = self.dimension {
            if d != dim && !dim.contains(d) {
                return false;
            }
        }
        if let Some(min_y) = self.min_y {
            if y < min_y {
                return false;
            }
        }
        if let Some(max_y) = self.max_y {
            if y > max_y {
                return false;
            }
        }
        if let Some(min_l) = self.min_light {
            if light < min_l {
                return false;
            }
        }
        if let Some(max_l) = self.max_light {
            if light > max_l {
                return false;
            }
        }
        true
    }
}

pub fn evaluate_rules(
    rules: &[SpawnRule],
    mob: &str,
    dim: &str,
    y: i32,
    light: u8,
) -> SpawnResult {
    for rule in rules {
        if rule.matches(mob, dim, y, light) {
            return rule.result;
        }
    }
    SpawnResult::Default
}

pub struct InControl {
    pub rules: Vec<SpawnRule>,
}

impl Default for InControl {
    fn default() -> Self {
        // Default rule: standard safe mob spawn defaults
        Self {
            rules: vec![
                SpawnRule {
                    mob: Some("creeper".into()),
                    dimension: None,
                    min_y: None,
                    max_y: None,
                    min_light: Some(8),
                    max_light: None,
                    result: SpawnResult::Deny,
                },
            ],
        }
    }
}

impl Mod for InControl {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("mob-spawn");
        Ok((
            InControl::default(),
            Registration::new().command("incontrol"),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage {
            // Can be extended for damage modifiers
            return EventOutcome::Pass;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "incontrol" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(Some(format!(
                "§6InControl§r: {} active rules loaded. Use `/incontrol reload` or `/incontrol test <mob> <y> <light>`.",
                self.rules.len()
            )));
        }

        match parts[0] {
            "reload" => {
                // Reload rules
                self.rules = InControl::default().rules;
                Ok(Some(format!(
                    "§aInControl rules reloaded! ({} rules active)",
                    self.rules.len()
                )))
            }
            "test" => {
                let mob = parts.get(1).copied().unwrap_or("minecraft:zombie");
                let y: i32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
                let light: u8 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
                let res = evaluate_rules(&self.rules, mob, "minecraft:overworld", y, light);
                Ok(Some(format!(
                    "Test spawn for {} at y={} light={}: {:?}",
                    mob, y, light, res
                )))
            }
            _ => Err("Usage: /incontrol [reload|test <mob> <y> <light>]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(InControl);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_rule_matching() {
        let rules = vec![
            SpawnRule {
                mob: Some("creeper".into()),
                dimension: Some("overworld".into()),
                min_y: Some(0),
                max_y: Some(70),
                min_light: Some(8),
                max_light: None,
                result: SpawnResult::Deny,
            },
            SpawnRule {
                mob: Some("zombie".into()),
                dimension: None,
                min_y: None,
                max_y: None,
                min_light: None,
                max_light: None,
                result: SpawnResult::Allow,
            },
        ];

        // Creeper in high light (light >= 8) -> Deny
        assert_eq!(
            evaluate_rules(&rules, "minecraft:creeper", "minecraft:overworld", 64, 10),
            SpawnResult::Deny
        );

        // Creeper in dark (light 0) -> not matching rule 1, doesn't match rule 2 -> Default
        assert_eq!(
            evaluate_rules(&rules, "minecraft:creeper", "minecraft:overworld", 64, 0),
            SpawnResult::Default
        );

        // Zombie -> Allow
        assert_eq!(
            evaluate_rules(&rules, "minecraft:zombie", "minecraft:nether", 30, 0),
            SpawnResult::Allow
        );
    }

    #[test]
    fn test_height_boundaries() {
        let rules = vec![SpawnRule {
            mob: None,
            dimension: None,
            min_y: Some(10),
            max_y: Some(50),
            min_light: None,
            max_light: None,
            result: SpawnResult::Deny,
        }];

        assert_eq!(
            evaluate_rules(&rules, "skeleton", "overworld", 30, 0),
            SpawnResult::Deny
        );
        assert_eq!(
            evaluate_rules(&rules, "skeleton", "overworld", 5, 0),
            SpawnResult::Default
        );
        assert_eq!(
            evaluate_rules(&rules, "skeleton", "overworld", 60, 0),
            SpawnResult::Default
        );
    }
}

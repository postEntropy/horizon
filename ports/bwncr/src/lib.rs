use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration, ServerEvent, ServerEventKind, chat, config,
    server, storage,
};

pub struct BwncrMod {
    pub silenced_events: u64,
}

impl BwncrMod {
    fn read_u64(key: &str) -> u64 {
        storage::get(key)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }

    fn write_u64(key: &str, value: u64) {
        let _ = storage::set(key, value.to_string().as_bytes());
    }

    fn muted_list() -> Vec<String> {
        let from_cfg = config::get_string("silence_us").unwrap_or_default();
        let from_store = storage::get("muted_sounds")
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default();
        let mut out: Vec<String> = from_cfg
            .split(',')
            .chain(from_store.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn should_silence_name(
        subject: &str,
        silence_wither: bool,
        silence_dragon: bool,
        silence_lightning: bool,
        silence_trader: bool,
    ) -> Option<&'static str> {
        let s = subject.strip_prefix("minecraft:").unwrap_or(subject);
        match s {
            "wither" if silence_wither => Some("wither"),
            "ender_dragon" if silence_dragon => Some("dragon"),
            "lightning_bolt" if silence_lightning => Some("lightning"),
            "wandering_trader" if silence_trader => Some("trader"),
            _ => None,
        }
    }

    fn should_silence_subject(subject: &str) -> Option<&'static str> {
        Self::should_silence_name(
            subject,
            config::get_bool("silence_wither").unwrap_or(true),
            config::get_bool("silence_dragon").unwrap_or(true),
            config::get_bool("silence_lightning").unwrap_or(false),
            config::get_bool("silence_trader").unwrap_or(false),
        )
    }

    fn sync_rules(&self) {
        let _ = server::set_rule(
            "bwncr:silence_wither",
            if config::get_bool("silence_wither").unwrap_or(true) {
                "true"
            } else {
                "false"
            },
        );
        let _ = server::set_rule(
            "bwncr:silence_dragon",
            if config::get_bool("silence_dragon").unwrap_or(true) {
                "true"
            } else {
                "false"
            },
        );
        let _ = server::set_rule(
            "bwncr:silenced_count",
            &self.silenced_events.to_string(),
        );
    }
}

impl Mod for BwncrMod {
    fn init() -> Result<(Self, Registration), String> {
        let silenced_events = Self::read_u64("silenced_count");
        let m = Self { silenced_events };
        m.sync_rules();
        Ok((m, Registration::new().command("bwncr")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
            .server_event(ServerEventKind::EntitySpawn)
            .server_event(ServerEventKind::EntityDeath)
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if let Some(tag) = Self::should_silence_subject(&event.subject) {
            self.silenced_events += 1;
            Self::write_u64("silenced_count", self.silenced_events);
            let _ = server::set_rule("bwncr:last_silenced", tag);
            let _ = server::set_rule("bwncr:silenced_count", &self.silenced_events.to_string());
            if config::get_bool("debug_mode").unwrap_or(false) {
                chat::show(&format!(
                    "[BWNCR] Silenced global {} sound at {},{},{} ({})",
                    tag, event.x, event.y, event.z, event.dimension
                ));
            }
        }
        EventOutcome::Pass
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["status"] => {
                let wither = config::get_bool("silence_wither").unwrap_or(true);
                let dragon = config::get_bool("silence_dragon").unwrap_or(true);
                let lightning = config::get_bool("silence_lightning").unwrap_or(false);
                let trader = config::get_bool("silence_trader").unwrap_or(false);
                let debug = config::get_bool("debug_mode").unwrap_or(false);
                let muted = Self::muted_list();
                Ok(Some(format!(
                    "BWNCR: wither={wither} dragon={dragon} lightning={lightning} trader={trader} debug={debug} custom={} silenced={}",
                    muted.len(),
                    self.silenced_events
                )))
            }
            ["listen"] => {
                let next = !config::get_bool("debug_mode").unwrap_or(false);
                config::set_bool("debug_mode", next)?;
                Ok(Some(format!("BWNCR debug sound listener: {next}")))
            }
            ["silence", target, val] => {
                let b = val
                    .parse::<bool>()
                    .map_err(|_| "expected true or false".to_string())?;
                let key = match *target {
                    "wither" => "silence_wither",
                    "dragon" => "silence_dragon",
                    "lightning" => "silence_lightning",
                    "trader" => "silence_trader",
                    _ => return Err(format!("unknown target {target}")),
                };
                config::set_bool(key, b)?;
                self.sync_rules();
                Ok(Some(format!("BWNCR {key}={b}")))
            }
            ["mute", sound] => {
                let mut list = Self::muted_list();
                if !list.iter().any(|s| s == sound) {
                    list.push((*sound).to_owned());
                }
                let joined = list.join(",");
                storage::set("muted_sounds", joined.as_bytes())?;
                Ok(Some(format!("BWNCR muted {sound} (total={})", list.len())))
            }
            ["unmute", sound] => {
                let mut list = Self::muted_list();
                list.retain(|s| s != sound);
                let joined = list.join(",");
                storage::set("muted_sounds", joined.as_bytes())?;
                Ok(Some(format!("BWNCR unmuted {sound} (total={})", list.len())))
            }
            ["list"] => {
                let list = Self::muted_list();
                Ok(Some(format!("BWNCR muted sounds: [{}]", list.join(", "))))
            }
            _ => Err(
                "Usage: /bwncr [status | listen | silence <wither|dragon|lightning|trader> <true|false> | mute <sound> | unmute <sound> | list]".into(),
            ),
        }
    }
}

horizon_mod_sdk::export_mod!(BwncrMod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_subject_check() {
        assert_eq!(
            BwncrMod::should_silence_name("minecraft:wither", true, true, false, false),
            Some("wither")
        );
        assert_eq!(
            BwncrMod::should_silence_name("minecraft:ender_dragon", true, true, false, false),
            Some("dragon")
        );
        assert_eq!(
            BwncrMod::should_silence_name("minecraft:cow", true, true, false, false),
            None
        );
    }
}

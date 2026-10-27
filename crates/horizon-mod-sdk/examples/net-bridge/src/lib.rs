//! Exercises the API 0.4 packet bridge in an integrated single-player world.
use horizon_mod_sdk::{LogicHooks, Mod, Registration, chat, net, warn};

const CHANNEL: &str = "net_bridge:echo";

struct NetBridge {
    sent: bool,
}

impl Mod for NetBridge {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self { sent: false }, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        if !self.sent {
            self.sent = true;
            if let Err(error) = net::send_to_server(CHANNEL, b"ping") {
                warn(&format!("Could not send test packet: {error}"));
            }
        }
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().channel(CHANNEL)
    }

    fn on_packet(&mut self, channel: &str, payload: &[u8], from_player: Option<&str>) {
        if channel != CHANNEL {
            return;
        }
        match payload {
            b"ping" => {
                let target = from_player.unwrap_or("*");
                if let Err(error) = net::send_to_client(target, CHANNEL, b"pong") {
                    warn(&format!("Could not reply to test packet: {error}"));
                }
            }
            b"pong" => chat::show("Mod packet round trip succeeded"),
            _ => {}
        }
    }
}

horizon_mod_sdk::export_mod!(NetBridge);

use tracing::{warn, debug};
use actix::{Actor, Context, StreamHandler};
use actix_web_actors::ws;

use crate::config::Settings;

// ------------------ InputWebsocket

/// Define HTTP actor
struct InputWebsocket {
    pub relays: Option<Vec<String>>, // external relays
}

impl Actor for InputWebsocket {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for InputWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        debug!("[handle] {:?}", msg);
        debug!("[i] {:?}", self.relays);

        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // TODO send to list of relays
                ctx.text(text)
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => {
                warn!("[handle] unexpected msg (type): {:?}", msg);
            },
        }
    }
}

// ------------------ Proxy Server

pub struct Proxy {
    settings: Settings,
}

impl Actor for Proxy {
    type Context = Context<Self>;
}

impl Proxy {
    pub fn new(settings: &Settings) -> Self {
        Proxy {
            settings: settings.clone(),
        }
    }
}

// TODO allow only specified address to connect
// impl Handler<Connect> for Proxy


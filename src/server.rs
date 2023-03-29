use tracing::{warn, debug, info};

use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};

use actix_web_actors::ws;

use crate::config::Settings;

// ------------------ InputWebsocket

/// Define HTTP actor
struct InputWebsocket;

impl Actor for InputWebsocket {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for InputWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        debug!("[handle] {:?}", msg);

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

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(InputWebsocket {}, &req, stream);

    info!("[index] {:?}", resp);
    resp
}

// ------------------ Server

pub struct Server {
    pub settings: Settings,
}

impl Server {
    pub fn new(settings: &Settings) -> Self {
        Server {
            settings: settings.clone(),
        }
    }

    pub async fn run(self: &Self) -> std::io::Result<()> {

        let addr = format!(
            "{}:{}",
            self.settings.network.address.trim(),
            self.settings.network.port
        );

        println!("relays?: {:?}", self.settings.sources);

        let socket_addr: String = match addr.parse() {
            Err(_) => panic!("listening address not valid"),
            Ok(k) => k,
        };

        info!("listening on: {:?}", socket_addr);
        HttpServer::new(|| App::new().route("/", web::get().to(index)))
            .bind(socket_addr)?
            .run()
            .await
    }
}

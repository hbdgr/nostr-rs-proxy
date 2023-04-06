use tracing::{warn, debug, info};

use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};

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

async fn index(
    req: HttpRequest,
    stream: web::Payload,
    relays: web::Data<Option<Vec<String>>>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(InputWebsocket {
        relays: relays.get_ref().to_owned()
    }, &req, stream);

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

        let relays = self.settings.sources.relays.clone();

        info!("listening on: {:?}", socket_addr);
        HttpServer::new(move ||
            App::new()
                .app_data(web::Data::new(relays.clone()))
                .route("/", web::get().to(index))
        )
        .workers(2)
        .bind(socket_addr)?
        .run()
        .await
    }
}

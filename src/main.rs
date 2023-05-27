mod config;
mod proxy;

use proxy::Proxy;

mod ws;
mod lobby;
mod messages;
mod start_connection;

use tracing::info;

// ---------------------------------------------------------
use actix_web::{web, App, HttpServer};
use actix::Actor;

use lobby::Lobby;
use start_connection::start_connection as start_connection_route;
// ---------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let todolater: Option<String> = None;
    let settings = config::Settings::new(&todolater);
    let proxy = Proxy::new(&settings).start();

    // standard logging
    tracing_subscriber::fmt::try_init().unwrap();

    let addr = format!(
        "{}:{}",
        settings.network.address.trim(),
        settings.network.port
    );

    // println!("relays?: {:?}", settings.sources);

    let socket_addr: String = match addr.parse() {
        Err(_) => panic!("listening address not valid"),
        Ok(k) => k,
    };

    info!("listening on: {:?}", socket_addr);
    HttpServer::new(move ||
        App::new()
            .service(start_connection_route)
            .app_data(web::Data::new(proxy.clone()))
    )
    .workers(2)
    .bind(socket_addr)?
    .run()
    .await
}

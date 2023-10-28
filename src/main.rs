mod config;
mod proxy;
mod relay_client;

mod proxy_connection;
mod messages;

use proxy::Proxy;
use tracing::{info,debug};

// ---------------------------------------------------------
use actix_web::{web, App, HttpServer};
use actix::Actor;

use proxy_connection::start;
// ---------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // standard logging
    tracing_subscriber::fmt::try_init().unwrap();

    let todolater: Option<String> = None;
    let settings = config::Settings::new(&todolater);
    let proxy = Proxy::new(&settings)
        .await
        .start();

    let addr = format!(
        "{}:{}",
        settings.network.address.trim(),
        settings.network.port
    );

    debug!("main: relays: {:?}", settings.sources);

    let socket_addr: String = match addr.parse() {
        Err(_) => panic!("listening address not valid"),
        Ok(k) => k,
    };

    info!("listening on: {:?}", socket_addr);
    HttpServer::new(move ||
        App::new()
            .service(start)
            .app_data(web::Data::new(proxy.clone()))
    )
    .workers(10)
    .bind(socket_addr)?
    .run()
    .await
}

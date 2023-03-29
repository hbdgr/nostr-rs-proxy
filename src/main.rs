use nostr_rs_proxy::config;
use nostr_rs_proxy::server;

// ---------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let todolater: Option<String> = None;
    let settings = config::Settings::new(&todolater);
    let server = server::Server::new(&settings);

    // standard logging
    tracing_subscriber::fmt::try_init().unwrap();

    server.run()
        .await
}

use crate::messages::ClientMessage;
use crate::proxy::Proxy;

// use actix_web::web::Bytes;
use awc::BoxedSocket;
// use futures_util::{SinkExt as _, StreamExt as _};
use tracing::{info,debug,warn};


use actix_codec::Framed;
use actix_http::ws::Codec;
use actix::prelude::{Actor, Context, Handler};

// use tokio::select;

pub struct RelayClient {
    relay_addr: String,
    connection: Option<Box<Framed<BoxedSocket, Codec>>>
}

impl RelayClient {
    pub fn new(relay_addr: &String) -> RelayClient {
        RelayClient {
            relay_addr: relay_addr.to_owned(),
            connection: None,
        }
    }

    pub async fn connect(&mut self) {
        debug!("RelayClient: connect()");

        let req = awc::Client::new()
            .ws(self.relay_addr.clone());

        // handle connection error
        // let (res, mut ws) = match result {
        let (res, ws) = match req.connect().await {
            Ok(tuple) => tuple,
            Err(error) => {
                // TODO reconnect
                warn!("Error connectiong to relay: {error:?}");
                warn!("Ignoring relay: {:?}", self.relay_addr);
                return
            }
        };

        debug!("RelayClient: new(), response: {res:?}");

        // save connection
        self.connection = Some(Box::new(ws));

        // loop {
        //     select! {
        //         Some(msg) = ws.next() => {
        //             match msg {
        //                 Ok(ws::Frame::Text(txt)) => {
        //                     // log echoed messages from server
        //                     info!("Server: {txt:?}")
        //                 }
        //
        //                 Ok(ws::Frame::Ping(_)) => {
        //                     // respond to ping probes
        //                     ws.send(ws::Message::Pong(Bytes::new())).await.unwrap();
        //                 }
        //
        //                 _ => {}
        //             }
        //         }
        //         else => break
        //     }
        // }
    }

    pub fn send_to_relay(&self, msg: ClientMessage, ctx: &mut Context<Proxy>) {
        debug!("RelayClient: send, msg: {:?}, ctx: {:?}", msg, ctx);
    }
}

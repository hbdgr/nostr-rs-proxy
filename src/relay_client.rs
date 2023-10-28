use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::messages::ClientMessage;

use awc::{ws, BoxedSocket};

use tracing::{info,debug,warn};

use futures_util::{
    SinkExt as _,
    StreamExt as _
};

use tokio::select;

use actix_web::web::Bytes;
use actix_codec::Framed;
use actix_http::ws::Codec;

use uuid::Uuid;


#[derive(Debug, Clone)]
pub struct RelayClient {
    pub uuid: Uuid,
    pub relay_addr: String,
    req_msg_queue: Arc<Mutex<Vec<ClientMessage>>>, // messages waiting to be send to relay
}

impl RelayClient {
    pub fn new(relay_addr: String) -> RelayClient {
        let relay = RelayClient {
            uuid: Uuid::new_v4(),
            relay_addr: relay_addr.clone(),
            req_msg_queue: Arc::new(Mutex::new(Vec::new())),
        };

        let mut relay_cloned = relay.clone();

        // run relay event loop
        actix_web::rt::spawn(async move {
            relay_cloned.run().await
        });

        relay
    }

    pub async fn connect(&mut self) -> Option<Framed<BoxedSocket, Codec>> {
        debug!("RelayClient {:?}: connect()", self.uuid);

        let req = awc::Client::new()
            .ws(self.relay_addr.clone());

        // handle connection error
        let (res, connection) = match req.connect().await {
            Ok(tuple) => tuple,
            Err(error) => {
                warn!("Error connectiong to relay: {error:?}");
                warn!("Ignoring relay: {:?}", self.relay_addr);

                return None
            }
        };
        debug!("RelayClient {:?}: new(), response: {res:?}", self.uuid);

        Some(connection)
    }

    pub async fn process_msg_queue(
        connection: Arc<Mutex<Framed<BoxedSocket, Codec>>>,
        req_msg_queue: Arc<Mutex<Vec<ClientMessage>>>
    ) {
        if let Ok(mut ws_unlocked) = connection.try_lock() {

            let mut queue_unlocked = req_msg_queue.lock().unwrap();

            if queue_unlocked.len() > 0 {
                for msg in queue_unlocked.iter() {
                    debug!("RelayClient:, Send msg: {:?}", msg);

                    // debug!("sending to connection");
                    let _ = ws_unlocked
                        .send(ws::Message::Text(msg.msg.clone().into()))
                        .await
                        .unwrap();
                }
                queue_unlocked.clear();
            }
        }
    }

    pub async fn connection_next(
        connection: Arc<Mutex<Framed<BoxedSocket, Codec>>>,
    ) {
        if let Ok(mut ws_unlocked) = connection.try_lock() {
            let msg = ws_unlocked.next().await.unwrap();

            debug!("RelayClient: run() - msg next() - GOT MSG: {:?}", msg);
            match msg {
                Ok(ws::Frame::Text(txt)) => {
                    // log messages from server
                    info!("RelayClient - next: {txt:?}");
                }
                Ok(ws::Frame::Ping(_)) => {
                    // respond to ping probes
                    info!("RelayClient - got Ping - sending Pong");
                    ws_unlocked.send(ws::Message::Pong(Bytes::new())).await.unwrap();
                }
                _ => {
                    info!("RelayClient - _");
                }
            }
        }
    }

    // Proccess requests / responses
    pub async fn run(&mut self) {
        debug!("RelayClient: run()");

        let mut self_cloned = self.clone();

        actix_web::rt::spawn(async move {
            let con = match self_cloned.connect().await {
                Some(c) => Arc::new(Mutex::new(c)),
                None => {
                    return // Err("bad connection");
                },
            };

            loop {
                select! {
                    _ = RelayClient::connection_next(
                        con.clone()
                    ) => {
                        actix_web::rt::task::yield_now().await;
                    }

                    _ = RelayClient::process_msg_queue(
                        con.clone(),
                        self_cloned.req_msg_queue.clone(),
                    ) => {
                        actix_web::rt::task::yield_now().await;
                    }
                }
            }
        });
    }

    pub fn schedule_request(&self, msg: ClientMessage) {
        debug!("RelayClient: request: {:?}", msg);

        let mut msg_queue = self.req_msg_queue
            .lock()
            .unwrap();

        debug!("RelayClient: unlocked msg_queue: : {:?}", msg_queue);
        msg_queue.push(
            msg.clone(),
        );
    }
}

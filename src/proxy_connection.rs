use crate::proxy::Proxy;
use crate::messages::{ClientMessage, Connect, Disconnect, WsMessage};
use actix::{fut, ActorContext, ActorFutureExt, ContextFutureSpawner, WrapFuture};
use actix::{Actor, Addr, Running, StreamHandler};
use actix::{AsyncContext, Handler};
use actix_web::{get, web::Data, web::Payload, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use std::time::{Duration, Instant};
use uuid::Uuid;


use tracing::debug;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_INTERVAL: Duration = Duration::from_secs(10);


#[get("/")]
pub async fn start(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Proxy>>,
) -> Result<HttpResponse, Error> {
    debug!("ProxyConnection: srv: {:?}", srv.get_ref());

    let ws = ProxyConnection::new(
        srv.get_ref().clone()
    );

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}

pub struct ProxyConnection {
    id: Uuid,
    hb: Instant,
    proxy_addr: Addr<Proxy>,
}

impl ProxyConnection {
    pub fn new(proxy: Addr<Proxy>) -> ProxyConnection {
        ProxyConnection {
            id: Uuid::new_v4(),
            hb: Instant::now(),
            proxy_addr: proxy,
        }
    }
}

impl Actor for ProxyConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.proxy_addr
            .send(Connect {
                addr: addr.recipient(),
                self_id: self.id,
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.proxy_addr.do_send(Disconnect {
            id: self.id,
        });
        Running::Stop
    }
}

impl ProxyConnection {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_INTERVAL {
                println!("disconnectiong due to failed heartbeat");
                act.proxy_addr.do_send(Disconnect {
                    id: act.id,
                });
                ctx.stop();
                return;
            }

            ctx.ping(b"PING");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ProxyConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => {},
            Ok(Text(s)) => self.proxy_addr.do_send(ClientMessage {
                id: self.id,
                msg: s.to_string(),
            }),
            Err(e) => panic!("{:?}", e),
        }
    }
}

impl Handler<WsMessage> for ProxyConnection {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("ProxyConnection: handle [WsMessage]: {:?}", msg);
        ctx.text(msg.0);
        Ok(true)
    }
}

use actix::prelude::{Message, Recipient};
use uuid::Uuid;

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub self_id: Uuid,
}

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: Uuid,
    pub msg: String,
}

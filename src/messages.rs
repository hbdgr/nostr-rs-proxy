use actix::prelude::{Message, Recipient};
use uuid::Uuid;

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
pub struct WsMessage(pub String);

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub self_id: Uuid,
}

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
pub struct Disconnect {
    pub id: Uuid,
}

#[derive(Debug)]
#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
pub struct ClientMessage {
    pub id: Uuid,
    pub msg: String,
}

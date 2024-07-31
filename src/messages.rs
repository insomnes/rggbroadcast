use serde::{Deserialize, Serialize};

use std::collections::HashMap;

macro_rules! define_message {
    ($name:ident, { $($field_name:ident: $field_type:ty),* $(,)? }) => {
        #[derive(Debug, Serialize, Deserialize, Clone)]
        pub struct $name {
            pub msg_id: usize,
            $(pub $field_name: $field_type),*
        }
    };
    ($name:ident, OK, { $($field_name:ident: $field_type:ty),* $(,)? }) => {
        #[derive(Debug, Serialize, Deserialize, Clone)]
        pub struct $name {
            pub msg_id: usize,
            pub in_reply_to: usize,
            $(pub $field_name: $field_type),*
        }
    };
}

define_message!(InitMessage, {
    node_id: String,
    node_ids: Vec<String>
});

define_message!(InitOkMessage, OK, {});

define_message!(EchoMessage, {
    echo: String
});

define_message!(EchoOkMessage, OK, {
    echo: String
});

define_message!(GenerateMessage, {});

define_message!(GenerateOkMessage, OK, {
    id: String
});

define_message!(BroadcastMessage, {
    message: usize
});

define_message!(BroadcastOkMessage, OK, {});

define_message!(TopologyMessage, {
    topology: HashMap<String, Vec<String>>
});

define_message!(TopologyOkMessage, OK, {});

define_message!(ReadMessage, {});

define_message!(ReadOkMessage, OK, {
    messages: Vec<usize>
});

define_message!(HeartbeatMessage, {
    slice: Vec<usize>,
    sum: usize,
    count: usize
});

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Message {
    Init(InitMessage),
    Echo(EchoMessage),
    Generate(GenerateMessage),
    Broadcast(BroadcastMessage),
    Topology(TopologyMessage),
    Read(ReadMessage),
    Heartbeat(HeartbeatMessage),
    #[serde(rename = "read_ok")]
    ReadOK(ReadOkMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum ReplyMessage {
    #[serde(rename = "init_ok")]
    InitOK(InitOkMessage),
    #[serde(rename = "echo_ok")]
    EchoOK(EchoOkMessage),
    #[serde(rename = "generate_ok")]
    GenerateOK(GenerateOkMessage),
    #[serde(rename = "broadcast_ok")]
    BroadcastOK(BroadcastOkMessage),
    #[serde(rename = "topology_ok")]
    TopologyOK(TopologyOkMessage),
    #[serde(rename = "read_ok")]
    ReadOK(ReadOkMessage),
    Read(ReadMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Packet {
    pub src: String,
    pub dest: String,
    pub body: Message,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReplyPacket {
    pub src: String,
    pub dest: String,
    pub body: ReplyMessage,
}

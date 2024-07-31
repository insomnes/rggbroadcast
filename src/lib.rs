use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use anyhow::anyhow;
use anyhow::Result as AnyResult;

mod messages;
mod numbers;
mod topology;

use messages::*;
use numbers::MasterOfNumbers;
use topology::Topology;

pub struct Transport {
    in_rx: async_channel::Receiver<String>,
    out_tx: async_channel::Sender<String>,
}

impl Transport {
    async fn read_packet(&self) -> AnyResult<Packet> {
        let line = self.in_rx.recv().await?;
        //eprintln!("LOG->Transport received line: {:?}", line);
        let packet: Packet = serde_json::from_str(&line).unwrap();
        Ok(packet)
    }

    async fn send_string(&self, string: String) -> AnyResult<()> {
        self.out_tx.send(string).await?;

        Ok(())
    }
}

pub struct Node {
    id: String,
    topology: Topology,
    msg_id_calc: IDCalculator,
    num_master: MasterOfNumbers,
    transport: Transport,
}

impl Node {
    fn build(
        init_message: messages::InitMessage,
        transport: Transport,
        grid_line_size: usize,
    ) -> Self {
        Node {
            id: init_message.node_id.clone(),
            topology: Topology::new(init_message.node_id, init_message.node_ids, grid_line_size),
            msg_id_calc: IDCalculator {
                message_id: AtomicUsize::new(0),
            },
            num_master: MasterOfNumbers::new(),
            transport,
        }
    }

    async fn read_packet(&self) -> AnyResult<Packet> {
        let packet = self.transport.read_packet().await?;
        //eprintln!("LOG->Node received packet: {:?}", packet);
        Ok(packet)
    }

    async fn send_string(&self, string: String) -> AnyResult<()> {
        //eprintln!("LOG->Node sending string: {:?}", string);
        self.transport.send_string(string).await
    }

    async fn run_main_loop(&self) -> AnyResult<()> {
        loop {
            let packet = self.read_packet().await?;
            self.handle_packet(packet).await?;
        }
    }
    async fn handle_packet(&self, packet: Packet) -> AnyResult<()> {
        let src = packet.src;
        let body = packet.body;
        match body {
            Message::Echo(echo_message) => self.handle_echo(echo_message, src).await,
            Message::Generate(generate_message) => {
                self.handle_generate(generate_message, src).await
            }
            Message::Topology(topology_message) => {
                self.handle_topology(topology_message, src).await
            }
            Message::Broadcast(broadcast_message) => {
                self.handle_broadcast(broadcast_message, src).await
            }
            Message::Read(read_message) => self.handle_read(read_message, src).await,
            Message::ReadOK(read_ok_message) => self.handle_read_ok(read_ok_message, src).await,
            Message::Heartbeat(heartbeat_message) => {
                self.handle_heartbeat(heartbeat_message, src).await
            }
            // Ok for local
            _ => panic!("Unknown message type"),
        }
    }

    fn make_reply_packet(&self, dest: String, body: ReplyMessage) -> ReplyPacket {
        ReplyPacket {
            src: self.topology.node_id.clone(),
            dest,
            body,
        }
    }

    async fn reply(&self, reply_packet: ReplyPacket) -> AnyResult<()> {
        let reply_json = serde_json::to_string(&reply_packet).unwrap();
        self.send_string(reply_json).await
    }

    async fn handle_echo(&self, echo_message: EchoMessage, src: String) -> AnyResult<()> {
        let reply_body = EchoOkMessage {
            msg_id: self.msg_id_calc.get_id(),
            in_reply_to: echo_message.msg_id,
            echo: echo_message.echo,
        };
        let reply_packet = self.make_reply_packet(src, ReplyMessage::EchoOK(reply_body));
        self.reply(reply_packet).await
    }

    async fn handle_generate(
        &self,
        generate_message: GenerateMessage,
        src: String,
    ) -> AnyResult<()> {
        let reply_body = GenerateOkMessage {
            msg_id: self.msg_id_calc.get_id(),
            in_reply_to: generate_message.msg_id,
            id: uuid::Uuid::new_v4().to_string(),
        };
        let reply_packet = self.make_reply_packet(src, ReplyMessage::GenerateOK(reply_body));
        self.reply(reply_packet).await
    }

    async fn handle_topology(
        &self,
        topology_message: TopologyMessage,
        src: String,
    ) -> AnyResult<()> {
        eprintln!("Topology message: {:?}", topology_message);
        let reply_body = TopologyOkMessage {
            msg_id: self.msg_id_calc.get_id(),
            in_reply_to: topology_message.msg_id,
        };
        let reply_packet = self.make_reply_packet(src, ReplyMessage::TopologyOK(reply_body));
        self.reply(reply_packet).await?;
        Ok(())
    }

    async fn handle_broadcast(
        &self,
        broadcast_message: BroadcastMessage,
        src: String,
    ) -> AnyResult<()> {
        let reply_body = BroadcastOkMessage {
            msg_id: self.msg_id_calc.get_id(),
            in_reply_to: broadcast_message.msg_id,
        };
        let reply_packet =
            self.make_reply_packet(src.clone(), ReplyMessage::BroadcastOK(reply_body));
        self.reply(reply_packet).await?;
        if !self.num_master.add_number(broadcast_message.message) {
            panic!("Number already known");
        }

        let targets = self.topology.get_random_targets_from_all(3, &[src]);
        self.send_heartbeat_to(targets).await
    }

    async fn handle_read(&self, read_message: ReadMessage, src: String) -> AnyResult<()> {
        let numbers = self.num_master.read_numbers(None);
        let reply_body = ReadOkMessage {
            msg_id: self.msg_id_calc.get_id(),
            in_reply_to: read_message.msg_id,
            messages: numbers,
        };
        let reply_packet = self.make_reply_packet(src.clone(), ReplyMessage::ReadOK(reply_body));
        self.reply(reply_packet).await
    }

    async fn handle_read_ok(&self, read_ok_message: ReadOkMessage, _src: String) -> AnyResult<()> {
        let numbers = read_ok_message.messages;
        self.num_master.extend_numbers(&numbers);
        Ok(())
    }

    async fn handle_heartbeat(
        &self,
        heartbeat_message: HeartbeatMessage,
        src: String,
    ) -> AnyResult<()> {
        let (our_sum, our_count) = self.num_master.get_sum_count();
        if our_sum == heartbeat_message.sum && our_count == heartbeat_message.count {
            return Ok(());
        }

        let (our_sum, our_count) = self.num_master.extend_numbers(&heartbeat_message.slice);
        if our_sum == heartbeat_message.sum && our_count == heartbeat_message.count {
            return Ok(());
        }

        let read_packet = Packet {
            src: self.id.clone(),
            dest: src.clone(),
            body: Message::Read(ReadMessage {
                msg_id: self.msg_id_calc.get_id(),
            }),
        };
        let read_packet_json = serde_json::to_string(&read_packet)?;
        self.send_string(read_packet_json).await
    }

    async fn send_heartbeat_to(&self, targets: Vec<String>) -> AnyResult<()> {
        let heartbeat_message = {
            let slice = self.num_master.read_numbers(Some(30));
            let (sum, count) = self.num_master.get_sum_count();
            if count == 0 {
                return Ok(());
            }
            HeartbeatMessage {
                msg_id: self.msg_id_calc.get_id(),
                slice,
                sum,
                count,
            }
        };
        for t in targets {
            let packet = Packet {
                src: self.id.clone(),
                dest: t.clone(),
                body: Message::Heartbeat(heartbeat_message.clone()),
            };
            let packet_json = serde_json::to_string(&packet)?;
            self.send_string(packet_json).await?;
        }
        Ok(())
    }
}

struct IDCalculator {
    message_id: AtomicUsize,
}

impl IDCalculator {
    fn get_id(&self) -> usize {
        self.message_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

async fn get_init_message() -> AnyResult<InitMessage> {
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let line = lines.next_line().await?.unwrap();
    let init_packet: Packet = serde_json::from_str(&line)?;

    let message = match init_packet.body {
        Message::Init(init_message) => init_message,
        _ => return Err(anyhow!("First message must be an Init message")),
    };
    let init_ok = InitOkMessage {
        in_reply_to: 1,
        msg_id: 1,
    };
    let init_ok_packet = ReplyPacket {
        src: init_packet.dest,
        dest: init_packet.src,
        body: ReplyMessage::InitOK(init_ok),
    };

    let init_ok_json = serde_json::to_string(&init_ok_packet)?;

    let mut stdout = tokio::io::stdout();

    stdout
        .write_all(format!("{}\n", init_ok_json).as_bytes())
        .await?;
    stdout.flush().await?;
    Ok(message)
}

pub async fn run_node(node_coros: u8) -> AnyResult<()> {
    eprintln!("Starting node");
    let init_message = get_init_message().await?;

    let (in_tx, in_rx) = async_channel::unbounded();
    let (out_tx, out_rx) = async_channel::unbounded();
    let transport = Transport {
        in_rx: in_rx.clone(),
        out_tx: out_tx.clone(),
    };
    let node = Arc::new(Node::build(init_message, transport, 5));
    eprintln!("Node {} initialized", node.id);

    let mut tasks = Vec::new();
    let read_input: tokio::task::JoinHandle<AnyResult<()>> = tokio::task::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = tokio::io::BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            in_tx.send(line).await?;
        }
        Ok(())
    });
    eprintln!("Read input task initialized");
    tasks.push(read_input);

    eprintln!("Initializing coros");
    for coro_id in 0..node_coros {
        eprintln!("Initializing coro {}", coro_id);
        let node = Arc::clone(&node);
        let task: tokio::task::JoinHandle<AnyResult<()>> = tokio::task::spawn(async move {
            eprintln!("Starting coro {}", coro_id);
            node.run_main_loop().await?;
            Ok(())
        });

        tasks.push(task);
    }
    eprintln!("All coros initialized");

    eprintln!("Initializing stdout task");
    let stdout_handle: tokio::task::JoinHandle<AnyResult<()>> = tokio::task::spawn(async move {
        eprintln!("Starting stdout task");
        let mut stdout = tokio::io::stdout();
        loop {
            let reply = out_rx.recv().await?;
            stdout.write_all(format!("{}\n", reply).as_bytes()).await?;
            stdout.flush().await?;
        }
    });
    tasks.push(stdout_handle);
    eprintln!("Stdout task initialized");

    let hb_node = Arc::clone(&node);

    eprintln!("Initializing heartbeat task");
    let heart_beat_handle: tokio::task::JoinHandle<AnyResult<()>> = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        eprintln!("Starting heartbeat task");
        // Higher for less messages
        // Lower for faster convergence
        // 150-200 is fine for faster one
        let mut tick = tokio::time::interval(Duration::from_millis(300));
        loop {
            tick.tick().await;
            // Lower for less messages
            // Higher for faster convergence
            // 3-4 is fine for faster one
            let targets = hb_node.topology.get_random_targets_from_neighbors(2, &[]);
            hb_node.send_heartbeat_to(targets).await?;
        }
    });
    tasks.push(heart_beat_handle);
    eprintln!("Heartbeat task initialized");

    for task in tasks {
        task.await??;
    }

    Ok(())
}

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use uuid::Uuid;

use rand::{rngs::ThreadRng, thread_rng, Rng};

#[derive(Clone, PartialEq, Eq, Debug)]
enum MessageType {
    Data,
    Ack,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Message {
    id: Uuid,
    from: usize,
    to: usize,
    message_type: MessageType,
}
impl Message {
    fn data(from: usize, to: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            message_type: MessageType::Data,
        }
    }

    fn ack(id: Uuid, from: usize, to: usize) -> Self {
        Self {
            id,
            from,
            to,
            message_type: MessageType::Ack,
        }
    }
}

#[derive(Clone, Debug)]
struct Process {
    id: usize,
    messages_to_send: Arc<RwLock<Vec<Message>>>,
    delivered_messages: Arc<RwLock<Vec<Message>>>,
}

const FAULTY_RATIO: f32 = 0.25;

impl Process {
    fn new(id: usize) -> Self {
        Self {
            id,
            messages_to_send: Arc::new(RwLock::new(Vec::new())),
            delivered_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn receive(&self, network: &Network, random: &mut ThreadRng) {
        let message = if let Ok(mut messages) = network.messages.write() {
            let message_position = messages.iter().position(|m| m.to == self.id);
            if message_position.is_some() && random.gen::<f32>() < FAULTY_RATIO {
                println!("Receive: faulty network");
                return;
            }
            message_position.map(|i| messages.remove(i))
        } else {
            None
        };
        if let Some(message) = message {
            match message.message_type {
                MessageType::Data => {
                    self.send_ack(network, message.id, message.from, random);
                    if let Ok(mut delivered_messages) = self.delivered_messages.write() {
                        if !delivered_messages.contains(&message) {
                            println!(
                                "Delivered message at: {}, from: {}",
                                message.to, message.from
                            );
                            delivered_messages.push(message);
                        }
                    }
                }
                MessageType::Ack => {
                    println!("Received ack at: {}, from: {}", message.to, message.from);
                    if let Ok(mut messages_to_send) = self.messages_to_send.write() {
                        let sent = messages_to_send
                            .iter()
                            .position(|m| {
                                m.id == message.id && m.from == message.to && m.to == message.from
                            })
                            .map(|i| messages_to_send.remove(i));
                        if sent.is_some() {
                            println!(
                                "-> Message send ended from: {}, to: {}",
                                message.to, message.from
                            );
                        }
                    }
                }
            }
        }
    }

    fn send(&self, network: &Network, receiver_id: usize) {
        if let Ok(mut messages) = network.messages.write() {
            let message = Message::data(self.id, receiver_id);
            if let Ok(mut messages) = self.messages_to_send.write() {
                messages.push(message.clone());
            }
            messages.push(message.clone());
            println!("Sent message from: {}, to: {}", self.id, receiver_id);
        }
    }

    fn send_ack(&self, network: &Network, id: Uuid, receiver_id: usize, random: &mut ThreadRng) {
        if random.gen::<f32>() < FAULTY_RATIO {
            println!("Sent ack: faulty network");
            return;
        }
        if let Ok(mut messages) = network.messages.write() {
            println!("Sent ack from: {}, to: {}", self.id, receiver_id);
            messages.push(Message::ack(id, self.id, receiver_id));
        }
    }

    fn send_until_ack(&self, network: &Network, random: &mut ThreadRng) -> bool {
        if let Ok(messages_to_send) = self.messages_to_send.read() {
            if !messages_to_send.is_empty() && random.gen::<f32>() < FAULTY_RATIO {
                println!("Send until ack: faulty network");
                return !messages_to_send.is_empty();
            }
            for message in messages_to_send.iter() {
                if let Ok(mut messages) = network.messages.write() {
                    println!("Sent message from: {}, to: {}", self.id, message.to);
                    messages.push(message.clone());
                }
            }
            !messages_to_send.is_empty()
        } else {
            false
        }
    }
}

struct Network {
    messages: Arc<RwLock<Vec<Message>>>,
}

impl Network {
    fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

fn main() {
    let network = Network::new();
    let processes = (0..5).map(Process::new).collect::<Vec<_>>();
    let finished = AtomicBool::new(false);

    std::thread::scope(|s| {
        for p in processes.iter() {
            s.spawn(|| {
                let mut random = thread_rng();
                loop {
                    p.receive(&network, &mut random);
                    std::thread::sleep(Duration::from_millis(250));
                    if finished.load(Ordering::Acquire) {
                        break;
                    }
                }
            });
        }

        s.spawn(|| {
            let mut random = thread_rng();
            for _ in 0..5 {
                let sender_index = random.gen_range(0..processes.len());
                let sender = &processes[sender_index];
                let mut receiver_index;
                loop {
                    receiver_index = random.gen_range(0..processes.len());
                    if sender_index != receiver_index {
                        break;
                    }
                }

                let receiver_id = processes[receiver_index].id;
                sender.send(&network, receiver_id);
            }
            std::thread::sleep(Duration::from_secs(5));
            loop {
                let mut send = false;
                for p in processes.iter() {
                    send = send || p.send_until_ack(&network, &mut random);
                }
                if !send {
                    finished.store(true, Ordering::Release);
                    break;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
            println!("All messages and acks sent!");
        });
    });
}

use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct BroadcastMessage {
    from: usize,
    to: usize,
}

impl BroadcastMessage {
    fn from(from: usize, to: usize) -> Self {
        Self { from, to }
    }
}

#[derive(Debug)]
struct Process {
    id: usize,
    delivered: Vec<BroadcastMessage>,
}

impl Process {
    fn new(id: usize) -> Self {
        Self {
            id,
            delivered: Vec::new(),
        }
    }

    fn receive(&mut self, network: Arc<Network>) {
        let message = if let Ok(mut messages) = network.messages.write() {
            messages
                .iter()
                .position(|m| m.to == self.id)
                .map(|index| messages.remove(index))
        } else {
            None
        };
        if let Some(message) = message {
            if !self.delivered.contains(&message) {
                println!("<- Forwarding from: {}, for: {}", self.id, message.from);
                network.send(message.from, Some(self.id));
                println!("^ Delivered from: {}, to: {}", message.from, self.id);
                self.delivered.push(message);
            }
        }
    }
}

struct Network {
    processes: Vec<usize>,
    messages: Arc<RwLock<Vec<BroadcastMessage>>>,
}

impl Network {
    fn new(processes: &[Process]) -> Self {
        Self {
            processes: processes.iter().map(|p| p.id).collect(),
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn send(&self, process_id: usize, forwarded_by: Option<usize>) {
        if let Ok(mut messages) = self.messages.write() {
            let mut one_sent = false;
            for (i, to) in self.processes.iter().enumerate() {
                if *to != process_id {
                    if let Some(forwarded_by) = forwarded_by {
                        if *to == forwarded_by {
                            println!("- Skipping sending for: {}", forwarded_by);
                        }
                    } else if one_sent {
                        println!("@ Crash from: {}, to: {} ({})", process_id, to, i);
                        break;
                    }
                    println!("-> Sending from: {}, to: {}", process_id, to);
                    messages.push(BroadcastMessage::from(process_id, *to));
                    one_sent = true;
                }
            }
        }
    }
}

pub fn main() {
    let mut processes = (0..5).map(Process::new).collect::<Vec<_>>();
    let ps = processes.iter().map(|p| p.id).collect::<Vec<_>>();
    let network = Arc::new(Network::new(&processes));
    std::thread::scope(|s| {
        for p in processes.iter_mut() {
            s.spawn(|| loop {
                p.receive(network.clone());
            });
        }

        s.spawn(|| {
            let mut random = rand::thread_rng();
            //loop {
            let process_id = ps[random.gen_range(0..ps.len())];
            network.send(process_id, None);
            //std::thread::sleep(Duration::new(10, 0));
            //}
        });
    });
}

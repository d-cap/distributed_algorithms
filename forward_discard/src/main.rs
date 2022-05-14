use std::collections::BTreeMap;
use std::thread::{self, JoinHandle};

use crossbeam::channel::{self, Receiver, Sender};

struct Node<T> {
    id: usize,
    announced: bool,
    map: BTreeMap<usize, u32>,
    tx_left: Sender<T>,
    rx_left: Receiver<T>,
    tx_right: Sender<T>,
    rx_right: Receiver<T>,
}

struct Ring {
    nodes: Vec<JoinHandle<()>>,
}

impl Ring {
    fn wait(self) {
        self.nodes.into_iter().fold((), |_, v| {
            v.join().unwrap();
        });
    }
}

impl From<Vec<Node<(usize, u32)>>> for Ring {
    fn from(nodes: Vec<Node<(usize, u32)>>) -> Self {
        Self {
            nodes: nodes
                .into_iter()
                .map(|mut n| {
                    thread::spawn(move || loop {
                        if !n.announced {
                            n.announced = true;
                            while let Err(_) = n.tx_left.send((n.id, 1)) {}
                            while let Err(_) = n.tx_right.send((n.id, 1)) {}
                        }

                        if let Ok((id, v)) = n.rx_left.try_recv() {
                            println!(
                                "Try: {}, {} -> {} ({}, {})",
                                id,
                                v,
                                n.id,
                                id != n.id,
                                n.map.get(&id).is_none()
                            );
                            if id != n.id && n.map.get(&id).is_none() {
                                n.map.insert(id, v);
                                println!("Insert: {}, {} -> {}", id, v, n.id);
                                println!("{}, {:?}", n.id, n.map);
                                while let Err(_) = n.tx_right.send((id, v + 1)) {}
                            } else {
                                println!("Discard: {} -> {}, {}", n.id, id, v);
                            }
                        }
                        if let Ok((id, v)) = n.rx_right.try_recv() {
                            println!(
                                "Try: {}, {} -> {} ({}, {})",
                                id,
                                v,
                                n.id,
                                id != n.id,
                                n.map.get(&id).is_none()
                            );
                            if id != n.id && n.map.get(&id).is_none() {
                                n.map.insert(id, v);
                                println!("Insert: {}, {} -> {}", id, v, n.id);
                                println!("{}, {:?}", n.id, n.map);
                                while let Err(_) = n.tx_left.send((id, v + 1)) {}
                            } else {
                                println!("Discard: {} -> {}, {}", n.id, id, v);
                            }
                        }
                    })
                })
                .collect::<Vec<_>>(),
        }
    }
}

fn main() {
    let size = 10;
    let mut nodes: Vec<Node<(usize, u32)>> = Vec::with_capacity(size);

    let (junction_tx_left, junction_rx_right) = crossbeam::channel::unbounded();
    let (junction_tx_right, junction_rx_left) = crossbeam::channel::unbounded();
    let mut previous_right_tx = junction_tx_right;
    let mut previous_right_rx = junction_rx_right;

    for i in 0..size {
        let (tx_left, rx_right) = channel::unbounded();
        let (tx_right, rx_left) = channel::unbounded();
        nodes.push(Node {
            id: i,
            announced: false,
            map: BTreeMap::new(),
            tx_left,
            rx_left,
            tx_right: previous_right_tx,
            rx_right: previous_right_rx,
        });

        if i + 1 == size - 1 {
            previous_right_tx = junction_tx_left.clone();
            previous_right_rx = junction_rx_left.clone();
        } else {
            previous_right_tx = tx_right;
            previous_right_rx = rx_right;
        }
    }

    let ring: Ring = nodes.into();
    ring.wait();
}

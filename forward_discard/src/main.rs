use std::collections::BTreeMap;
use std::sync::atomic::AtomicUsize;
use std::thread::{self, JoinHandle};

static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

use crossbeam::channel::{self, Receiver, Sender};

struct Node<T> {
    id: usize,
    announced: bool,
    finished: bool,
    nodes: usize,
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
                    thread::spawn(move || {
                        loop {
                            if !n.announced {
                                n.announced = true;
                                while n.tx_left.send((n.id, 1)).is_err() {}
                                while n.tx_right.send((n.id, 1)).is_err() {}
                            } else if n.nodes == n.map.len() && !n.finished {
                                THREAD_COUNT.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                                n.finished = true;
                            }

                            if n.finished {
                                let threads =
                                    THREAD_COUNT.load(std::sync::atomic::Ordering::Acquire);
                                if threads == n.nodes + 1 {
                                    println!("Map: {}, {:?}", n.id, n.map);
                                    break;
                                }
                            }

                            if let Ok((id, v)) = n.rx_left.try_recv() {
                                if id != n.id && n.map.get(&id).is_none() {
                                    n.map.insert(id, v);
                                    let mut count = 0;
                                    while n.tx_right.send((id, v + 1)).is_err() {
                                        count += 1;
                                        if count > 100 {
                                            // break;
                                        }
                                    }
                                }
                            }
                            if let Ok((id, v)) = n.rx_right.try_recv() {
                                if id != n.id && n.map.get(&id).is_none() {
                                    n.map.insert(id, v);
                                    let mut count = 0;
                                    while n.tx_left.send((id, v + 1)).is_err() {
                                        count += 1;
                                        if count > 100 {
                                            // break;
                                        }
                                    }
                                }
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

    let (junction_first_node_tx, junction_last_node_rx) = crossbeam::channel::unbounded();
    let (junction_last_node_tx, junction_first_node_rx) = crossbeam::channel::unbounded();
    let mut previous_right_tx = junction_first_node_tx.clone();
    let mut previous_right_rx = junction_first_node_rx.clone();

    for i in 0..size {
        let (tx_left, rx_right) = if i == size - 1 {
            (
                junction_last_node_tx.clone(),
                junction_first_node_rx.clone(),
            )
        } else {
            channel::unbounded()
        };
        let (tx_right, rx_left) = if i == size - 1 {
            (
                junction_first_node_tx.clone(),
                junction_last_node_rx.clone(),
            )
        } else {
            channel::unbounded()
        };
        nodes.push(Node {
            id: i,
            announced: false,
            finished: false,
            nodes: size - 1,
            map: BTreeMap::new(),
            tx_left,
            rx_left,
            tx_right: previous_right_tx,
            rx_right: previous_right_rx,
        });

        previous_right_tx = tx_right;
        previous_right_rx = rx_right;
    }

    let ring: Ring = nodes.into();
    ring.wait();
}

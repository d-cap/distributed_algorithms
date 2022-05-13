use std::sync::mpsc::{self, Receiver, Sender};

struct Ring<T> {
    nodes: Vec<Node<T>>,
}

struct Node<T> {
    tx_left: Sender<T>,
    rx_left: Receiver<T>,
    tx_right: Sender<T>,
    rx_right: Receiver<T>,
}

fn main() {
    let size = 10;
    let mut ring: Ring<&str> = Ring {
        nodes: Vec::with_capacity(size),
    };

    let (junction_tx_left, junction_rx_right) = mpsc::channel();
    let (junction_tx_right, junction_rx_left) = mpsc::channel();
    let mut previous_right_tx = junction_tx_right;
    let mut previous_right_rx = junction_rx_right;

    for i in 0..size {
        let (tx_left, rx_right) = mpsc::channel();
        let (tx_right, rx_left) = mpsc::channel();
        ring.nodes.push(Node {
            tx_left,
            rx_left,
            tx_right: previous_right_tx,
            rx_right: previous_right_rx,
        });

        if i + 1 == size - 1 {
            previous_right_tx = junction_tx_left;
            previous_right_rx = junction_rx_left;
        } else {
            previous_right_tx = tx_right;
            previous_right_rx = rx_right;
        }
    }
}

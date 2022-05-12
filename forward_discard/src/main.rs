use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for i in 1..10 {
            tx.send(i).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    });

    while let Some(val) = rx.recv().ok() {
        println!("hi number {} from the main thread!", val);
    }
}

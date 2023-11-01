use std::{sync::RwLock, time::Duration};

use actix_web::{get, web, App, Error, HttpResponse, HttpServer};
use lazy_static::lazy_static;
use rand::prelude::*;

use merkle_tree::MerkleTree;
use reqwest::Client;

lazy_static! {
    static ref LOG_SERVER: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref CLIENT: Client = Client::new();
    static ref TREE: RwLock<MerkleTree<u8, u16>> = RwLock::new(MerkleTree::new());
}

#[get("/hash/{node_index}")]
async fn get_hash_service(node_index: web::Path<usize>) -> Result<HttpResponse, Error> {
    if let Ok(tree) = TREE.read() {
        let node_index = node_index.into_inner();
        Ok(HttpResponse::Ok().body(tree.hashes[node_index].hash.to_string()))
    } else {
        Ok(HttpResponse::BadRequest().finish())
    }
}

#[get("/value/{node_index}")]
async fn get_value_service(node_index: web::Path<usize>) -> Result<HttpResponse, Error> {
    if let Ok(tree) = TREE.read() {
        let node_index = node_index.into_inner();
        Ok(HttpResponse::Ok().body(tree.data[node_index].value.to_string()))
    } else {
        Ok(HttpResponse::BadRequest().finish())
    }
}

const NODE_TO_INSERT: usize = 16;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server...");
    let log_server = std::env::var("LOG_SERVER").expect("Log server must be populated");
    if let Ok(mut v) = LOG_SERVER.write() {
        *v = log_server;
    }
    let answer_node = std::env::var("ANSWER_NODE");
    let mut rng = rand::thread_rng();
    let missing_index = if answer_node.is_ok() {
        let missing_index = rng.gen_range(0..NODE_TO_INSERT);
        println!("Mismatch node should be: {}", missing_index);
        missing_index
    } else {
        0
    };
    if let Ok(mut tree) = TREE.write() {
        for i in 0..NODE_TO_INSERT {
            if answer_node.is_ok() && i == missing_index {
                continue;
            }
            tree.insert(i as u8, i as u16);
        }
    };
    if let Ok(answer_node) = answer_node {
        actix_web::rt::spawn(async move {
            std::thread::sleep(Duration::from_secs(5));
            if let Ok(tree) = TREE.read() {
                log(&format!("Proposer node: {:?}", tree.data)).await;
                let mut queue = vec![tree.root];
                let mut mismatching_value: Option<usize> = None;
                while let Some(node_index) = queue.pop() {
                    let node = &tree.hashes[node_index];
                    if let Ok(node_hash) = get_hash(&answer_node, node_index).await {
                        if node.hash != node_hash {
                            match node.left {
                                merkle_tree::Child::Node(n) => queue.push(n),
                                merkle_tree::Child::Value(v) => mismatching_value = Some(v),
                                merkle_tree::Child::None => {}
                            }
                            match node.right {
                                merkle_tree::Child::Node(n) => queue.push(n),
                                merkle_tree::Child::Value(v) => mismatching_value = Some(v),
                                merkle_tree::Child::None => {}
                            }
                        }
                    }
                }
                if let Some(mismatching_value) = mismatching_value {
                    let mismatching_value = get_value(&answer_node, mismatching_value).await;
                    match mismatching_value {
                        Ok(v) => log(&format!("Mismatching value: {}", v)).await,
                        Err(_) => log("Error retrieving mismatching value").await,
                    }
                } else {
                    log("No mismatching value").await;
                }
            }
        })
        .await
        .unwrap();
    } else {
        log(&format!("Answering node: {:?}", TREE.read().unwrap().data)).await;
    };

    HttpServer::new(|| {
        App::new()
            .service(get_hash_service)
            .service(get_value_service)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

async fn get_hash(answer_node: &str, node: usize) -> Result<u64, ()> {
    match CLIENT
        .get(format!("{}/hash/{}", answer_node, node))
        .send()
        .await
    {
        Ok(response) => {
            if response.status() != reqwest::StatusCode::OK {
                println!("Error sending log message: {}", response.status());
                Err(())
            } else if let Ok(bytes) = response.bytes().await {
                let value = bytes.escape_ascii().to_string().parse::<u64>().unwrap();
                Ok(value)
            } else {
                Err(())
            }
        }
        Err(e) => {
            println!("{}", e);
            Err(())
        }
    }
}

async fn get_value(answer_node: &str, node: usize) -> Result<u16, ()> {
    match CLIENT
        .get(format!("{}/value/{}", answer_node, node))
        .send()
        .await
    {
        Ok(response) => {
            if response.status() != reqwest::StatusCode::OK {
                println!("Error sending log message: {}", response.status());
                Err(())
            } else if let Ok(bytes) = response.bytes().await {
                let value = bytes.escape_ascii().to_string().parse::<u16>().unwrap();
                Ok(value)
            } else {
                Err(())
            }
        }
        Err(e) => {
            println!("{}", e);
            Err(())
        }
    }
}

async fn log(message: &str) {
    dbg!(println!("message: {}", message));
    let log_server = if let Ok(log_server) = LOG_SERVER.read() {
        log_server.clone()
    } else {
        "".to_owned()
    };
    let body = format!(
        "{}, for node: {}",
        message,
        gethostname::gethostname().to_str().unwrap(),
    );
    match CLIENT.post(log_server).body(body).send().await {
        Ok(response) => {
            if response.status() != reqwest::StatusCode::OK {
                println!("Error sending log message: {}", response.status());
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

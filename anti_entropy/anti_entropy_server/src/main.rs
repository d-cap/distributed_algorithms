use std::sync::RwLock;

use actix_web::{post, App, Error, HttpResponse, HttpServer};
use lazy_static::lazy_static;

use merkle_tree::MerkleTree;
use reqwest::Client;

lazy_static! {
    static ref LOG_SERVER: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref ANSWER_NODE: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref CLIENT: Client = Client::new();
}

#[post("/answer")]
async fn answer() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut tree = MerkleTree::new();
    tree.insert(0, "test");
    let log_server = std::env::var("LOG_SERVER").expect("Log server must be populated");
    if let Ok(mut v) = LOG_SERVER.write() {
        *v = log_server;
    }
    let answer_node = std::env::var("ANSWER_NODE");
    if let Ok(answer_node) = answer_node {
        actix_web::rt::spawn(async move {
            loop {
                log(&format!("Answer node: {}", answer_node)).await;
            }
        })
        .await
        .unwrap();
    }

    HttpServer::new(|| App::new().service(answer))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn should() {}
}

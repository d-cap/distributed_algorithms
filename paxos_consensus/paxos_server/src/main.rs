use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use actix_web::{
    get, post,
    web::{self, Buf},
    App, Error, HttpResponse, HttpServer,
};
use futures::future::join_all;
use futures_util::StreamExt as _;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref NODE_ID: AtomicU64 = AtomicU64::new(0);
    static ref PROPOSAL_COUNT: AtomicU64 = AtomicU64::new(0);
    static ref LOG_SERVER: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref NODE_ROLE: RwLock<Role> = RwLock::new(Role::Learner);
    static ref PROPOSING_VALUE: RwLock<String> = RwLock::new("".to_owned());
    static ref CURRENT_VALUE: RwLock<Option<String>> = RwLock::new(None);
    static ref PAXOS_NODES: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref EMPTY_STRING: String = "".to_owned();
    static ref CLIENT: Client = reqwest::Client::new();
}

pub type Propose = u64;

#[derive(Deserialize, Serialize)]
pub struct Accept {
    proposal_number: u32,
    value: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Proposer,
    Acceptor,
    Learner,
}

impl TryFrom<String> for Role {
    type Error = std::io::Error;

    fn try_from(role: String) -> Result<Self, Self::Error> {
        match role.as_str() {
            "proposer" => Ok(Self::Proposer),
            "acceptor" => Ok(Self::Acceptor),
            "learner" => Ok(Self::Learner),
            _ => Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("Role not valid: {}", role),
            )),
        }
    }
}

#[post("/consensus")]
async fn consensus_start(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let value = bytes.escape_ascii().to_string();
    log("Consensus started", &value).await;
    let proposal_id = get_next_id();
    let proposes = PAXOS_NODES.read().map(|paxos_nodes| {
        paxos_nodes
            .iter()
            .map(|n| {
                CLIENT
                    .post(format!("{}/propose", n))
                    .body(proposal_id.to_string())
                    .send()
            })
            .collect::<Vec<_>>()
    });
    let promises = proposes.map(|proposes| async { join_all(proposes).await });
    Ok(HttpResponse::Ok().body(format!("Value {value} accepted!")))
}

#[post("/propose")]
async fn propose(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let role = *NODE_ROLE.read().unwrap();
    if role == Role::Learner {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let proposal_number: Propose = bytes.get_u64();
    log("Propose started", &proposal_number.to_string()).await;
    Ok(HttpResponse::Ok().finish())
}

#[post("/accept")]
async fn accept(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let role = *NODE_ROLE.read().unwrap();
    if role == Role::Learner {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let value = serde_json::from_slice::<Accept>(&bytes);
    Ok(HttpResponse::Ok().finish())
}

#[post("/accepted")]
async fn accepted(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let role = *NODE_ROLE.read().unwrap();
    if role != Role::Proposer {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let value = serde_json::from_slice::<Accept>(&bytes);
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let node_id_value = std::env::var("NODE_ID").expect("Node id not set");
    NODE_ID.store(
        node_id_value
            .parse::<u64>()
            .expect("Node id should be a number"),
        Ordering::Release,
    );
    let log_server_value = std::env::var("LOG_SERVER").expect("Log server not set");
    println!("Log server used: {}", log_server_value);
    if let Ok(mut log_server) = LOG_SERVER.write() {
        *log_server = log_server_value;
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "internal error",
        ));
    }
    let node_role_value: Role = std::env::var("PAXOS_ROLE")
        .expect("Paxos roles not set")
        .try_into()?;
    if let Ok(mut node_role) = NODE_ROLE.write() {
        *node_role = node_role_value;
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "internal error",
        ));
    }
    let paxos_nodes_values = std::env::var("PAXOS_NODES").expect("Paxos nodes are not set");
    if let Ok(mut paxos_nodes) = PAXOS_NODES.write() {
        paxos_nodes.extend(paxos_nodes_values.split(',').map(|v| v.to_string()));
        println!("Paxos nodes: {:?}", paxos_nodes);
    }

    println!("Starting server...");
    HttpServer::new(|| App::new().service(consensus_start))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

async fn log(message: &str, value: &str) {
    let log_server = if let Ok(log_server) = LOG_SERVER.read() {
        log_server.clone()
    } else {
        EMPTY_STRING.clone()
    };
    match CLIENT
        .post(log_server)
        .body(format!(
            "{}, for node: {:?}, for value: {}",
            message,
            gethostname::gethostname(),
            value
        ))
        .send()
        .await
    {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::OK {
                println!("message logged");
            } else {
                println!("message not logged: {}", response.status());
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

fn get_next_id() -> u64 {
    PROPOSAL_COUNT.fetch_add(1, Ordering::Release) * 10 + NODE_ID.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};

    use super::*;

    #[actix_web::test]
    async fn should_start_consensus_process() {
        let app = test::init_service(App::new().service(consensus_start)).await;
        let req = test::TestRequest::post()
            .uri("/consensus")
            .set_payload("this is a value")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    async fn should_generate_next_id() {
        let mut ids = (0..10).map(|_| get_next_id()).collect::<Vec<_>>();
        ids.dedup();
        assert_eq!(ids, vec![0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);

        NODE_ID.store(1, Ordering::Release);
        PROPOSAL_COUNT.store(0, Ordering::Release);
        let mut ids = (0..10).map(|_| get_next_id()).collect::<Vec<_>>();
        ids.dedup();
        assert_eq!(ids, vec![1, 11, 21, 31, 41, 51, 61, 71, 81, 91]);
    }
}

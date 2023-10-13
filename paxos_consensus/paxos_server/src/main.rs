use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer};
use futures::future::join_all;
use futures_util::StreamExt as _;
use lazy_static::lazy_static;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref NODE_ID: AtomicU64 = AtomicU64::new(0);
    static ref LOG_SERVER: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref NODE_ROLE: RwLock<Role> = RwLock::new(Role::Learner);
    static ref PAXOS_ACCEPTOR_NODES: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref PAXOS_LEARNER_NODES: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref CLIENT: Client = reqwest::Client::new();
    // Proposal phase
    static ref PROPOSAL_ID: AtomicU64 = AtomicU64::new(0);
    static ref PROPOSAL_NUMBER_TO_IGNORE: AtomicU64 = AtomicU64::new(0);
    // Accepting phase
    static ref CURRENT_VALUE: RwLock<Option<String>> = RwLock::new(None);
    // Read phase
    static ref EMPTY_STRING: String = "".to_owned();
}

pub type Propose = u64;

#[derive(Deserialize, Serialize)]
pub struct Accept<'a> {
    proposal_number: u64,
    value: &'a str,
}

impl<'a> Accept<'a> {
    fn new(proposal_number: u64, value: &'a str) -> Self {
        Self {
            proposal_number,
            value,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    let role = *NODE_ROLE.read().unwrap();
    if role != Role::Proposer {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let value = bytes.escape_ascii().to_string();
    log("Proposer: Consensus started", &value).await;
    let proposal_number = get_next_id();
    let mut acceptors_amount = 0;
    let futures = PAXOS_ACCEPTOR_NODES.read().map(|paxos_acceptor_nodes| {
        acceptors_amount = paxos_acceptor_nodes.len();
        paxos_acceptor_nodes
            .iter()
            .map(|n| {
                CLIENT
                    .post(format!("{}/propose", n))
                    .body(proposal_number.to_string())
                    .send()
            })
            .collect::<Vec<_>>()
    });
    let futures = if let Ok(futures) = futures {
        join_all(futures).await
    } else {
        vec![]
    };
    let mut promised_amount = 0;
    for response in futures {
        match response {
            Ok(v) => {
                if v.status() == StatusCode::OK {
                    log("Proposer: Propose sent", &proposal_number.to_string()).await;
                    promised_amount += 1;
                } else {
                    log(
                        "Proposer: Propose sent with errors",
                        &v.status().to_string(),
                    )
                    .await;
                }
            }
            Err(e) => log("Proposer: Error", &e.to_string()).await,
        }
    }

    if promised_amount > acceptors_amount / 2 {
        let mut acceptors_accepted_amount = 0;
        let futures = PAXOS_ACCEPTOR_NODES.read().map(|paxos_acceptor_nodes| {
            acceptors_accepted_amount = paxos_acceptor_nodes.len();
            paxos_acceptor_nodes
                .iter()
                .map(|n| {
                    CLIENT
                        .post(format!("{}/accept", n))
                        .json(&Accept::new(proposal_number, &value))
                        .send()
                })
                .collect::<Vec<_>>()
        });
        let futures = if let Ok(futures) = futures {
            join_all(futures).await
        } else {
            vec![]
        };
        let mut accepted_amount = 0;
        for response in futures {
            match response {
                Ok(v) => {
                    if v.status() == StatusCode::ACCEPTED {
                        log("Proposer: Accept sent", &proposal_number.to_string()).await;
                        accepted_amount += 1;
                    } else {
                        log("Proposer: Accept sent with errors", &v.status().to_string()).await;
                    }
                }
                Err(e) => log("Proposer: Error", &e.to_string()).await,
            }
        }
        if accepted_amount > acceptors_amount / 2 {
            return Ok(HttpResponse::Ok().body(format!("Value {value} accepted!")));
        }
    }
    Ok(HttpResponse::NotAcceptable().finish())
}

#[post("/propose")]
async fn propose(mut value: web::Payload) -> Result<HttpResponse, Error> {
    log("Acceptor: Propose started", "").await;
    let role = *NODE_ROLE.read().unwrap();
    if role == Role::Learner {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    let proposal_number: Propose = String::from_utf8(bytes.to_vec())
        .unwrap()
        .parse::<u64>()
        .unwrap();
    if proposal_number < PROPOSAL_NUMBER_TO_IGNORE.load(Ordering::Acquire) {
        log(
            "Acceptor: Propose not acceptable",
            &proposal_number.to_string(),
        )
        .await;
        Ok(HttpResponse::NotAcceptable().finish())
    } else {
        PROPOSAL_NUMBER_TO_IGNORE.store(proposal_number, Ordering::Release);
        log("Acceptor: promised", &proposal_number.to_string()).await;
        Ok(HttpResponse::Ok().finish())
    }
}

#[post("/accept")]
async fn accept(mut value: web::Payload) -> Result<HttpResponse, Error> {
    log("Acceptor: Accept start", "").await;
    let role = *NODE_ROLE.read().unwrap();
    if role == Role::Learner {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    if let Ok(value) = serde_json::from_slice::<Accept>(&bytes) {
        let promised = PROPOSAL_NUMBER_TO_IGNORE.load(Ordering::Acquire);
        if value.proposal_number < promised {
            log(
                "Acceptor: Accept not acceptable already promised higher number",
                &promised.to_string(),
            )
            .await;
            Ok(HttpResponse::NotAcceptable().finish())
        } else {
            log("Acceptor: Trying to accept", value.value).await;
            let futures = PAXOS_ACCEPTOR_NODES.read().map(|paxos_acceptor_nodes| {
                paxos_acceptor_nodes
                    .iter()
                    .map(|n| {
                        CLIENT
                            .post(format!("{}/update_value", n))
                            .json(&value)
                            .send()
                    })
                    .collect::<Vec<_>>()
            });
            let futures = if let Ok(futures) = futures {
                join_all(futures).await
            } else {
                vec![]
            };
            for response in futures {
                match response {
                    Ok(v) => {
                        if v.status() == StatusCode::OK {
                            log("Acceptor: Value updated", "").await;
                        } else {
                            log(
                                "Acceptor: Value updated with error",
                                &v.status().to_string(),
                            )
                            .await;
                        }
                    }
                    Err(e) => log("Acceptor: Error", &e.to_string()).await,
                }
            }
            log("Acceptor: Accepted value", value.value).await;
            Ok(HttpResponse::Accepted().finish())
        }
    } else {
        log("Acceptor: Accept value not valid", "").await;
        Ok(HttpResponse::BadRequest().finish())
    }
}

#[post("/update_value")]
async fn update_value(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    if let Ok(value) = serde_json::from_slice::<Accept>(&bytes) {
        if let Ok(mut current_value) = CURRENT_VALUE.write() {
            *current_value = Some(value.value.to_string());
        }
        log("Leaner: Accepted value", value.value).await;
        Ok(HttpResponse::Ok().finish())
    } else {
        log("Learner: Accept value not valid", "").await;
        Ok(HttpResponse::BadRequest().finish())
    }
}

#[get("/value")]
async fn get_value() -> Result<HttpResponse, Error> {
    let current_value = CURRENT_VALUE.read().map_or(None, |v| v.clone());
    if let Some(current_value) = current_value {
        Ok(HttpResponse::Ok().body(current_value))
    } else {
        log("Learner: Get value not possible (value not set)", "").await;
        Ok(HttpResponse::NotFound().finish())
    }
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
    let paxos_acceptor_nodes_values =
        std::env::var("PAXOS_ACCEPTOR_NODES").expect("Paxos acceptor nodes are not set");
    if let Ok(mut paxos_acceptor_nodes) = PAXOS_ACCEPTOR_NODES.write() {
        paxos_acceptor_nodes.extend(
            paxos_acceptor_nodes_values
                .split(',')
                .map(|v| v.to_string()),
        );
        println!("Paxos nodes: {:?}", paxos_acceptor_nodes);
    }

    let paxos_learner_nodes_values =
        std::env::var("PAXOS_LEARNER_NODES").expect("Paxos learner nodes are not set");
    if let Ok(mut paxos_learner_nodes) = PAXOS_LEARNER_NODES.write() {
        paxos_learner_nodes.extend(paxos_learner_nodes_values.split(',').map(|v| v.to_string()));
        println!("Paxos nodes: {:?}", paxos_learner_nodes);
    }

    println!("Starting server...");
    HttpServer::new(|| {
        App::new()
            .service(consensus_start)
            .service(propose)
            .service(accept)
            .service(update_value)
            .service(get_value)
    })
    .bind((
        "0.0.0.0",
        std::env::var("PORT").unwrap().parse::<u16>().unwrap(),
    ))?
    .workers(3)
    .run()
    .await
}

async fn log(message: &str, value: &str) {
    let log_server = if let Ok(log_server) = LOG_SERVER.read() {
        log_server.clone()
    } else {
        EMPTY_STRING.clone()
    };
    let body = if value.is_empty() {
        format!(
            "{}, for node: {}",
            message,
            gethostname::gethostname().to_str().unwrap(),
        )
    } else {
        format!(
            "{}, for node: {}, for value: {}",
            message,
            gethostname::gethostname().to_str().unwrap(),
            value
        )
    };
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

fn get_next_id() -> u64 {
    PROPOSAL_ID.fetch_add(1, Ordering::Release) * 10 + NODE_ID.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};

    use super::*;

    #[actix_web::test]
    async fn should_start_consensus_process() {
        reset_values();
        if let Ok(mut node_role) = NODE_ROLE.write() {
            *node_role = Role::Proposer;
        }
        let mut server = mockito::Server::new();
        if let Ok(mut paxos_acceptor_nodes) = PAXOS_ACCEPTOR_NODES.write() {
            paxos_acceptor_nodes.push(server.url());
        }
        let mock_propose = server
            .mock("POST", "/propose")
            .with_status(StatusCode::OK.as_u16() as usize)
            .create();
        let mock_accept = server
            .mock("POST", "/accept")
            .with_status(StatusCode::ACCEPTED.as_u16() as usize)
            .create();
        let app = test::init_service(
            App::new()
                .service(consensus_start)
                .service(propose)
                .service(accept),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/consensus")
            .set_payload("this is a value")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        mock_propose.assert();
        mock_accept.assert();
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }

    #[actix_web::test]
    async fn should_promise_for_value() {
        reset_values();
        if let Ok(mut node_role) = NODE_ROLE.write() {
            *node_role = Role::Acceptor;
        }
        let app = test::init_service(App::new().service(propose)).await;
        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472389.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }

    #[actix_web::test]
    async fn should_accept_value() {
        reset_values();
        if let Ok(mut node_role) = NODE_ROLE.write() {
            *node_role = Role::Acceptor;
        }
        let mut server = mockito::Server::new();
        if let Ok(mut paxos_acceptor_nodes) = PAXOS_ACCEPTOR_NODES.write() {
            paxos_acceptor_nodes.push(server.url());
        }
        let mock_update_value = server
            .mock("POST", "/update_value")
            .with_status(StatusCode::OK.as_u16() as usize)
            .create();
        let app = test::init_service(App::new().service(accept)).await;
        let req = test::TestRequest::post()
            .uri("/accept")
            .set_payload(serde_json::to_string(&Accept::new(1001, "value")).unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        mock_update_value.assert();
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }

    #[test]
    async fn should_generate_next_id() {
        let mut ids = (0..10).map(|_| get_next_id()).collect::<Vec<_>>();
        ids.dedup();
        assert_eq!(ids, vec![0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);

        NODE_ID.store(1, Ordering::Release);
        PROPOSAL_ID.store(0, Ordering::Release);
        let mut ids = (0..10).map(|_| get_next_id()).collect::<Vec<_>>();
        ids.dedup();
        assert_eq!(ids, vec![1, 11, 21, 31, 41, 51, 61, 71, 81, 91]);
    }

    fn reset_values() {
        NODE_ID.store(0, Ordering::Release);
        NODE_ROLE.write().map(|mut n| *n = Role::Learner).unwrap();
        PAXOS_ACCEPTOR_NODES.write().map(|mut n| n.clear()).unwrap();
        PAXOS_LEARNER_NODES.write().map(|mut n| n.clear()).unwrap();
        PROPOSAL_ID.store(0, Ordering::Release);
        PROPOSAL_NUMBER_TO_IGNORE.store(0, Ordering::Release);
        CURRENT_VALUE.write().map(|mut v| *v = None).unwrap();
    }
}

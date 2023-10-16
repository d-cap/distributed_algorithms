use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use actix_web::{App, HttpServer};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    acceptor::{accept, propose},
    learner::{get_value, update_value},
    proposer::consensus_start,
};

mod acceptor;
mod learner;
mod proposer;

lazy_static! {
    static ref NODE_ID: AtomicU64 = AtomicU64::new(0);
    static ref LOG_SERVER: RwLock<String> = RwLock::new("invalid-server".to_owned());
    static ref NODE_ROLE: RwLock<Role> = RwLock::new(Role::Learner);
    static ref PAXOS_ACCEPTOR_NODES: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref PAXOS_LEARNER_NODES: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref CLIENT: Client = reqwest::Client::new();
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

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};

    use crate::proposer::{PROPOSAL_ID, PROPOSAL_NUMBER_TO_IGNORE};

    use super::*;

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

    pub fn reset_values() {
        NODE_ID.store(0, Ordering::Release);
        NODE_ROLE.write().map(|mut n| *n = Role::Learner).unwrap();
        PAXOS_ACCEPTOR_NODES.write().map(|mut n| n.clear()).unwrap();
        PAXOS_LEARNER_NODES.write().map(|mut n| n.clear()).unwrap();
        PROPOSAL_ID.store(0, Ordering::Release);
        PROPOSAL_NUMBER_TO_IGNORE.store(0, Ordering::Release);
        CURRENT_VALUE.write().map(|mut v| *v = None).unwrap();
    }
}

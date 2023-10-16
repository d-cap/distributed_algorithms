use std::sync::atomic::{AtomicU64, Ordering};

use actix_web::{post, web, Error, HttpResponse};
use futures::future::join_all;
use futures_util::StreamExt as _;
use lazy_static::lazy_static;
use reqwest::StatusCode;

use crate::{log, Accept, Role, CLIENT, NODE_ID, NODE_ROLE, PAXOS_ACCEPTOR_NODES};

lazy_static! {
    // Proposal phase
    pub static ref PROPOSAL_ID: AtomicU64 = AtomicU64::new(0);
    pub static ref PROPOSAL_NUMBER_TO_IGNORE: AtomicU64 = AtomicU64::new(0);
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

fn get_next_id() -> u64 {
    PROPOSAL_ID.fetch_add(1, Ordering::Release) * 10 + NODE_ID.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use actix_web::{http::StatusCode, test, App};

    use crate::{
        acceptor::{accept, propose},
        proposer::{consensus_start, get_next_id, PROPOSAL_ID},
        tests::reset_values,
        Role, CURRENT_VALUE, NODE_ID, NODE_ROLE, PAXOS_ACCEPTOR_NODES,
    };

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
}

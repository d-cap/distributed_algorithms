use std::sync::atomic::Ordering;

use actix_web::{post, web, Error, HttpResponse};
use futures::future::join_all;
use futures_util::StreamExt as _;
use reqwest::StatusCode;

use crate::{
    log, proposer::PROPOSAL_NUMBER_TO_IGNORE, Accept, Propose, Role, CLIENT, NODE_ROLE,
    PAXOS_ACCEPTOR_NODES,
};

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

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};
}

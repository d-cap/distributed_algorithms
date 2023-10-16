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

    use crate::{tests::reset_values, Role, CURRENT_VALUE, NODE_ROLE};

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
    async fn should_promise_with_higher_promise() {
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

        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472388.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }

    #[actix_web::test]
    async fn should_promise_with_accept_with_higher_value() {
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
        let app = test::init_service(App::new().service(propose).service(accept)).await;
        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472389.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
        let req = test::TestRequest::post()
            .uri("/accept")
            .set_payload(serde_json::to_string(&Accept::new(1483472389, "value")).unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        mock_update_value.assert();
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);

        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472388.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
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

    #[actix_web::test]
    async fn should_promise_and_accept_value() {
        reset_values();
        if let Ok(mut node_role) = NODE_ROLE.write() {
            *node_role = Role::Acceptor;
        }
        let app = test::init_service(App::new().service(propose).service(accept)).await;
        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472389.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);

        let mut server = mockito::Server::new();
        if let Ok(mut paxos_acceptor_nodes) = PAXOS_ACCEPTOR_NODES.write() {
            paxos_acceptor_nodes.push(server.url());
        }
        let mock_update_value = server
            .mock("POST", "/update_value")
            .with_status(StatusCode::OK.as_u16() as usize)
            .create();
        let req = test::TestRequest::post()
            .uri("/accept")
            .set_payload(serde_json::to_string(&Accept::new(1483472389, "value")).unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        mock_update_value.assert();
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }

    #[actix_web::test]
    async fn should_not_accept_with_higher_promise() {
        reset_values();
        if let Ok(mut node_role) = NODE_ROLE.write() {
            *node_role = Role::Acceptor;
        }
        let app = test::init_service(App::new().service(propose).service(accept)).await;
        let req = test::TestRequest::post()
            .uri("/propose")
            .set_payload(1483472389.to_string())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);

        let mut server = mockito::Server::new();
        if let Ok(mut paxos_acceptor_nodes) = PAXOS_ACCEPTOR_NODES.write() {
            paxos_acceptor_nodes.push(server.url());
        }
        let mock_update_value = server
            .mock("POST", "/update_value")
            .with_status(StatusCode::OK.as_u16() as usize)
            .create();
        let req = test::TestRequest::post()
            .uri("/accept")
            .set_payload(serde_json::to_string(&Accept::new(1483472388, "value")).unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        mock_update_value.expect_at_most(0);
        assert_eq!(*CURRENT_VALUE.read().unwrap(), None);
    }
}

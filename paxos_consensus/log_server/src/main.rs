use std::{sync::Mutex, time::Instant};

use actix_web::{error, post, web, App, Error, HttpResponse, HttpServer};

use lazy_static::lazy_static;

lazy_static! {
    static ref LOGS: Mutex<Vec<(Instant, String)>> = Mutex::new(Vec::new());
}

#[post("/log")]
async fn log(message: web::Payload) -> Result<HttpResponse, Error> {
    println!("Message received");
    if let Ok(bytes) = message.to_bytes().await {
        let m = String::from_utf8(bytes.to_vec());
        if let Ok(m) = m {
            if let Ok(mut logs) = LOGS.lock() {
                logs.push((Instant::now(), m));
                println!("Message saved: {:?}", logs.last().unwrap());
                return Ok(HttpResponse::Ok().finish());
            } else {
                return Err(error::ErrorInternalServerError("not possible to save data"));
            }
        } else {
            return Err(error::ErrorBadRequest("data is not a string"));
        }
    }
    Err(error::ErrorBadRequest(
        "request not valid, not possible to read the body",
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server...");
    HttpServer::new(|| App::new().service(log))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use actix_web::{test, App};

    use super::*;

    #[actix_web::test]
    async fn should_save_message() {
        let app = test::init_service(App::new().service(log)).await;
        let body_message = "this is a message".to_string();
        let req = test::TestRequest::post()
            .uri("/log")
            .set_payload(body_message.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        if let Ok(logs) = LOGS.lock() {
            assert_eq!(body_message, logs.last().unwrap().1);
        }
    }
}

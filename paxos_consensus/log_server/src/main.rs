use std::{sync::Mutex, time::Instant};

use actix_web::{error, post, web, App, Error, HttpResponse, HttpServer};

use lazy_static::lazy_static;

lazy_static! {
    static ref LOGS: Mutex<Vec<(Instant, String)>> = Mutex::new(Vec::new());
}

#[post("/log")]
async fn greet(message: web::Payload) -> Result<HttpResponse, Error> {
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
    HttpServer::new(|| App::new().service(greet))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

use actix_web::{get, web, App, HttpServer, Responder};

enum Role {
    Proposer,
    Acceptor,
    Learner,
}

#[get("/consensus/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    println!("Consensus message start");
    let log_server = std::env::var("LOG_SERVER").expect("Log server not set");
    let client = reqwest::Client::new();
    match client
        .post(log_server)
        .body("consensum message arrived")
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
    format!("Hello {name}!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_server = std::env::var("LOG_SERVER").expect("Log server not set");
    println!("Log server used: {}", log_server);
    let roles = std::env::var("PAXOS_ROLES").expect("Paxos roles not set");
    for r in roles.split(',') {
        println!("Role: {}", r);
    }

    println!("Starting server...");
    HttpServer::new(|| App::new().service(greet))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

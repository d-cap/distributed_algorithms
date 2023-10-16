use actix_web::{get, post, web, Error, HttpResponse};
use futures_util::StreamExt as _;

use crate::{log, Accept, CURRENT_VALUE};

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

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};
}

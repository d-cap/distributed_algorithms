use actix_web::{get, post, web, Error, HttpResponse};
use futures_util::StreamExt as _;

use crate::{log, Accept, CURRENT_VALUE};

#[post("/update_value")]
async fn update_value(mut value: web::Payload) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = value.next().await {
        bytes.extend_from_slice(&item?);
    }
    match serde_json::from_slice::<Accept>(&bytes) {
        Ok(value) => {
            if let Ok(mut current_value) = CURRENT_VALUE.write() {
                *current_value = Some(value.value.to_string());
            }
            log("Leaner: Accepted value", value.value).await;
            Ok(HttpResponse::Ok().finish())
        }
        Err(e) => {
            log("Learner: Accept value not valid", &e.to_string()).await;
            Ok(HttpResponse::BadRequest().finish())
        }
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
    use actix_web::{body::to_bytes, http::StatusCode, test, App};
    use futures_util::stream;

    use crate::tests::reset_values;

    use super::*;

    #[actix_web::test]
    async fn should_read_without_value() {
        reset_values();
        let app = test::init_service(App::new().service(get_value)).await;
        let req = test::TestRequest::get().uri("/value").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn should_read_with_value() {
        reset_values();
        if let Ok(mut current_value) = CURRENT_VALUE.write() {
            *current_value = Some("this is the current value".to_owned());
        }
        let app = test::init_service(App::new().service(get_value)).await;
        let req = test::TestRequest::get().uri("/value").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(body, "this is the current value");
    }

    #[actix_web::test]
    async fn should_write_without_value() {
        reset_values();
        let app = test::init_service(App::new().service(update_value)).await;
        let req = test::TestRequest::post()
            .uri("/update_value")
            .set_payload(
                serde_json::to_string(&Accept::new(1, "this is another current value")).unwrap(),
            )
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        if let Ok(current_value) = CURRENT_VALUE.read() {
            assert_eq!(
                *current_value,
                Some("this is another current value".to_owned())
            );
        } else {
            panic!("This should be present");
        }
    }

    #[actix_web::test]
    async fn should_write_with_value() {
        reset_values();
        if let Ok(mut current_value) = CURRENT_VALUE.write() {
            *current_value = Some("this is the current value".to_owned());
        }
        let app = test::init_service(App::new().service(update_value)).await;
        let req = test::TestRequest::post()
            .uri("/update_value")
            .set_payload(
                serde_json::to_string(&Accept::new(1, "this is another current value")).unwrap(),
            )
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        if let Ok(current_value) = CURRENT_VALUE.read() {
            assert_eq!(
                *current_value,
                Some("this is another current value".to_owned())
            );
        } else {
            panic!("This should be present");
        }
    }

    #[actix_web::test]
    async fn should_write_with_value_and_multiple_calls() {
        reset_values();
        if let Ok(mut current_value) = CURRENT_VALUE.write() {
            *current_value = Some("this is the current value".to_owned());
        }
        let app = actix_test::start(|| App::new().service(update_value));
        let client = reqwest::Client::new();
        let url = app.url("/update_value");
        let futures = stream::iter(
            (0..3)
                .map(|i| (url.clone(), format!("this is another current value {}", i)))
                .collect::<Vec<_>>(),
        )
        .map(|v| {
            let client = client.clone();
            tokio::spawn(async move {
                client
                    .post(v.0)
                    .body(serde_json::to_string(&Accept::new(1, &v.1)).unwrap())
                    .send()
                    .await
            })
        })
        .buffer_unordered(3);

        futures
            .for_each(|f| async move {
                assert_eq!(f.unwrap().unwrap().status(), StatusCode::OK);
            })
            .await;
        if let Ok(current_value) = CURRENT_VALUE.read() {
            assert_ne!(*current_value, None);
        } else {
            panic!("This should be present");
        }
    }
}

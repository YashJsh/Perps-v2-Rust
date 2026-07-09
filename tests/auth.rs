use actix_web::{App, body::to_bytes, http::StatusCode, test, web};
use perps_v1::{
    controllers::auth::{sign_in, sign_up},
    store::AppState,
    utils::user::User,
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::mpsc;

fn test_app_state() -> web::Data<AppState> {
    let (tx, _rx) = mpsc::channel(10);
    web::Data::new(AppState {
        users: Mutex::new(HashMap::new()),
        sender: tx,
    })
}

#[actix_web::test]
async fn signup_returns_created_for_new_user() {
    let app_state = test_app_state();
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/signup", web::post().to(sign_up)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/signup")
        .set_json(json!({
            "user_id": 1,
            "password": "password123"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = to_bytes(response.into_body()).await.unwrap();

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body, r#""User created succcessfully""#);
}

#[actix_web::test]
async fn signup_rejects_duplicate_user_ids() {
    let app_state = test_app_state();
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/signup", web::post().to(sign_up)),
    )
    .await;

    let first_request = test::TestRequest::post()
        .uri("/signup")
        .set_json(json!({
            "user_id": 1,
            "password": "password123"
        }))
        .to_request();
    let first_response = test::call_service(&app, first_request).await;
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let second_request = test::TestRequest::post()
        .uri("/signup")
        .set_json(json!({
            "user_id": 1,
            "password": "different-password"
        }))
        .to_request();

    let second_response = test::call_service(&app, second_request).await;
    let status = second_response.status();
    let body = to_bytes(second_response.into_body()).await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, r#""User already exists""#);
}

#[actix_web::test]
async fn signin_returns_token_for_valid_credentials() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret");
    }

    let app_state = test_app_state();
    app_state.users.lock().unwrap().insert(
        1,
        User {
            id: 1,
            username: "alice".to_string(),
            password: "password123".to_string(),
        },
    );

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/signin", web::post().to(sign_in)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/signin")
        .set_json(json!({
            "user_id": 1,
            "password": "password123"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = test::read_body(response).await;
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["success"], true);
    assert_eq!(payload["user_id"], 1);
    assert_eq!(payload["token"].as_str().unwrap().is_empty(), false);
}

#[actix_web::test]
async fn signin_rejects_incorrect_password() {
    let app_state = test_app_state();
    app_state.users.lock().unwrap().insert(
        1,
        User {
            id: 1,
            username: "alice".to_string(),
            password: "password123".to_string(),
        },
    );

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/signin", web::post().to(sign_in)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/signin")
        .set_json(json!({
            "user_id": 1,
            "password": "wrong-password"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = to_bytes(response.into_body()).await.unwrap();

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, r#""Incorrect Password""#);
}

#[actix_web::test]
async fn signin_returns_not_found_for_unknown_user() {
    let app_state = test_app_state();
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/signin", web::post().to(sign_in)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/signin")
        .set_json(json!({
            "user_id": 99,
            "password": "password123"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = to_bytes(response.into_body()).await.unwrap();

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, r#""User not found""#);
}

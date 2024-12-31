use std::sync::Arc;

use axum::{routing::post, Router};
use solana_client::nonblocking::rpc_client::RpcClient;
use tower_http::cors::CorsLayer;

mod constants;
mod endpoints;
mod utils;

use constants::*;
use endpoints::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
    let rpc_client = Arc::new(RpcClient::new(rpc_url.clone()));

    let cors = CorsLayer::new()
        .allow_origin(["https://yoko.fund".parse().unwrap()])
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    let app = Router::<()>::new()
        .route(
            "/get-fund",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                let rpc_url = rpc_url.clone();
                move |body| get_fund(body, rpc_client, rpc_url)
            }),
        )
        .route(
            "/get-swap-msg",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                move |body| get_swap_msg(body, rpc_client)
            }),
        )
        .route(
            "/get-create-fund-msg",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                move |body| get_create_fund_msg(body, rpc_client)
            }),
        )
        .route(
            "/get-deposit-msg",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                move |body| get_deposit_msg(body, rpc_client)
            }),
        )
        .route(
            "/get-create-payout-msg",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                move |body| get_create_payout_msg(body, rpc_client)
            }),
        )
        .route(
            "/get-claim-payout-msg",
            post({
                let rpc_client = Arc::clone(&rpc_client);
                move |body| get_claim_payout_msg(body, rpc_client)
            }),
        )
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server is running on port {}", port);

    axum::serve(listener, app).await.unwrap();
}

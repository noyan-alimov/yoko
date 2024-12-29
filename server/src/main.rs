use std::sync::Arc;

use axum::{routing::post, Router};
use solana_client::nonblocking::rpc_client::RpcClient;
use tower_http::cors::{Any, CorsLayer};

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
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Server is running on port 8000");

    axum::serve(listener, app).await.unwrap();
}

use std::{str::FromStr, sync::Arc};

use axum::Json;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use spl_token::state::Mint;
use steel::Pubkey;

use crate::utils::get_swap_message;

#[derive(Deserialize)]
pub struct GetSwapMsgPayload {
    pub fund_manager: String,
    pub from_mint: String,
    pub to_mint: String,
    pub in_amount: f64,
    pub quote: serde_json::Value,
}

#[derive(Serialize)]
pub struct GetSwapMsgResponse {
    pub msg: String,
}

pub async fn get_swap_msg(
    Json(payload): Json<GetSwapMsgPayload>,
    rpc_client: Arc<RpcClient>,
) -> Result<Json<GetSwapMsgResponse>, (axum::http::StatusCode, String)> {
    if payload.in_amount <= 0.0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "In amount must be greater than 0".to_string(),
        ));
    }
    let fund_manager_pubkey = Pubkey::from_str(&payload.fund_manager).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid fund manager pubkey".to_string(),
        )
    })?;
    let from_mint_pubkey = Pubkey::from_str(&payload.from_mint).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid from mint pubkey".to_string(),
        )
    })?;
    let to_mint_pubkey = Pubkey::from_str(&payload.to_mint).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid to mint pubkey".to_string(),
        )
    })?;
    let from_mint_data = rpc_client
        .get_account_data(&from_mint_pubkey)
        .await
        .unwrap();
    let from_mint_data = Mint::unpack(&from_mint_data).unwrap();
    let in_amount = (payload.in_amount * (10u64.pow(from_mint_data.decimals as u32) as f64)) as u64;

    Ok(Json(GetSwapMsgResponse {
        msg: get_swap_message(
            rpc_client,
            fund_manager_pubkey,
            from_mint_pubkey,
            to_mint_pubkey,
            in_amount,
            payload.quote,
        )
        .await,
    }))
}

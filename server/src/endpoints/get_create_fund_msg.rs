use std::{str::FromStr, sync::Arc};

use axum::Json;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{compute_budget::ComputeBudgetInstruction, message::v0::Message};
use steel::Pubkey;
use yoko_program_api::{
    sdk::create_fund,
    state::{fund_pda, fund_token_account_pda},
};

#[derive(Deserialize)]
pub struct GetCreateFundMsgPayload {
    pub fund_manager: String,
    pub main_mint: String,
    pub authority_fee: u64,
}

#[derive(Serialize)]
pub struct GetCreateFundMsgResponse {
    pub msg: String,
}

pub async fn get_create_fund_msg(
    Json(payload): Json<GetCreateFundMsgPayload>,
    rpc_client: Arc<RpcClient>,
) -> Result<Json<GetCreateFundMsgResponse>, (axum::http::StatusCode, String)> {
    let fund_manager_pubkey = Pubkey::from_str(&payload.fund_manager).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid fund manager pubkey".to_string(),
        )
    })?;
    let main_mint_pubkey = Pubkey::from_str(&payload.main_mint).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid main mint pubkey".to_string(),
        )
    })?;
    let fund = fund_pda(&fund_manager_pubkey).0;
    let main_token_account = fund_token_account_pda(&fund, &main_mint_pubkey).0;

    let create_fund_ixn = create_fund(
        fund,
        fund_manager_pubkey,
        main_mint_pubkey,
        main_token_account,
        payload.authority_fee,
    );

    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(250_000),
        ComputeBudgetInstruction::set_compute_unit_price(500_000),
    ]);

    instructions.push(create_fund_ixn);

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    let message =
        Message::try_compile(&fund_manager_pubkey, &instructions, &[], recent_blockhash).unwrap();

    Ok(Json(GetCreateFundMsgResponse {
        msg: base64::encode(message.serialize()),
    }))
}

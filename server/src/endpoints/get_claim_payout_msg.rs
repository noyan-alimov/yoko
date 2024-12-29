use std::{str::FromStr, sync::Arc};

use axum::Json;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{compute_budget::ComputeBudgetInstruction, message::v0::Message};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_token::instruction::close_account;
use steel::{AccountDeserialize, Pubkey};
use yoko_program_api::{
    sdk::claim_payout,
    state::{payout_pda, payout_token_account_pda, position_pda, Fund, Position},
};

use crate::WSOL;

#[derive(Deserialize)]
pub struct GetClaimPayoutMsgPayload {
    pub fund: String,
    pub depositor: String,
}

#[derive(Serialize)]
pub struct GetClaimPayoutMsgResponse {
    pub msg: String,
}

pub async fn get_claim_payout_msg(
    Json(payload): Json<GetClaimPayoutMsgPayload>,
    rpc_client: Arc<RpcClient>,
) -> Result<Json<GetClaimPayoutMsgResponse>, (axum::http::StatusCode, String)> {
    let fund_pubkey = Pubkey::from_str(&payload.fund).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid fund pubkey".to_string(),
        )
    })?;
    let depositor_pubkey = Pubkey::from_str(&payload.depositor).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid depositor pubkey".to_string(),
        )
    })?;
    let position = position_pda(&fund_pubkey, &depositor_pubkey).0;
    let position_data = rpc_client.get_account_data(&position).await.unwrap();
    let position_data = Position::try_from_bytes(&position_data).unwrap();
    let fund_data = rpc_client.get_account_data(&fund_pubkey).await.unwrap();
    let fund_data = Fund::try_from_bytes(&fund_data).unwrap();
    let depositor_main_token_account =
        get_associated_token_address(&depositor_pubkey, &fund_data.main_mint);
    let payout = payout_pda(&fund_pubkey, position_data.payouts_counter + 1).0;
    let payout_main_token_account = payout_token_account_pda(&payout).0;

    let claim_payout_ixn = claim_payout(
        position,
        depositor_pubkey,
        payout,
        payout_main_token_account,
        depositor_main_token_account,
        fund_pubkey,
    );

    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(250_000),
        ComputeBudgetInstruction::set_compute_unit_price(500_000),
        create_associated_token_account_idempotent(
            &depositor_pubkey,
            &depositor_pubkey,
            &fund_data.main_mint,
            &spl_token::ID,
        ),
        claim_payout_ixn,
    ]);

    if fund_data.main_mint == Pubkey::from_str(WSOL).unwrap() {
        instructions.push(
            close_account(
                &spl_token::ID,
                &depositor_main_token_account,
                &depositor_pubkey,
                &depositor_pubkey,
                &[],
            )
            .unwrap(),
        );
    }

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    let message =
        Message::try_compile(&depositor_pubkey, &instructions, &[], recent_blockhash).unwrap();

    Ok(Json(GetClaimPayoutMsgResponse {
        msg: base64::encode(message.serialize()),
    }))
}

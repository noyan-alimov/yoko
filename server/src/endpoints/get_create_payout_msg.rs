use std::{str::FromStr, sync::Arc};

use axum::Json;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, message::v0::Message, program_pack::Pack,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_token::{instruction::close_account, state::Mint};
use steel::{AccountDeserialize, Pubkey};
use yoko_program_api::{
    sdk::create_payout,
    state::{fund_token_account_pda, payout_pda, payout_token_account_pda, Fund},
};

use crate::WSOL;

#[derive(Deserialize)]
pub struct GetCreatePayoutMsgPayload {
    pub fund: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct GetCreatePayoutMsgResponse {
    pub msg: String,
}

pub async fn get_create_payout_msg(
    Json(payload): Json<GetCreatePayoutMsgPayload>,
    rpc_client: Arc<RpcClient>,
) -> Result<Json<GetCreatePayoutMsgResponse>, (axum::http::StatusCode, String)> {
    if payload.amount <= 0.0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Amount must be greater than 0".to_string(),
        ));
    }
    let fund_pubkey = Pubkey::from_str(&payload.fund).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid fund pubkey".to_string(),
        )
    })?;
    let fund_data = rpc_client.get_account_data(&fund_pubkey).await.unwrap();
    let fund_data = Fund::try_from_bytes(&fund_data).unwrap();
    let fund_authority_token_account =
        get_associated_token_address(&fund_data.authority, &fund_data.main_mint);
    let fund_main_token_account = fund_token_account_pda(&fund_pubkey, &fund_data.main_mint).0;
    let payout = payout_pda(&fund_pubkey, fund_data.payouts_counter + 1).0;
    let payout_main_token_account = payout_token_account_pda(&payout).0;

    let main_mint_data = rpc_client
        .get_account_data(&fund_data.main_mint)
        .await
        .unwrap();
    let main_mint_data = Mint::unpack(&main_mint_data).unwrap();
    let amount = (payload.amount * (10u64.pow(main_mint_data.decimals as u32) as f64)) as u64;
    let create_payout_ixn = create_payout(
        fund_data.authority,
        fund_authority_token_account,
        fund_pubkey,
        fund_main_token_account,
        payout,
        payout_main_token_account,
        fund_data.main_mint,
        amount,
    );

    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(250_000),
        ComputeBudgetInstruction::set_compute_unit_price(500_000),
        create_associated_token_account_idempotent(
            &fund_data.authority,
            &fund_data.authority,
            &fund_data.main_mint,
            &spl_token::ID,
        ),
        create_payout_ixn,
    ]);

    if fund_data.main_mint == Pubkey::from_str(WSOL).unwrap() {
        instructions.push(
            close_account(
                &spl_token::ID,
                &fund_authority_token_account,
                &fund_data.authority,
                &fund_data.authority,
                &[],
            )
            .unwrap(),
        );
    }

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    let message =
        Message::try_compile(&fund_data.authority, &instructions, &[], recent_blockhash).unwrap();

    Ok(Json(GetCreatePayoutMsgResponse {
        msg: base64::encode(message.serialize()),
    }))
}

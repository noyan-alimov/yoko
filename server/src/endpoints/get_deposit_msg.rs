use std::{str::FromStr, sync::Arc};

use axum::Json;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, message::v0::Message, program_pack::Pack,
    system_instruction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{instruction::sync_native, state::Mint};
use steel::{AccountDeserialize, Pubkey};
use yoko_program_api::{
    sdk::{create_position, deposit},
    state::{fund_token_account_pda, position_pda, Fund},
};

use crate::WSOL;

#[derive(Deserialize)]
pub struct GetDepositMsgPayload {
    pub fund: String,
    pub depositor: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct GetDepositMsgResponse {
    pub msg: String,
}

pub async fn get_deposit_msg(
    Json(payload): Json<GetDepositMsgPayload>,
    rpc_client: Arc<RpcClient>,
) -> Result<Json<GetDepositMsgResponse>, (axum::http::StatusCode, String)> {
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
    let depositor_pubkey = Pubkey::from_str(&payload.depositor).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid depositor pubkey".to_string(),
        )
    })?;
    let fund_data = rpc_client.get_account_data(&fund_pubkey).await.unwrap();
    let fund_data = Fund::try_from_bytes(&fund_data).unwrap();
    let fund_main_token_account = fund_token_account_pda(&fund_pubkey, &fund_data.main_mint).0;
    let main_mint_data = rpc_client
        .get_account_data(&fund_data.main_mint)
        .await
        .unwrap();
    let main_mint_data = Mint::unpack(&main_mint_data).unwrap();
    let position_pubkey = position_pda(&fund_pubkey, &depositor_pubkey).0;
    let depositor_token_account =
        get_associated_token_address(&depositor_pubkey, &fund_data.main_mint);

    let amount = (payload.amount * (10u64.pow(main_mint_data.decimals as u32) as f64)) as u64;

    let deposit_ixn = deposit(
        position_pubkey,
        fund_pubkey,
        fund_main_token_account,
        depositor_pubkey,
        depositor_token_account,
        amount,
    );

    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(250_000),
        ComputeBudgetInstruction::set_compute_unit_price(500_000),
    ]);

    if fund_data.main_mint
        == Pubkey::from_str(WSOL).unwrap()
    {
        match rpc_client.get_account_data(&depositor_token_account).await {
            Err(_) => {
                instructions.push(create_associated_token_account(
                    &depositor_pubkey,
                    &depositor_pubkey,
                    &fund_data.main_mint,
                    &spl_token::ID,
                ));
            }
            Ok(_) => {}
        }

        instructions.push(system_instruction::transfer(
            &depositor_pubkey,
            &depositor_token_account,
            amount,
        ));

        instructions.push(sync_native(&spl_token::ID, &depositor_token_account).unwrap());
    }

    match rpc_client.get_account_data(&position_pubkey).await {
        Err(_) => {
            let create_position_ixn =
                create_position(position_pubkey, fund_pubkey, depositor_pubkey);
            instructions.push(create_position_ixn);
        }
        Ok(_) => {}
    }

    instructions.push(deposit_ixn);

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    let message =
        Message::try_compile(&depositor_pubkey, &instructions, &[], recent_blockhash).unwrap();

    Ok(Json(GetDepositMsgResponse {
        msg: base64::encode(message.serialize()),
    }))
}

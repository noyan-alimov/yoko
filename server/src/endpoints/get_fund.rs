use std::{str::FromStr, sync::Arc};

use axum::Json;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use spl_token::state::Account;
use steel::{AccountDeserialize, Pubkey};
use yoko_program_api::state::{fund_pda, fund_token_account_pda, Fund};

#[derive(Deserialize)]
pub struct GetFundPayload {
    pub fund_manager: String,
}

#[derive(Serialize)]
pub struct Token {
    pub asset: Asset,
    pub token_account: String,
    pub amount: u64,
    pub ui_amount: f64,
    pub usd_amount: f64,
}
#[derive(Serialize)]
pub struct GetFundResponse {
    pub fund_pubkey: String,
    pub manager: String,
    pub total_deposited: u64,
    pub payouts_counter: u64,
    pub manager_fee: u64,
    pub main_token: Token,
    pub other_tokens: Vec<Token>,
    pub total_usd_amount: f64,
}

pub async fn get_fund(
    Json(payload): Json<GetFundPayload>,
    rpc_client: Arc<RpcClient>,
    helius_url: String,
) -> Result<Json<GetFundResponse>, (axum::http::StatusCode, String)> {
    let fund = fund_pda(&Pubkey::from_str(&payload.fund_manager).unwrap()).0;

    let fund_data = match rpc_client.get_account_data(&fund).await {
        Ok(data) => data,
        Err(_) => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Fund not found".to_string(),
            ))
        }
    };

    let fund_data = Fund::try_from_bytes(&fund_data).unwrap();

    let main_asset = get_asset(helius_url.clone(), fund_data.main_mint.to_string())
        .await
        .unwrap();
    let main_token_account = fund_token_account_pda(&fund, &fund_data.main_mint).0;
    let main_token_account_data = rpc_client
        .get_account_data(&main_token_account)
        .await
        .unwrap();
    let main_token_account_data = Account::unpack(&main_token_account_data).unwrap();

    let mut total_usd_amount = 0.0;
    let main_ui_amount =
        main_token_account_data.amount as f64 / 10.0_f64.powi(main_asset.decimals as i32);
    let main_usd_amount = (main_ui_amount * main_asset.price_info.price_per_token * 100.0).round() / 100.0;
    total_usd_amount += main_usd_amount;

    let main_token = Token {
        asset: main_asset.clone(),
        token_account: main_token_account.to_string(),
        amount: main_token_account_data.amount,
        ui_amount: main_ui_amount,
        usd_amount: main_usd_amount,
    };

    let mut other_tokens = vec![];

    for mint in fund_data.other_mints.iter() {
        let asset = get_asset(helius_url.clone(), mint.to_string())
            .await
            .unwrap();
        let token_account = fund_token_account_pda(&fund, mint).0;
        let token_account_data = rpc_client.get_account_data(&token_account).await.unwrap();
        let token_account_data = Account::unpack(&token_account_data).unwrap();

        let ui_amount = token_account_data.amount as f64 / 10.0_f64.powi(asset.decimals as i32);
        let usd_amount = (ui_amount * asset.price_info.price_per_token * 100.0).round() / 100.0;
        total_usd_amount += usd_amount;
        other_tokens.push(Token {
            asset: asset.clone(),
            token_account: token_account.to_string(),
            amount: token_account_data.amount,
            ui_amount,
            usd_amount,
        });
    }

    other_tokens.sort_by(|a, b| b.usd_amount.partial_cmp(&a.usd_amount).unwrap());

    Ok(Json(GetFundResponse {
        fund_pubkey: fund.to_string(),
        manager: fund_data.authority.to_string(),
        total_deposited: fund_data.total_deposited,
        payouts_counter: fund_data.payouts_counter,
        manager_fee: fund_data.authority_fee,
        main_token,
        other_tokens,
        total_usd_amount,
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub decimals: u8,
    pub price_info: PriceInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceInfo {
    pub price_per_token: f64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub links: Links,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetResponse {
    pub content: Content,
    pub token_info: TokenInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Asset {
    pub mint: String,
    pub image: String,
    pub symbol: String,
    pub decimals: u8,
    pub price_info: PriceInfo,
}

async fn get_asset(helius_url: String, mint: String) -> Result<Asset, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(helius_url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "my-id",
            "method": "getAsset",
            "params": {
                "id": mint,
                "displayOptions": {
                    "showFungible": true
                }
            }
        }))
        .send()
        .await?;

    let json_response: Value = response.json().await?;
    let asset: AssetResponse = serde_json::from_value(json_response["result"].clone())?;

    Ok(Asset {
        mint,
        image: asset.content.links.image,
        symbol: asset.token_info.symbol,
        decimals: asset.token_info.decimals,
        price_info: PriceInfo {
            price_per_token: asset.token_info.price_info.price_per_token,
            currency: asset.token_info.price_info.currency,
        },
    })
}

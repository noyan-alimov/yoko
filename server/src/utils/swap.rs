use std::{str::FromStr, sync::Arc};

use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{compute_budget::ComputeBudgetInstruction, message::v0::Message as MessageV0};
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use steel::{AccountDeserialize, AccountMeta, Instruction, Pubkey};
use yoko_program_api::{
    sdk::{create_fund_token_account, swap},
    state::{fund_pda, fund_token_account_pda, Fund},
};

pub async fn get_swap_message(
    rpc_client: Arc<RpcClient>,
    fund_manager_pubkey: Pubkey,
    from_mint_pubkey: Pubkey,
    to_mint_pubkey: Pubkey,
    amount: u64,
    quote: serde_json::Value,
) -> String {
    let fund_manager = fund_manager_pubkey.to_string();
    // let from_mint = from_mint_pubkey.to_string();
    // let to_mint = to_mint_pubkey.to_string();

    // let quote = get_jup_quote(&from_mint, &to_mint, amount).await.unwrap();

    let swap_ixn = get_jup_swap_ixn(fund_manager.as_str(), quote)
        .await
        .unwrap();

    let jupiter_data_base64 = swap_ixn["swapInstruction"]["data"].as_str().unwrap();
    let jupiter_data = base64::decode(jupiter_data_base64).unwrap();

    let jupiter_accounts = swap_ixn["swapInstruction"]["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|account| solana_sdk::instruction::AccountMeta {
            pubkey: Pubkey::from_str(account["pubkey"].as_str().unwrap()).unwrap(),
            is_signer: account["isSigner"].as_bool().unwrap(),
            is_writable: account["isWritable"].as_bool().unwrap(),
        })
        .collect::<Vec<_>>();

    let address_lookup_table_addresses: Vec<Pubkey> = swap_ixn["addressLookupTableAddresses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|addr| Pubkey::from_str(addr.as_str().unwrap()).unwrap())
        .collect();

    let address_lookup_table_accounts = futures::future::join_all(
        address_lookup_table_addresses.iter().map(|address| {
            let rpc_client = rpc_client.clone();
            async move {
                rpc_client
                    .get_account_with_commitment(address, rpc_client.commitment())
                    .await
                    .unwrap()
                    .value
                    .map(|account| {
                        let lookup_table = solana_address_lookup_table_program::state::AddressLookupTable::deserialize(
                            &account.data,
                        )
                        .unwrap();

                        solana_sdk::address_lookup_table_account::AddressLookupTableAccount {
                            key: *address,
                            addresses: lookup_table.addresses.into_iter().copied().collect(),
                        }
                    })
                    .unwrap()
            }
        }),
    )
    .await
    .into_iter()
    .collect::<Vec<_>>();
    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(500_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ]);

    instructions.extend(get_create_fund_manager_atas_ixns(
        fund_manager_pubkey,
        from_mint_pubkey,
        to_mint_pubkey,
    ));

    instructions.extend(
        get_swap_ixns(
            &rpc_client,
            fund_manager_pubkey,
            &jupiter_data,
            jupiter_accounts,
            from_mint_pubkey,
            to_mint_pubkey,
            amount,
        )
        .await
        .unwrap(),
    );

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    let message = MessageV0::try_compile(
        &fund_manager_pubkey,
        &instructions,
        &address_lookup_table_accounts,
        recent_blockhash,
    )
    .unwrap();

    base64::encode(message.serialize())
}

const JUP_API_ENDPOINT: &str = "https://quote-api.jup.ag/v6";

// async fn get_jup_quote(
//     from_mint: &str,
//     to_mint: &str,
//     amount: u64,
// ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
//     let client = reqwest::Client::new();

//     let url = format!(
//         "{}/quote?outputMint={}&inputMint={}&amount={}&slippage=0.5&onlyDirectRoutes=true",
//         JUP_API_ENDPOINT,
//         to_mint.to_string(),
//         from_mint.to_string(),
//         amount
//     );

//     let response = client
//         .get(&url)
//         .header("Accept", "application/json")
//         .send()
//         .await?
//         .json()
//         .await?;

//     Ok(response)
// }

#[derive(Serialize)]
struct SwapRequestData {
    #[serde(rename = "quoteResponse")]
    quote_response: serde_json::Value,
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
}

async fn get_jup_swap_ixn(
    user: &str,
    quote: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let data = SwapRequestData {
        quote_response: quote,
        user_public_key: user.to_string(),
    };

    let response = client
        .post(&format!("{}/swap-instructions", JUP_API_ENDPOINT))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn get_fund_source_token_account(
    rpc_client: &Arc<RpcClient>,
    fund_manager_pubkey: Pubkey,
    from_mint: Pubkey,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let fund = get_fund(rpc_client, fund_manager_pubkey).await?.0;
    let token_account = fund_token_account_pda(&fund, &from_mint).0;
    Ok(token_account)
}

// returns (fund_destination_token_account, need_to_create_fund_token_account)
async fn get_fund_destination_token_account(
    rpc_client: &Arc<RpcClient>,
    fund_manager_pubkey: Pubkey,
    to_mint: Pubkey,
) -> Result<(Pubkey, bool), Box<dyn std::error::Error>> {
    let (fund, fund_data) = get_fund(rpc_client, fund_manager_pubkey).await?;
    Ok((
        fund_token_account_pda(&fund, &to_mint).0,
        if to_mint == fund_data.main_mint {
            false
        } else {
            !fund_data.other_mints.contains(&to_mint)
        },
    ))
}

async fn get_swap_ixns(
    rpc_client: &Arc<RpcClient>,
    fund_manager_pubkey: Pubkey,
    jupiter_route_cpi_data: &[u8],
    jupiter_accounts_metas: Vec<AccountMeta>,
    from_mint: Pubkey,
    to_mint: Pubkey,
    in_amount: u64,
) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
    let fund = fund_pda(&fund_manager_pubkey).0;

    let (fund_destination_token_account, need_to_create_fund_token_account) =
        get_fund_destination_token_account(rpc_client, fund_manager_pubkey, to_mint).await?;

    let mut instructions = vec![];
    if need_to_create_fund_token_account {
        instructions.push(create_fund_token_account(
            fund,
            fund_manager_pubkey,
            fund_destination_token_account,
            to_mint,
        ));
    }

    instructions.push(swap(
        fund_manager_pubkey,
        fund,
        get_fund_source_token_account(rpc_client, fund_manager_pubkey, from_mint).await?,
        fund_destination_token_account,
        Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
        jupiter_accounts_metas,
        in_amount,
        jupiter_route_cpi_data,
    ));

    Ok(instructions)
}

fn get_create_fund_manager_atas_ixns(
    fund_manager: Pubkey,
    from_mint: Pubkey,
    to_mint: Pubkey,
) -> Vec<Instruction> {
    vec![
        create_associated_token_account_idempotent(
            &fund_manager,
            &fund_manager,
            &from_mint,
            &spl_token::ID,
        ),
        create_associated_token_account_idempotent(
            &fund_manager,
            &fund_manager,
            &to_mint,
            &spl_token::ID,
        ),
    ]
}

async fn get_fund(
    rpc_client: &Arc<RpcClient>,
    fund_manager_pubkey: Pubkey,
) -> Result<(Pubkey, Fund), Box<dyn std::error::Error>> {
    let fund = fund_pda(&fund_manager_pubkey).0;
    let fund_data = rpc_client.get_account_data(&fund).await?;
    Ok((fund, *Fund::try_from_bytes(&fund_data)?))
}

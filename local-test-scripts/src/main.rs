use base64;
use serde::Serialize;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::{
    message::v0::Message as MessageV0, program_pack::Pack, pubkey::Pubkey, signature::Signer,
    signer::keypair::read_keypair_file, transaction::VersionedTransaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::state::Account as SplTokenAccount;
use std::str::FromStr;
use steel::*;
use yoko_program_api::state::ArraySet;
use yoko_program_api::{
    sdk::{
        claim_payout, create_fund, create_fund_token_account, create_payout, create_position,
        deposit, swap,
    },
    state::{
        fund_pda, fund_token_account_pda, payout_pda, payout_token_account_pda, position_pda, Fund,
        Payout, Position,
    },
};

struct YokoConfig {
    client: RpcClient,
    fund_manager: solana_sdk::signer::keypair::Keypair,
    depositor: solana_sdk::signer::keypair::Keypair,
    mint: Pubkey,
}

impl YokoConfig {
    fn new() -> Self {
        let client = RpcClient::new("");
        let fund_manager = read_keypair_file("/Users/noyan/.config/solana/id.json")
            .expect("Failed to load fund manager keypair");
        let depositor = read_keypair_file(
            "/Users/noyan/Documents/side_projects/yoko/yoko_program/local-test-scripts/depositor.json",
        ).expect("Failed to load depositor keypair");
        let mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .expect("Failed to parse mint address");

        Self {
            client,
            fund_manager,
            depositor,
            mint,
        }
    }

    fn get_fund_manager_token_account(&self) -> Pubkey {
        get_associated_token_address(&self.fund_manager.pubkey(), &self.mint)
    }

    fn get_depositor_token_account(&self) -> Pubkey {
        get_associated_token_address(&self.depositor.pubkey(), &self.mint)
    }

    fn get_fund(&self) -> Result<(Pubkey, Fund), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let fund_data = self.client.get_account_data(&fund)?;
        Ok((fund, *Fund::try_from_bytes(&fund_data)?))
    }

    fn get_fund_main_token_account_data(
        &self,
    ) -> Result<(Pubkey, SplTokenAccount), Box<dyn std::error::Error>> {
        let (fund, fund_data) = self.get_fund()?;
        let token_account = fund_token_account_pda(&fund, &fund_data.main_mint).0;
        let token_account_data = self.client.get_account_data(&token_account)?;
        let token_account_data = SplTokenAccount::unpack(&token_account_data)?;
        Ok((token_account, token_account_data))
    }

    fn get_fund_token_account_data(
        &self,
        mint: Pubkey,
    ) -> Result<(Pubkey, SplTokenAccount), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let token_account = fund_token_account_pda(&fund, &mint).0;
        let token_account_data = self.client.get_account_data(&token_account)?;
        let token_account_data = SplTokenAccount::unpack(&token_account_data)?;
        Ok((token_account, token_account_data))
    }

    fn get_position(&self) -> Result<(Pubkey, Position), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let position = position_pda(&fund, &self.depositor.pubkey()).0;
        let position_data = self.client.get_account_data(&position)?;
        Ok((position, *Position::try_from_bytes(&position_data)?))
    }

    fn get_payout(&self, counter: u64) -> Result<Payout, Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let payout = payout_pda(&fund, counter).0;
        let payout_data = self.client.get_account_data(&payout)?;
        Ok(*Payout::try_from_bytes(&payout_data)?)
    }

    fn send_transaction(
        &self,
        ixns: Vec<solana_sdk::instruction::Instruction>,
        signer: &solana_sdk::signer::keypair::Keypair,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let mut instructions = vec![];
        instructions.extend([
            ComputeBudgetInstruction::set_compute_unit_limit(500_000),
            ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
        ]);
        instructions.extend(ixns);

        let message =
            MessageV0::try_compile(&signer.pubkey(), &instructions, &[], recent_blockhash)?;

        let transaction = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[signer],
        )?;

        println!("Simulating transaction...");
        let simulation = self.client.simulate_transaction(&transaction)?;
        println!("Simulation result: {:?}\n", simulation);

        println!("Sending transaction...");
        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        println!("Transaction successful! Signature: {}", signature);
        Ok(())
    }

    fn create_fund(&self) -> Result<(), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let main_token_account = fund_token_account_pda(&fund, &self.mint).0;
        let instruction = create_fund(
            fund,
            self.fund_manager.pubkey(),
            self.mint,
            main_token_account,
            10,
        );
        self.send_transaction(vec![instruction], &self.fund_manager)
    }

    fn create_position(&self) -> Result<(), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let position = position_pda(&fund, &self.depositor.pubkey()).0;
        let instruction = create_position(position, fund, self.depositor.pubkey());
        self.send_transaction(vec![instruction], &self.depositor)
    }

    fn deposit(&self, amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let position = position_pda(&fund, &self.depositor.pubkey()).0;
        let main_token_account = fund_token_account_pda(&fund, &self.mint).0;

        let instruction = deposit(
            position,
            fund,
            main_token_account,
            self.depositor.pubkey(),
            self.get_depositor_token_account(),
            amount,
        );
        self.send_transaction(vec![instruction], &self.depositor)
    }

    fn create_payout(&self, amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let fund_main_token_account = fund_token_account_pda(&fund, &self.mint).0;
        let fund_data = self.get_fund()?.1;
        let payout = payout_pda(&fund, fund_data.payouts_counter + 1).0;
        let payout_main_token_account = payout_token_account_pda(&payout).0;
        let protocol_fee_token_account = get_associated_token_address(
            &Pubkey::from_str("H61JjSDPCwvAs1k2vaPAX6d917Pu4dPWykcexvXXzGph").unwrap(),
            &fund_data.main_mint,
        );

        let ixns = vec![
            create_associated_token_account_idempotent(
                &self.fund_manager.pubkey(),
                &self.fund_manager.pubkey(),
                &self.mint,
                &spl_token::ID,
            ),
            create_payout(
                self.fund_manager.pubkey(),
                self.get_fund_manager_token_account(),
                fund,
                fund_main_token_account,
                payout,
                payout_main_token_account,
                self.mint,
                protocol_fee_token_account,
                amount,
            ),
        ];
        self.send_transaction(ixns, &self.fund_manager)
    }

    fn claim_payout(&self) -> Result<(), Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;
        let position = position_pda(&fund, &self.depositor.pubkey()).0;
        let position_data = self.get_position()?.1;
        let payout = payout_pda(&fund, position_data.payouts_counter + 1).0;
        let payout_main_token_account = payout_token_account_pda(&payout).0;

        let ixns = vec![
            create_associated_token_account_idempotent(
                &self.depositor.pubkey(),
                &self.depositor.pubkey(),
                &self.mint,
                &spl_token::ID,
            ),
            claim_payout(
                position,
                self.depositor.pubkey(),
                payout,
                payout_main_token_account,
                self.get_depositor_token_account(),
                fund,
            ),
        ];
        self.send_transaction(ixns, &self.depositor)
    }

    fn get_create_fund_token_account_ixn(
        &self,
        fund: Pubkey,
        fund_token_account: Pubkey,
        mint: Pubkey,
    ) -> Instruction {
        create_fund_token_account(fund, self.fund_manager.pubkey(), fund_token_account, mint)
    }

    fn get_fund_source_token_account(
        &self,
        from_mint: Pubkey,
    ) -> Result<Pubkey, Box<dyn std::error::Error>> {
        let fund = self.get_fund()?.0;
        let token_account = fund_token_account_pda(&fund, &from_mint).0;
        Ok(token_account)
    }

    // returns (fund_destination_token_account, need_to_create_fund_token_account)
    fn get_fund_destination_token_account(
        &self,
        to_mint: Pubkey,
    ) -> Result<(Pubkey, bool), Box<dyn std::error::Error>> {
        let (fund, fund_data) = self.get_fund()?;
        Ok((
            fund_token_account_pda(&fund, &to_mint).0,
            if to_mint == self.mint {
                false
            } else {
                !fund_data.other_mints.contains(&to_mint)
            },
        ))
    }

    fn get_swap_ixns(
        &self,
        jupiter_route_cpi_data: &[u8],
        jupiter_accounts_metas: Vec<AccountMeta>,
        from_mint: Pubkey,
        to_mint: Pubkey,
        in_amount: u64,
    ) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
        let fund = fund_pda(&self.fund_manager.pubkey()).0;

        let (fund_destination_token_account, need_to_create_fund_token_account) =
            self.get_fund_destination_token_account(to_mint)?;

        let mut instructions = vec![];
        if need_to_create_fund_token_account {
            instructions.push(self.get_create_fund_token_account_ixn(
                fund,
                fund_destination_token_account,
                to_mint,
            ));
        }

        instructions.push(swap(
            self.fund_manager.pubkey(),
            fund,
            self.get_fund_source_token_account(from_mint)?,
            fund_destination_token_account,
            Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
            jupiter_accounts_metas,
            in_amount,
            jupiter_route_cpi_data,
        ));

        Ok(instructions)
    }

    fn get_create_fund_manager_atas_ixns(
        &self,
        from_mint: Pubkey,
        to_mint: Pubkey,
    ) -> Vec<Instruction> {
        vec![
            create_associated_token_account_idempotent(
                &self.fund_manager.pubkey(),
                &self.fund_manager.pubkey(),
                &from_mint,
                &spl_token::ID,
            ),
            create_associated_token_account_idempotent(
                &self.fund_manager.pubkey(),
                &self.fund_manager.pubkey(),
                &to_mint,
                &spl_token::ID,
            ),
        ]
    }
}

const JUP_API_ENDPOINT: &str = "https://quote-api.jup.ag/v6";

async fn get_jup_quote(
    from_mint: &str,
    to_mint: &str,
    amount: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}/quote?outputMint={}&inputMint={}&amount={}&slippage=0.5&onlyDirectRoutes=true",
        JUP_API_ENDPOINT,
        to_mint.to_string(),
        from_mint.to_string(),
        amount
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

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

async fn do_swap(config: &YokoConfig) {
    let from_mint = "5mbK36SZ7J19An8jFochhQS4of8g6BwUjbeCSxBSoWdp";
    let to_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let amount = 2_000_000;

    let quote = get_jup_quote(from_mint, to_mint, amount).await.unwrap();

    let swap_ixn = get_jup_swap_ixn(config.fund_manager.pubkey().to_string().as_str(), quote)
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

    let address_lookup_table_accounts: Vec<solana_sdk::address_lookup_table_account::AddressLookupTableAccount> = address_lookup_table_addresses
        .iter()
        .map(|address| {
            config
                .client
                .get_account_with_commitment(address, config.client.commitment())
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
        })
        .collect();

    let mut instructions = vec![];

    instructions.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(500_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ]);

    instructions.extend(config.get_create_fund_manager_atas_ixns(
        Pubkey::from_str(from_mint).unwrap(),
        Pubkey::from_str(to_mint).unwrap(),
    ));

    instructions.extend(
        config
            .get_swap_ixns(
                &jupiter_data,
                jupiter_accounts,
                Pubkey::from_str(from_mint).unwrap(),
                Pubkey::from_str(to_mint).unwrap(),
                amount,
            )
            .unwrap(),
    );

    let recent_blockhash = config.client.get_latest_blockhash().unwrap();

    let message = MessageV0::try_compile(
        &config.fund_manager.pubkey(),
        &instructions,
        &address_lookup_table_accounts,
        recent_blockhash,
    )
    .unwrap();

    let transaction = VersionedTransaction::try_new(
        solana_sdk::message::VersionedMessage::V0(message),
        &[&config.fund_manager],
    )
    .unwrap();

    println!("Simulating transaction...");
    let simulation = config.client.simulate_transaction(&transaction).unwrap();
    println!("Simulation result: {:?}\n", simulation);

    println!("Sending transaction...");
    let signature = config
        .client
        .send_and_confirm_transaction(&transaction)
        .unwrap();
    println!("Transaction successful! Signature: {}", signature);
}

pub fn fetch_program_accounts(
    program_id: &Pubkey,
    connection: &RpcClient,
) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(
            std::mem::size_of::<Position>() as u64 + 8,
        )]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: 0,  // Request 0 bytes of data
            }),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };

    let accounts = connection.get_program_accounts_with_config(program_id, config)?;
    Ok(accounts.into_iter().map(|(pubkey, _)| pubkey).collect())
}

#[tokio::main]
async fn main() {
    let config = YokoConfig::new();

    let program_id = Pubkey::from_str("4NmD5nA9Rd8SCgW6kXyG1zzUGkfDg3TUiZTmPEMM3ZLU").unwrap();
    let accounts = fetch_program_accounts(&program_id, &config.client).unwrap();
    println!("accounts len: {:?}", accounts.len());

    // let (fund, fund_data) = config.get_fund().unwrap();
    // println!("fund: {:?}", fund);
    // println!("fund_data: {:?}", fund_data);

    // for mint in fund_data.other_mints.iter() {
    //     println!("mint: {:?}", mint);
    //     let (token_account, token_account_data) =
    //         config.get_fund_token_account_data(*mint).unwrap();
    //     println!("token_account: {:?}", token_account);
    //     println!("token_account_data: {:?}", token_account_data);
    // }

    // let fund_main_token_account_data = config.get_fund_main_token_account_data().unwrap();
    // println!(
    //     "fund_main_token_account_data: {:?}",
    //     fund_main_token_account_data
    // );

    // let (position, position_data) = config.get_position().unwrap();
    // println!("position: {:?}", position);
    // println!("position_data: {:?}", position_data);

    // do_swap(&config).await;

    // config.claim_payout().unwrap();
}

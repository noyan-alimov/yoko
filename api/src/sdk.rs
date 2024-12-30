use steel::*;

use crate::prelude::*;

pub fn create_fund(
    fund: Pubkey,
    authority: Pubkey,
    main_mint: Pubkey,
    main_token_account: Pubkey,
    authority_fee: u64,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(fund, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(main_mint, false),
            AccountMeta::new(main_token_account, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data: CreateFund {
            authority_fee: authority_fee.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn create_position(position: Pubkey, fund: Pubkey, authority: Pubkey) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(position, false),
            AccountMeta::new_readonly(fund, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data: CreatePosition {}.to_bytes(),
    }
}

pub fn deposit(
    position: Pubkey,
    fund: Pubkey,
    fund_main_token_account: Pubkey,
    depositor_authority: Pubkey,
    depositor_token_account: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(position, false),
            AccountMeta::new(fund, false),
            AccountMeta::new(fund_main_token_account, false),
            AccountMeta::new(depositor_authority, true),
            AccountMeta::new(depositor_token_account, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Deposit {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn create_payout(
    fund_authority: Pubkey,
    fund_authority_token_account: Pubkey,
    fund: Pubkey,
    fund_main_token_account: Pubkey,
    payout: Pubkey,
    payout_main_token_account: Pubkey,
    main_mint: Pubkey,
    protocol_fee_token_account: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(fund_authority, true),
            AccountMeta::new(fund_authority_token_account, false),
            AccountMeta::new(fund, false),
            AccountMeta::new(fund_main_token_account, false),
            AccountMeta::new(payout, false),
            AccountMeta::new(payout_main_token_account, false),
            AccountMeta::new_readonly(main_mint, false),
            AccountMeta::new(protocol_fee_token_account, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data: CreatePayout {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn claim_payout(
    position: Pubkey,
    position_authority: Pubkey,
    payout: Pubkey,
    payout_main_token_account: Pubkey,
    depositor_main_token_account: Pubkey,
    fund: Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(position, false),
            AccountMeta::new(position_authority, true),
            AccountMeta::new(payout, false),
            AccountMeta::new(payout_main_token_account, false),
            AccountMeta::new(depositor_main_token_account, false),
            AccountMeta::new_readonly(fund, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: ClaimPayout {}.to_bytes(),
    }
}

pub fn swap(
    fund_authority: Pubkey,
    fund: Pubkey,
    fund_source_token_account: Pubkey,
    fund_destination_token_account: Pubkey,
    jupiter_program: Pubkey,
    jupiter_accounts_metas: Vec<AccountMeta>,
    in_amount: u64,
    jupiter_route_cpi_data: &[u8],
) -> Instruction {
    let mut data = vec![5]; // instruction discriminator
    data.extend_from_slice(&in_amount.to_le_bytes());
    data.extend_from_slice(jupiter_route_cpi_data);

    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(fund_authority, true),
            AccountMeta::new(fund, false),
            AccountMeta::new(fund_destination_token_account, false),
            AccountMeta::new(fund_source_token_account, false),
            AccountMeta::new_readonly(jupiter_program, false),
        ]
        .into_iter()
        .chain(jupiter_accounts_metas)
        .collect(),
        data,
    }
}

pub fn create_fund_token_account(
    fund: Pubkey,
    fund_authority: Pubkey,
    fund_token_account: Pubkey,
    mint: Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(fund, false),
            AccountMeta::new(fund_authority, true),
            AccountMeta::new(fund_token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data: CreateFundTokenAccount {}.to_bytes(),
    }
}

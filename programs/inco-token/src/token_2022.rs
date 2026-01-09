use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use inco_lightning::cpi::accounts::{Operation, Allow};
use inco_lightning::cpi::{e_add, e_ge, e_select, e_sub, new_euint128, as_euint128, allow};
use inco_lightning::types::Euint128;
use inco_lightning::ID as INCO_LIGHTNING_ID;
pub use crate::{AccountState, COption, CustomError, IncoMint, IncoAccount};

pub const TOKEN_2022_ID: Pubkey = anchor_lang::solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

// ========== HELPER FUNCTION ==========

/// Helper to call allow with accounts from remaining_accounts
fn call_allow_from_remaining<'info>(
    inco_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    handle: Euint128,
    allowed_pubkey: Pubkey,
    account_offset: usize,
) -> Result<()> {
    if remaining_accounts.len() < account_offset + 2 {
        return Err(CustomError::InvalidInstruction.into());
    }
    
    let allowance_account = &remaining_accounts[account_offset];
    let allowed_address = &remaining_accounts[account_offset + 1];
    
    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Allow {
            allowance_account: allowance_account.clone(),
            signer: signer.clone(),
            allowed_address: allowed_address.clone(),
            system_program: system_program.clone(),
        }
    );
    
    allow(cpi_ctx, handle.0, true, allowed_pubkey)?;
    Ok(())
}

// ========== TOKEN 2022 CHECKED FUNCTIONS ==========

/// Transfer checked - validates decimals match mint
/// remaining_accounts:
///   [0] source_allowance_account (mut)
///   [1] source_owner_address (readonly)
///   [2] dest_allowance_account (mut)
///   [3] dest_owner_address (readonly)
pub fn transfer_checked<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferChecked<'info>>,
    ciphertext: Vec<u8>,
    input_type: u8,
    decimals: u8,
) -> Result<()> {
    let source = &mut ctx.accounts.source;
    let destination = &mut ctx.accounts.destination;
    let mint = &ctx.accounts.mint;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(destination.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(destination.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(source.mint == mint.key(), CustomError::MintMismatch);
    require!(destination.mint == mint.key(), CustomError::MintMismatch);
    require!(mint.decimals == decimals, CustomError::MintDecimalsMismatch);

    if source.key() == destination.key() {
        return Ok(());
    }

    let authority_key = ctx.accounts.authority.key();
    if source.owner != authority_key {
        match source.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => return Err(CustomError::OwnerMismatch.into()),
        }
    }

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx2, source.amount, amount, 0u8)?;

    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx3, 0)?;

    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let transfer_amount = e_select(cpi_ctx4, has_sufficient, amount, zero_value, 0u8)?;

    let cpi_ctx5 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_source_balance = e_sub(cpi_ctx5, source.amount, transfer_amount, 0u8)?;
    source.amount = new_source_balance;

    let cpi_ctx6 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_dest_balance = e_add(cpi_ctx6, destination.amount, transfer_amount, 0u8)?;
    destination.amount = new_dest_balance;

    if ctx.remaining_accounts.len() >= 2 {
        call_allow_from_remaining(
            &inco, &signer,
            &ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts,
            new_source_balance, source.owner, 0,
        )?;
    }

    if ctx.remaining_accounts.len() >= 4 {
        call_allow_from_remaining(
            &inco, &signer,
            &ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts,
            new_dest_balance, destination.owner, 2,
        )?;
    }

    Ok(())
}

/// Mint to checked - validates decimals match mint
/// remaining_accounts:
///   [0] allowance_account (mut)
///   [1] owner_address (readonly)
pub fn mint_to_checked<'info>(
    ctx: Context<'_, '_, '_, 'info, MintToChecked<'info>>,
    ciphertext: Vec<u8>,
    input_type: u8,
    decimals: u8,
) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    let account = &mut ctx.accounts.account;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);
    require!(mint.decimals == decimals, CustomError::MintDecimalsMismatch);

    let mint_authority = match mint.mint_authority {
        COption::Some(authority) => authority,
        COption::None => return Err(CustomError::FixedSupply.into()),
    };
    require!(mint_authority == ctx.accounts.authority.key(), CustomError::OwnerMismatch);

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_supply = e_add(cpi_ctx2, mint.supply, amount, 0u8)?;
    mint.supply = new_supply;

    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_balance = e_add(cpi_ctx3, account.amount, amount, 0u8)?;
    account.amount = new_balance;

    if ctx.remaining_accounts.len() >= 2 {
        call_allow_from_remaining(
            &inco, &signer,
            &ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts,
            new_balance, account.owner, 0,
        )?;
    }

    Ok(())
}

/// Burn checked - validates decimals match mint
/// remaining_accounts:
///   [0] allowance_account (mut)
///   [1] owner_address (readonly)
pub fn burn_checked<'info>(
    ctx: Context<'_, '_, '_, 'info, BurnChecked<'info>>,
    ciphertext: Vec<u8>,
    input_type: u8,
    decimals: u8,
) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &mut ctx.accounts.mint;

    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);
    require!(mint.decimals == decimals, CustomError::MintDecimalsMismatch);

    let authority_key = ctx.accounts.authority.key();
    if account.owner != authority_key {
        match account.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => return Err(CustomError::OwnerMismatch.into()),
        }
    }

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx2, account.amount, amount, 0u8)?;

    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx3, 0)?;

    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let burn_amount = e_select(cpi_ctx4, has_sufficient, amount, zero_value, 0u8)?;

    let cpi_ctx5 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_balance = e_sub(cpi_ctx5, account.amount, burn_amount, 0u8)?;
    account.amount = new_balance;

    let cpi_ctx6 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_supply = e_sub(cpi_ctx6, mint.supply, burn_amount, 0u8)?;
    mint.supply = new_supply;

    if ctx.remaining_accounts.len() >= 2 {
        call_allow_from_remaining(
            &inco, &signer,
            &ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts,
            new_balance, account.owner, 0,
        )?;
    }

    Ok(())
}

/// Approve checked - validates decimals match mint
/// remaining_accounts:
///   [0] allowance_account (mut)
///   [1] delegate_address (readonly)
pub fn approve_checked<'info>(
    ctx: Context<'_, '_, '_, 'info, ApproveChecked<'info>>,
    ciphertext: Vec<u8>,
    input_type: u8,
    decimals: u8,
) -> Result<()> {
    let source = &mut ctx.accounts.source;
    let mint = &ctx.accounts.mint;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(source.owner == ctx.accounts.owner.key(), CustomError::OwnerMismatch);
    require!(source.mint == mint.key(), CustomError::MintMismatch);
    require!(mint.decimals == decimals, CustomError::MintDecimalsMismatch);

    let inco = ctx.accounts.inco_lightning_program.to_account_info();

    let cpi_ctx = CpiContext::new(
        inco.clone(),
        Operation { signer: ctx.accounts.owner.to_account_info() }
    );
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    source.delegate = COption::Some(ctx.accounts.delegate.key());
    source.delegated_amount = amount;

    if ctx.remaining_accounts.len() >= 2 {
        call_allow_from_remaining(
            &inco,
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts,
            amount, ctx.accounts.delegate.key(), 0,
        )?;
    }

    Ok(())
}

pub fn initialize_account3<'info>(ctx: Context<'_, '_, '_, 'info, InitializeAccount3<'info>>) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &ctx.accounts.mint;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Uninitialized, CustomError::AlreadyInUse);

    account.mint = mint.key();
    account.owner = ctx.accounts.authority.key();

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    account.amount = as_euint128(cpi_ctx, 0)?;
    account.delegate = COption::None;
    account.state = AccountState::Initialized;
    account.is_native = COption::None;

    let cpi_ctx2 = CpiContext::new(inco, Operation { signer });
    account.delegated_amount = as_euint128(cpi_ctx2, 0)?;
    account.close_authority = COption::None;

    Ok(())
}

pub fn revoke_2022<'info>(ctx: Context<'_, '_, '_, 'info, Revoke2022<'info>>) -> Result<()> {
    let source = &mut ctx.accounts.source;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.owner == ctx.accounts.authority.key(), CustomError::OwnerMismatch);

    source.delegate = COption::None;

    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation { signer: ctx.accounts.authority.to_account_info() }
    );
    source.delegated_amount = as_euint128(cpi_ctx, 0)?;

    Ok(())
}

pub fn close_account_2022<'info>(ctx: Context<'_, '_, '_, 'info, CloseAccount2022<'info>>) -> Result<()> {
    let account = &ctx.accounts.account;

    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);

    let authority_key = ctx.accounts.authority.key();
    let is_owner = account.owner == authority_key;
    let is_close_authority = match account.close_authority {
        COption::Some(close_auth) => close_auth == authority_key,
        COption::None => false,
    };
    require!(is_owner || is_close_authority, CustomError::OwnerMismatch);

    let dest_starting_lamports = ctx.accounts.destination.lamports();
    **ctx.accounts.destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(ctx.accounts.account.to_account_info().lamports())
        .ok_or(CustomError::Overflow)?;
    **ctx.accounts.account.to_account_info().lamports.borrow_mut() = 0;

    Ok(())
}

// ========== ACCOUNT CONTEXTS ==========

#[derive(Accounts)]
pub struct TransferChecked<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = source.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub source: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(
        mut,
        constraint = destination.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = destination.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub destination: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintToChecked<'info> {
    #[account(mut, constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut, constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState)]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BurnChecked<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut, constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveChecked<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = source.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub source: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    /// CHECK: Delegate address
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeAccount3<'info> {
    #[account(init_if_needed, payer = authority, space = 8 + IncoAccount::LEN)]
    pub account: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Revoke2022<'info> {
    #[account(mut, constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState)]
    pub source: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount2022<'info> {
    #[account(mut, constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState)]
    pub account: Account<'info, IncoAccount>,
    /// CHECK: Destination for lamports
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Clone)]
pub struct Token2022Confidential;

impl anchor_lang::Id for Token2022Confidential {
    fn id() -> Pubkey {
        TOKEN_2022_ID
    }
}

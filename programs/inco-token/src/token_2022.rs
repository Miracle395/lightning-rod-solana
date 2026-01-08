use anchor_lang::prelude::*;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use inco_lightning::cpi::accounts::Operation;
use inco_lightning::cpi::{e_add, e_ge, e_select, e_sub, new_euint128, as_euint128};
use inco_lightning::types::Euint128;
use inco_lightning::ID as INCO_LIGHTNING_ID;
pub use crate::{AccountState, COption, CustomError, IncoMint, IncoAccount};

pub const TOKEN_2022_ID: Pubkey = anchor_lang::solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

// ========== TOKEN 2022 FUNCTIONS ==========

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

    // Early return if source and destination are the same
    if source.key() == destination.key() {
        return Ok(());
    }

    // Check ownership/delegation
    let authority_key = ctx.accounts.authority.key();
    if source.owner != authority_key {
        match source.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => return Err(CustomError::OwnerMismatch.into()),
        }
    }

    // Create encrypted amount
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.authority.to_account_info(),
        }
    );
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    // Use internal helper function
    transfer_internal(
        source,
        destination,
        amount,
        &ctx.accounts.authority,
        &ctx.accounts.inco_lightning_program,
    )
}

pub fn transfer_checked_with_handle<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferChecked<'info>>,
    amount_handle: Euint128,
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

    // Early return if source and destination are the same
    if source.key() == destination.key() {
        return Ok(());
    }

    // Check ownership/delegation
    let authority_key = ctx.accounts.authority.key();
    if source.owner != authority_key {
        match source.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => return Err(CustomError::OwnerMismatch.into()),
        }
    }

    transfer_internal(
        source,
        destination,
        amount_handle,
        &ctx.accounts.authority,
        &ctx.accounts.inco_lightning_program,
    )
}

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

    // Check mint authority
    let mint_authority = match mint.mint_authority {
        COption::Some(authority) => authority,
        COption::None => return Err(CustomError::FixedSupply.into()),
    };
    require!(mint_authority == ctx.accounts.authority.key(), CustomError::OwnerMismatch);

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    // Create encrypted amount from ciphertext
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    // Add to supply
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_supply = e_add(cpi_ctx2, mint.supply, amount, 0u8)?;
    mint.supply = new_supply;

    // Add to account balance
    let cpi_ctx3 = CpiContext::new(inco, Operation { signer });
    let new_balance = e_add(cpi_ctx3, account.amount, amount, 0u8)?;
    account.amount = new_balance;

    Ok(())
}

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

    // Check authority
    let authority_key = ctx.accounts.authority.key();
    if account.owner != authority_key {
        match account.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => return Err(CustomError::OwnerMismatch.into()),
        }
    }

    // Create encrypted burn amount
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.authority.to_account_info(),
        }
    );
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    burn_internal(
        account,
        mint,
        amount,
        &ctx.accounts.authority,
        &ctx.accounts.inco_lightning_program,
    )
}

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

    // Create encrypted delegated amount
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.owner.to_account_info(),
        }
    );
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    source.delegate = COption::Some(ctx.accounts.delegate.key());
    source.delegated_amount = amount;

    Ok(())
}

pub fn initialize_account3<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAccount3<'info>>,
) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &ctx.accounts.mint;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Uninitialized, CustomError::AlreadyInUse);

    account.mint = mint.key();
    account.owner = ctx.accounts.authority.key();

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    // Create encrypted zero handle for amount
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_amount = as_euint128(cpi_ctx, 0)?;

    account.amount = zero_amount;
    account.delegate = COption::None;
    account.state = AccountState::Initialized;
    account.is_native = COption::None;

    // Create encrypted zero handle for delegated_amount
    let cpi_ctx2 = CpiContext::new(inco, Operation { signer });
    let zero_delegated = as_euint128(cpi_ctx2, 0)?;

    account.delegated_amount = zero_delegated;
    account.close_authority = COption::None;

    Ok(())
}

pub fn revoke_2022<'info>(
    ctx: Context<'_, '_, '_, 'info, Revoke2022<'info>>
) -> Result<()> {
    let source = &mut ctx.accounts.source;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.owner == ctx.accounts.authority.key(), CustomError::OwnerMismatch);

    source.delegate = COption::None;

    // Create encrypted zero handle for delegated_amount
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.authority.to_account_info(),
        }
    );
    let zero_delegated = as_euint128(cpi_ctx, 0)?;

    source.delegated_amount = zero_delegated;

    Ok(())
}

pub fn close_account_2022<'info>(
    ctx: Context<'_, '_, '_, 'info, CloseAccount2022<'info>>
) -> Result<()> {
    let account = &ctx.accounts.account;

    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);

    // Check authority (owner or close_authority)
    let authority_key = ctx.accounts.authority.key();
    let is_owner = account.owner == authority_key;
    let is_close_authority = match account.close_authority {
        COption::Some(close_auth) => close_auth == authority_key,
        COption::None => false,
    };

    require!(is_owner || is_close_authority, CustomError::OwnerMismatch);

    msg!("WARNING: Close account should be called with encrypted balance verification(client side)");

    // Transfer remaining lamports to destination
    let dest_starting_lamports = ctx.accounts.destination.lamports();
    **ctx.accounts.destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(ctx.accounts.account.to_account_info().lamports())
        .ok_or(CustomError::Overflow)?;
    **ctx.accounts.account.to_account_info().lamports.borrow_mut() = 0;

    Ok(())
}

// ========== INTERNAL HELPER FUNCTIONS ==========

fn transfer_internal<'info>(
    source: &mut Account<'info, IncoAccount>,
    destination: &mut Account<'info, IncoAccount>,
    amount: Euint128,
    authority: &Signer<'info>,
    inco_lightning_program: &AccountInfo<'info>,
) -> Result<()> {
    let inco = inco_lightning_program.to_account_info();
    let signer = authority.to_account_info();

    // Check sufficient balance
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx, source.amount, amount, 0u8)?;

    // Create zero handle
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx2, 0)?;

    // Select transfer amount based on sufficient balance
    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let transfer_amount = e_select(cpi_ctx3, has_sufficient, amount, zero_value, 0u8)?;

    // Subtract from source
    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_source_balance = e_sub(cpi_ctx4, source.amount, transfer_amount, 0u8)?;
    source.amount = new_source_balance;

    // Add to destination
    let cpi_ctx5 = CpiContext::new(inco, Operation { signer });
    let new_dest_balance = e_add(cpi_ctx5, destination.amount, transfer_amount, 0u8)?;
    destination.amount = new_dest_balance;

    Ok(())
}

fn burn_internal<'info>(
    account: &mut Account<'info, IncoAccount>,
    mint: &mut Account<'info, IncoMint>,
    amount: Euint128,
    authority: &Signer<'info>,
    inco_lightning_program: &AccountInfo<'info>,
) -> Result<()> {
    let inco = inco_lightning_program.to_account_info();
    let signer = authority.to_account_info();

    // Check sufficient balance and perform conditional burn
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx, account.amount, amount, 0u8)?;

    // Create zero handle
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx2, 0)?;

    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let burn_amount = e_select(cpi_ctx3, has_sufficient, amount, zero_value, 0u8)?;

    // Subtract from account
    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_balance = e_sub(cpi_ctx4, account.amount, burn_amount, 0u8)?;
    account.amount = new_balance;

    // Subtract from total supply
    let cpi_ctx5 = CpiContext::new(inco, Operation { signer });
    let new_supply = e_sub(cpi_ctx5, mint.supply, burn_amount, 0u8)?;
    mint.supply = new_supply;

    Ok(())
}

// ========== ACCOUNT CONTEXTS FOR TOKEN 2022 ==========

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
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintToChecked<'info> {
    #[account(
        mut,
        constraint = mint.is_initialized @ CustomError::UninitializedState,
    )]
    pub mint: Account<'info, IncoMint>,
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnChecked<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(
        mut,
        constraint = mint.is_initialized @ CustomError::UninitializedState,
    )]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
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
    /// CHECK: This is just stored as a delegate
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
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
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Revoke2022<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
    )]
    pub source: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount2022<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    /// CHECK: This is the destination account that will receive the lamports from the closed account
    pub destination: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

// ========== TOKEN 2022 ID AND PROGRAM ==========

#[derive(Clone)]
pub struct Token2022Confidential;

impl anchor_lang::Id for Token2022Confidential {
    fn id() -> Pubkey {
        TOKEN_2022_ID
    }
}

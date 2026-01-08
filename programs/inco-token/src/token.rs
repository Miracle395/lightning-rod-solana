use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};
use inco_lightning::cpi::accounts::Operation;
use inco_lightning::cpi::{e_add, e_ge, e_select, e_sub, new_euint128, as_euint128};
use inco_lightning::types::Euint128;
use inco_lightning::ID as INCO_LIGHTNING_ID;
pub use crate::{AccountState, COption, CustomError, IncoMint, IncoAccount};


// ========== TOKEN INSTRUCTIONS ==========

pub fn initialize_mint(
    ctx: Context<InitializeMint>,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>
) -> Result<()> {
    let mint = &mut ctx.accounts.mint;

    require!(!mint.is_initialized, CustomError::AlreadyInUse);

    mint.mint_authority = COption::Some(mint_authority);

    // Create encrypted zero handle for supply
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.payer.to_account_info(),
        }
    );
    let zero_supply = as_euint128(cpi_ctx, 0)?;

    mint.supply = zero_supply;
    mint.decimals = decimals;
    mint.is_initialized = true;
    mint.freeze_authority = match freeze_authority {
        Some(authority) => COption::Some(authority),
        None => COption::None,
    };

    Ok(())
}

pub fn initialize_account(ctx: Context<InitializeAccount>) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &ctx.accounts.mint;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Uninitialized, CustomError::AlreadyInUse);

    account.mint = mint.key();
    account.owner = ctx.accounts.owner.key();

    // Create encrypted zero handle for amount
    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.payer.to_account_info();

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

pub fn mint_to(ctx: Context<IncoMintTo>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    let account = &mut ctx.accounts.account;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);

    // Check mint authority
    let mint_authority = match mint.mint_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::FixedSupply.into());
        }
    };
    require!(mint_authority == ctx.accounts.mint_authority.key(), CustomError::OwnerMismatch);

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.mint_authority.to_account_info();

    // Create encrypted amount from ciphertext
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    // Add to supply with overflow protection
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_supply = e_add(cpi_ctx2, mint.supply, amount, 0u8)?;
    mint.supply = new_supply;

    // Add to account balance
    let cpi_ctx3 = CpiContext::new(inco, Operation { signer });
    let new_balance = e_add(cpi_ctx3, account.amount, amount, 0u8)?;
    account.amount = new_balance;

    Ok(())
}

pub fn mint_to_with_handle(ctx: Context<IncoMintTo>, amount_handle: Euint128) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    let account = &mut ctx.accounts.account;

    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);

    // Check mint authority
    let mint_authority = match mint.mint_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::FixedSupply.into());
        }
    };
    require!(mint_authority == ctx.accounts.mint_authority.key(), CustomError::OwnerMismatch);

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.mint_authority.to_account_info();

    // Add to supply
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_supply = e_add(cpi_ctx, mint.supply, amount_handle, 0u8)?;
    mint.supply = new_supply;

    // Add to account balance
    let cpi_ctx2 = CpiContext::new(inco, Operation { signer });
    let new_balance = e_add(cpi_ctx2, account.amount, amount_handle, 0u8)?;
    account.amount = new_balance;

    Ok(())
}

pub fn transfer(ctx: Context<IncoTransfer>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
    let source = &mut ctx.accounts.source;
    let destination = &mut ctx.accounts.destination;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(destination.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(destination.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(source.mint == destination.mint, CustomError::MintMismatch);

    // early return if source and destination are the same
    if source.key() == destination.key() {
        return Ok(());
    }

    // Check ownership/delegation
    let authority_key = ctx.accounts.authority.key();
    if source.owner != authority_key {
        // Check if it's a valid delegate
        match source.delegate {
            COption::Some(delegate) if delegate == authority_key => {
                // Valid delegate transfer - check delegated amount
            }
            _ => {
                return Err(CustomError::OwnerMismatch.into());
            }
        }
    }

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    // Create encrypted amount
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    // Check sufficient balance
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx2, source.amount, amount, 0u8)?;

    // Create zero handle for conditional logic
    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx3, 0)?;

    // Select transfer amount based on sufficient balance
    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let transfer_amount = e_select(
        cpi_ctx4,
        has_sufficient,
        amount,
        zero_value,
        0u8
    )?;

    // Subtract from source
    let cpi_ctx5 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_source_balance = e_sub(
        cpi_ctx5,
        source.amount,
        transfer_amount, 
        0u8
    )?;
    source.amount = new_source_balance;

    // Add to destination
    let cpi_ctx6 = CpiContext::new(inco, Operation { signer });
    let new_dest_balance = e_add(
        cpi_ctx6,
        destination.amount,
        transfer_amount, 
        0u8
    )?;
    destination.amount = new_dest_balance;

    Ok(())
}

pub fn transfer_with_handle(ctx: Context<IncoTransfer>, amount_handle: Euint128) -> Result<()> {
    let source = &mut ctx.accounts.source;
    let destination = &mut ctx.accounts.destination;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(destination.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(destination.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(source.mint == destination.mint, CustomError::MintMismatch);

    // early return if source and destination are the same
    if source.key() == destination.key() {
        return Ok(());
    }

    // Check ownership/delegation
    let authority_key = ctx.accounts.authority.key();
    if source.owner != authority_key {
        // Check if it's a valid delegate
        match source.delegate {
            COption::Some(delegate) if delegate == authority_key => {
                // Valid delegate transfer
            }
            _ => {
                return Err(CustomError::OwnerMismatch.into());
            }
        }
    }

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    // Check sufficient balance
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx, source.amount, amount_handle, 0u8)?;

    // Create zero handle for conditional logic
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx2, 0)?;

    // Select transfer amount based on sufficient balance
    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let transfer_amount = e_select(cpi_ctx3, has_sufficient, amount_handle, zero_value, 0u8)?;

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

pub fn approve(ctx: Context<IncoApprove>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
    let source = &mut ctx.accounts.source;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(source.owner == ctx.accounts.owner.key(), CustomError::OwnerMismatch);

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

pub fn revoke(ctx: Context<IncoRevoke>) -> Result<()> {
    let source = &mut ctx.accounts.source;

    require!(source.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(source.owner == ctx.accounts.owner.key(), CustomError::OwnerMismatch);

    source.delegate = COption::None;

    // Create encrypted zero handle for delegated_amount
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.owner.to_account_info(),
        }
    );
    let zero_delegated = as_euint128(cpi_ctx, 0)?;

    source.delegated_amount = zero_delegated;

    Ok(())
}

pub fn burn(ctx: Context<IncoBurn>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &mut ctx.accounts.mint;

    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.state != AccountState::Frozen, CustomError::AccountFrozen);
    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);

    // Check authority
    let authority_key = ctx.accounts.authority.key();
    if account.owner != authority_key {
        match account.delegate {
            COption::Some(delegate) if delegate == authority_key => {}
            _ => {
                return Err(CustomError::OwnerMismatch.into());
            }
        }
    }

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.authority.to_account_info();

    // Create encrypted burn amount
    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let amount = new_euint128(cpi_ctx, ciphertext, input_type)?;

    // Check sufficient balance and perform conditional burn
    let cpi_ctx2 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let has_sufficient = e_ge(cpi_ctx2, account.amount, amount, 0u8)?;

    // Create zero handle
    let cpi_ctx3 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_value = as_euint128(cpi_ctx3, 0)?;

    let cpi_ctx4 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let burn_amount = e_select(cpi_ctx4, has_sufficient, amount, zero_value, 0u8)?;

    // Subtract from account
    let cpi_ctx5 = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let new_balance = e_sub(cpi_ctx5, account.amount, burn_amount, 0u8)?;
    account.amount = new_balance;

    // Subtract from total supply
    let cpi_ctx6 = CpiContext::new(inco, Operation { signer });
    let new_supply = e_sub(cpi_ctx6, mint.supply, burn_amount, 0u8)?;
    mint.supply = new_supply;

    Ok(())
}

pub fn freeze_account(ctx: Context<FreezeAccount>) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &ctx.accounts.mint;

    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);

    // Check freeze authority
    let freeze_authority = match mint.freeze_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::MintCannotFreeze.into());
        }
    };
    require!(
        freeze_authority == ctx.accounts.freeze_authority.key(),
        CustomError::OwnerMismatch
    );

    account.state = AccountState::Frozen;

    Ok(())
}

pub fn thaw_account(ctx: Context<ThawAccount>) -> Result<()> {
    let account = &mut ctx.accounts.account;
    let mint = &ctx.accounts.mint;

    require!(account.state == AccountState::Frozen, CustomError::InvalidState);
    require!(mint.is_initialized, CustomError::UninitializedState);
    require!(account.mint == mint.key(), CustomError::MintMismatch);

    // Check freeze authority
    let freeze_authority = match mint.freeze_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::MintCannotFreeze.into());
        }
    };
    require!(
        freeze_authority == ctx.accounts.freeze_authority.key(),
        CustomError::OwnerMismatch
    );

    account.state = AccountState::Initialized;

    Ok(())
}

pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
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



pub fn set_mint_authority(
    ctx: Context<SetMintAuthority>,
    new_authority: Option<Pubkey>
) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    require!(mint.is_initialized, CustomError::UninitializedState);

    // Check current mint authority
    let current_authority = match mint.mint_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::FixedSupply.into());
        }
    };
    require!(
        current_authority == ctx.accounts.current_authority.key(),
        CustomError::OwnerMismatch
    );

    mint.mint_authority = match new_authority {
        Some(authority) => COption::Some(authority),
        None => COption::None,
    };

    Ok(())
}

pub fn set_freeze_authority(
    ctx: Context<SetFreezeAuthority>,
    new_authority: Option<Pubkey>
) -> Result<()> {
    let mint = &mut ctx.accounts.mint;
    require!(mint.is_initialized, CustomError::UninitializedState);

    // Check current freeze authority
    let current_authority = match mint.freeze_authority {
        COption::Some(authority) => authority,
        COption::None => {
            return Err(CustomError::MintCannotFreeze.into());
        }
    };
    require!(
        current_authority == ctx.accounts.current_authority.key(),
        CustomError::OwnerMismatch
    );

    mint.freeze_authority = match new_authority {
        Some(authority) => COption::Some(authority),
        None => COption::None,
    };

    Ok(())
}

pub fn set_account_owner(ctx: Context<SetAccountOwner>, new_owner: Pubkey) -> Result<()> {
    let account = &mut ctx.accounts.account;
    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.owner == ctx.accounts.current_owner.key(), CustomError::OwnerMismatch);

    account.owner = new_owner;

    Ok(())
}

pub fn set_close_authority(
    ctx: Context<SetCloseAuthority>,
    new_authority: Option<Pubkey>
) -> Result<()> {
    let account = &mut ctx.accounts.account;
    require!(account.state == AccountState::Initialized, CustomError::UninitializedState);
    require!(account.owner == ctx.accounts.owner.key(), CustomError::OwnerMismatch);

    account.close_authority = match new_authority {
        Some(authority) => COption::Some(authority),
        None => COption::None,
    };

    Ok(())
}

// ========== ACCOUNT CONTEXTS ==========

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(init, payer = payer, space = 8 + IncoMint::LEN)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeAccount<'info> {
    #[account(init, payer = payer, space = 8 + IncoAccount::LEN)]
    pub account: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    /// CHECK: This is just used for account initialization, validated in instruction
    pub owner: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IncoMintTo<'info> {
    #[account(
        mut,
        constraint = mint.is_initialized @ CustomError::UninitializedState,
    )]
    pub mint: Account<'info, IncoMint>,
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.mint == mint.key() @ CustomError::MintMismatch,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IncoTransfer<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = source.state != AccountState::Frozen @ CustomError::AccountFrozen,
    )]
    pub source: Account<'info, IncoAccount>,
    #[account(
        mut,
        constraint = destination.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = destination.state != AccountState::Frozen @ CustomError::AccountFrozen,
        constraint = destination.mint == source.mint @ CustomError::MintMismatch,
    )]
    pub destination: Account<'info, IncoAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IncoApprove<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = source.state != AccountState::Frozen @ CustomError::AccountFrozen,
        constraint = source.owner == owner.key() @ CustomError::OwnerMismatch,
    )]
    pub source: Account<'info, IncoAccount>,
    /// CHECK: This is just stored as a delegate
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IncoRevoke<'info> {
    #[account(
        mut,
        constraint = source.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = source.owner == owner.key() @ CustomError::OwnerMismatch,
    )]
    pub source: Account<'info, IncoAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IncoBurn<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.state != AccountState::Frozen @ CustomError::AccountFrozen,
        constraint = account.mint == mint.key() @ CustomError::MintMismatch,
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
pub struct FreezeAccount<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.mint == mint.key() @ CustomError::MintMismatch,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub freeze_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ThawAccount<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Frozen @ CustomError::InvalidState,
        constraint = account.mint == mint.key() @ CustomError::MintMismatch,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(constraint = mint.is_initialized @ CustomError::UninitializedState)]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub freeze_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
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


#[derive(Accounts)]
pub struct SetMintAuthority<'info> {
    #[account(
        mut,
        constraint = mint.is_initialized @ CustomError::UninitializedState,
    )]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub current_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetFreezeAuthority<'info> {
    #[account(
        mut,
        constraint = mint.is_initialized @ CustomError::UninitializedState,
    )]
    pub mint: Account<'info, IncoMint>,
    #[account(mut)]
    pub current_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetAccountOwner<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    pub current_owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetCloseAuthority<'info> {
    #[account(
        mut,
        constraint = account.state == AccountState::Initialized @ CustomError::UninitializedState,
        constraint = account.owner == owner.key() @ CustomError::OwnerMismatch,
    )]
    pub account: Account<'info, IncoAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

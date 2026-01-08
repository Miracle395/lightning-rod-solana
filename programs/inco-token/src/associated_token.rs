use anchor_lang::prelude::*;
use inco_lightning::cpi::accounts::Operation;
use inco_lightning::cpi::as_euint128;
use inco_lightning::ID as INCO_LIGHTNING_ID;
pub use crate::{ IncoAccount, IncoMint, COption, AccountState };

/// Create an associated token account for encrypted tokens
pub fn create(ctx: Context<Create>) -> Result<()> {
    let account = &mut ctx.accounts.associated_token;
    let mint = &ctx.accounts.mint;

    require!(mint.is_initialized, IncoAssociatedTokenError::UninitializedMint);
    require!(account.state == AccountState::Uninitialized, IncoAssociatedTokenError::AlreadyInitialized);

    // Initialize the encrypted token account
    account.mint = mint.key();
    account.owner = ctx.accounts.wallet.key();

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.payer.to_account_info();

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

/// Create associated token account idempotent 
pub fn create_idempotent(ctx: Context<CreateIdempotent>) -> Result<()> {
    let account = &mut ctx.accounts.associated_token;
    let mint = &ctx.accounts.mint;

    require!(mint.is_initialized, IncoAssociatedTokenError::UninitializedMint);

    // If already initialized, just return successfully
    if account.state == AccountState::Initialized {
        return Ok(());
    }

    // Same initialization logic as create
    account.mint = mint.key();
    account.owner = ctx.accounts.wallet.key();

    let inco = ctx.accounts.inco_lightning_program.to_account_info();
    let signer = ctx.accounts.payer.to_account_info();

    let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
    let zero_amount = as_euint128(cpi_ctx, 0)?;

    account.amount = zero_amount;
    account.delegate = COption::None;
    account.state = AccountState::Initialized;
    account.is_native = COption::None;

    let cpi_ctx2 = CpiContext::new(inco, Operation { signer });
    let zero_delegated = as_euint128(cpi_ctx2, 0)?;

    account.delegated_amount = zero_delegated;
    account.close_authority = COption::None;

    Ok(())
}

// ========== HELPER FUNCTIONS ==========

/// Get the associated token account address for encrypted tokens
pub fn get_associated_token_address(wallet: &Pubkey, token_mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(wallet, token_mint, &crate::ID)
}

/// Get associated token account address with specific program ID
pub fn get_associated_token_address_with_program_id(
    wallet: &Pubkey,
    token_mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program_id.as_ref(), token_mint.as_ref()],
        &crate::ID,
    ).0
}

// ========== ACCOUNT CONTEXTS ==========

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + IncoAccount::LEN,
        seeds = [
            wallet.key().as_ref(),
            crate::ID.as_ref(),
            mint.key().as_ref(),
        ],
        bump
    )]
    pub associated_token: Account<'info, IncoAccount>,
    
    /// CHECK: This is the wallet that will own the associated token account
    pub wallet: UncheckedAccount<'info>,
    
    #[account(constraint = mint.is_initialized @ IncoAssociatedTokenError::UninitializedMint)]
    pub mint: Account<'info, IncoMint>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateIdempotent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + IncoAccount::LEN,
        seeds = [
            wallet.key().as_ref(),
            crate::ID.as_ref(),
            mint.key().as_ref(),
        ],
        bump
    )]
    pub associated_token: Account<'info, IncoAccount>,
    
    /// CHECK: This is the wallet that will own the associated token account
    pub wallet: UncheckedAccount<'info>,
    
    #[account(constraint = mint.is_initialized @ IncoAssociatedTokenError::UninitializedMint)]
    pub mint: Account<'info, IncoMint>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

// ========== ERROR CODES ==========
#[error_code]
pub enum IncoAssociatedTokenError {
    #[msg("The mint is not initialized")]
    UninitializedMint,
    #[msg("The associated token account is already initialized")]
    AlreadyInitialized,
    #[msg("Invalid inco_lightning program")]
    InvalidProgram,
}

use anchor_lang::prelude::*;
use inco_lightning::cpi::accounts::Operation;
use inco_lightning::cpi::new_euint128;
use inco_lightning::ID as INCO_LIGHTNING_ID;

/// Build encrypted memo - logs encrypted memo to transaction
/// NOTE: Memo content must be convertible to u128 (max 16 bytes) since we use euint128 type
/// This means memo can be max 16 characters for ASCII or represent a numeric value
pub fn build_memo(
    ctx: Context<BuildMemo>, 
    encrypted_memo: Vec<u8>,
    input_type: u8
) -> Result<()> {
    for account_info in ctx.remaining_accounts.iter() {
        require!(account_info.is_signer, IncoMemoError::MissingRequiredSignature);
        msg!("Signed by {}", account_info.key());
    }

    // IMPORTANT: This validates that the encrypted data represents a valid u128 value
    let cpi_ctx = CpiContext::new(
        ctx.accounts.inco_lightning_program.to_account_info(),
        Operation {
            signer: ctx.accounts.authority.to_account_info(),
        }
    );
    let _validated_memo = new_euint128(cpi_ctx, encrypted_memo.clone(), input_type)?;

    // Log encrypted memo data for TEE to capture and decrypt
    msg!("Encrypted memo (len {})", encrypted_memo.len());
    msg!("Encrypted memo data: {:?}", encrypted_memo);
    msg!("Input type: {}", input_type);
    msg!("Primary signer: {}", ctx.accounts.authority.key());

    Ok(())
}

// ========== ACCOUNT CONTEXTS ==========

#[derive(Accounts)]
pub struct BuildMemo<'info> {
    pub authority: Signer<'info>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    // remaining_accounts used for additional signers (like SPL memo)
}

// ========== ERROR CODES ==========
#[error_code]
pub enum IncoMemoError {
    #[msg("Missing required signature for memo")]
    MissingRequiredSignature,
    #[msg("Invalid inco_lightning program")]
    InvalidProgram,
}

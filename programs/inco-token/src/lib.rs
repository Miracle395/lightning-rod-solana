#![allow(unexpected_cfgs)]
#![allow(ambiguous_glob_reexports)]

use anchor_lang::prelude::*;

pub mod token;
pub mod associated_token;
pub mod memo;
pub mod metadata;
pub mod token_2022;

// Re-export everything
pub use token::*;
pub use memo::*;
pub use associated_token::*;
pub use metadata::*;
pub use token_2022::*;

declare_id!("4cyJHzecVWuU2xux6bCAPAhALKQT8woBh4Vx3AGEGe5N");

// ========== SHARED TYPES ==========

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AccountState {
    Uninitialized = 0,
    Initialized = 1,
    Frozen = 2,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum COption<T> {
    None,
    Some(T),
}

impl<T> Default for COption<T> {
    fn default() -> Self {
        COption::None
    }
}

impl<T> COption<T> {
    pub fn is_some(&self) -> bool {
        matches!(self, COption::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, COption::None)
    }
}

// ========== SHARED ACCOUNT STRUCTURES ==========

#[account]
pub struct IncoMint {
    pub mint_authority: COption<Pubkey>,
    pub supply: inco_lightning::types::Euint128,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: COption<Pubkey>,
}

impl IncoMint {
    pub const LEN: usize = 36 + 32 + 1 + 1 + 36; // 106 bytes
}

#[account]
pub struct IncoAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: inco_lightning::types::Euint128,
    pub delegate: COption<Pubkey>,
    pub state: AccountState,
    pub is_native: COption<u64>,
    pub delegated_amount: inco_lightning::types::Euint128,
    pub close_authority: COption<Pubkey>,
}

impl IncoAccount {
    pub const LEN: usize = 32 + 32 + 32 + 36 + 1 + 12 + 32 + 36; // 213 bytes
}

#[program]
pub mod inco_token {
    use super::*;

    // ========== TOKEN INSTRUCTIONS ==========

    pub fn initialize_mint(
        ctx: Context<InitializeMint>,
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: Option<Pubkey>
    ) -> Result<()> {
        token::initialize_mint(ctx, decimals, mint_authority, freeze_authority)
    }

    pub fn initialize_account(ctx: Context<InitializeAccount>) -> Result<()> {
        token::initialize_account(ctx)
    }

    /// Mint tokens to an account
    /// remaining_accounts: [allowance_account, owner_address]
    pub fn mint_to<'info>(
        ctx: Context<'_, '_, '_, 'info, IncoMintTo<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8
    ) -> Result<()> {
        token::mint_to(ctx, ciphertext, input_type)
    }

    /// Transfer tokens between accounts
    /// remaining_accounts: [source_allowance, source_owner, dest_allowance, dest_owner]
    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, IncoTransfer<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8
    ) -> Result<()> {
        token::transfer(ctx, ciphertext, input_type)
    }

    /// Approve a delegate
    /// remaining_accounts: [allowance_account, delegate_address]
    pub fn approve<'info>(
        ctx: Context<'_, '_, '_, 'info, IncoApprove<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8
    ) -> Result<()> {
        token::approve(ctx, ciphertext, input_type)
    }

    pub fn revoke(ctx: Context<IncoRevoke>) -> Result<()> {
        token::revoke(ctx)
    }

    /// Burn tokens
    /// remaining_accounts: [allowance_account, owner_address]
    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, IncoBurn<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8
    ) -> Result<()> {
        token::burn(ctx, ciphertext, input_type)
    }

    pub fn freeze_account(ctx: Context<FreezeAccount>) -> Result<()> {
        token::freeze_account(ctx)
    }

    pub fn thaw_account(ctx: Context<ThawAccount>) -> Result<()> {
        token::thaw_account(ctx)
    }

    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        token::close_account(ctx)
    }

    pub fn set_mint_authority(ctx: Context<SetMintAuthority>, new_authority: Option<Pubkey>) -> Result<()> {
        token::set_mint_authority(ctx, new_authority)
    }

    pub fn set_freeze_authority(ctx: Context<SetFreezeAuthority>, new_authority: Option<Pubkey>) -> Result<()> {
        token::set_freeze_authority(ctx, new_authority)
    }

    pub fn set_account_owner(ctx: Context<SetAccountOwner>, new_owner: Pubkey) -> Result<()> {
        token::set_account_owner(ctx, new_owner)
    }

    pub fn set_close_authority(ctx: Context<SetCloseAuthority>, new_authority: Option<Pubkey>) -> Result<()> {
        token::set_close_authority(ctx, new_authority)
    }

    // ========== MEMO INSTRUCTIONS ==========

    pub fn build_memo(ctx: Context<BuildMemo>, encrypted_memo: Vec<u8>, input_type: u8) -> Result<()> {
        memo::build_memo(ctx, encrypted_memo, input_type)
    }

    // ========== ASSOCIATED TOKEN INSTRUCTIONS ==========

    pub fn create(ctx: Context<Create>) -> Result<()> {
        associated_token::create(ctx)
    }

    pub fn create_idempotent(ctx: Context<CreateIdempotent>) -> Result<()> {
        associated_token::create_idempotent(ctx)
    }

    // ========== METADATA INSTRUCTIONS ==========

    pub fn create_metadata_account(ctx: Context<CreateMetadata>, args: CreateMetadataArgs) -> Result<()> {
        metadata::create_metadata_account(ctx, args)
    }

    pub fn update_metadata_account(ctx: Context<UpdateMetadata>, args: UpdateMetadataArgs) -> Result<()> {
        metadata::update_metadata_account(ctx, args)
    }

    pub fn create_master_edition(ctx: Context<CreateMasterEdition>, args: CreateMasterEditionArgs) -> Result<()> {
        metadata::create_master_edition(ctx, args)
    }

    pub fn print_edition(ctx: Context<PrintEdition>, args: PrintEditionArgs) -> Result<()> {
        metadata::print_edition(ctx, args)
    }

    pub fn sign_metadata(ctx: Context<SignMetadata>) -> Result<()> {
        metadata::sign_metadata(ctx)
    }

    pub fn remove_creator_verification(ctx: Context<RemoveCreatorVerification>) -> Result<()> {
        metadata::remove_creator_verification(ctx)
    }

    pub fn set_and_verify_collection(ctx: Context<SetAndVerifyCollection>, collection: Collection) -> Result<()> {
        metadata::set_and_verify_collection(ctx, collection)
    }

    pub fn verify_collection(ctx: Context<VerifyCollection>) -> Result<()> {
        metadata::verify_collection(ctx)
    }

    pub fn unverify_collection(ctx: Context<UnverifyCollection>) -> Result<()> {
        metadata::unverify_collection(ctx)
    }

    // ========== TOKEN 2022 INSTRUCTIONS ==========

    pub fn transfer_checked<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferChecked<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8,
        decimals: u8
    ) -> Result<()> {
        token_2022::transfer_checked(ctx, ciphertext, input_type, decimals)
    }

    pub fn mint_to_checked<'info>(
        ctx: Context<'_, '_, '_, 'info, MintToChecked<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8,
        decimals: u8
    ) -> Result<()> {
        token_2022::mint_to_checked(ctx, ciphertext, input_type, decimals)
    }

    pub fn burn_checked<'info>(
        ctx: Context<'_, '_, '_, 'info, BurnChecked<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8,
        decimals: u8
    ) -> Result<()> {
        token_2022::burn_checked(ctx, ciphertext, input_type, decimals)
    }

    pub fn approve_checked<'info>(
        ctx: Context<'_, '_, '_, 'info, ApproveChecked<'info>>,
        ciphertext: Vec<u8>,
        input_type: u8,
        decimals: u8
    ) -> Result<()> {
        token_2022::approve_checked(ctx, ciphertext, input_type, decimals)
    }

    pub fn initialize_account3<'info>(ctx: Context<'_, '_, '_, 'info, InitializeAccount3<'info>>) -> Result<()> {
        token_2022::initialize_account3(ctx)
    }

    pub fn revoke_2022<'info>(ctx: Context<'_, '_, '_, 'info, Revoke2022<'info>>) -> Result<()> {
        token_2022::revoke_2022(ctx)
    }

    pub fn close_account_2022<'info>(ctx: Context<'_, '_, '_, 'info, CloseAccount2022<'info>>) -> Result<()> {
        token_2022::close_account_2022(ctx)
    }
}

// ========== ERROR CODES ==========
#[error_code]
pub enum CustomError {
    #[msg("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Invalid Mint")]
    InvalidMint,
    #[msg("Account not associated with this Mint")]
    MintMismatch,
    #[msg("Owner does not match")]
    OwnerMismatch,
    #[msg("Fixed supply")]
    FixedSupply,
    #[msg("Account already in use")]
    AlreadyInUse,
    #[msg("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners,
    #[msg("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners,
    #[msg("State is uninitialized")]
    UninitializedState,
    #[msg("Native tokens not supported")]
    NativeNotSupported,
    #[msg("Non-native account has balance")]
    NonNativeHasBalance,
    #[msg("Invalid instruction")]
    InvalidInstruction,
    #[msg("Invalid state")]
    InvalidState,
    #[msg("Overflow")]
    Overflow,
    #[msg("Authority type not supported")]
    AuthorityTypeNotSupported,
    #[msg("Mint cannot freeze")]
    MintCannotFreeze,
    #[msg("Account frozen")]
    AccountFrozen,
    #[msg("Mint decimals mismatch")]
    MintDecimalsMismatch,
    #[msg("Non-native not supported")]
    NonNativeNotSupported,
}

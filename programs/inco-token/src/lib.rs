#![allow(unexpected_cfgs)]
#![allow(ambiguous_glob_reexports)]

use anchor_lang::prelude::*;
use inco_lightning::types::Euint128;

pub mod token;
pub mod associated_token;
pub mod memo;
pub mod metadata;
pub mod token_2022;

// Re-export everything - the #[program] macro needs these in scope
pub use token::*;
pub use memo::*;
pub use associated_token::*;
pub use metadata::*;
pub use token_2022::*;

declare_id!("3hdtBkqkTt2pmNUetqq5KUAmwGUxuFEKGtJ62kDtNvQT");

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
    /// Optional authority used to mint new tokens
    pub mint_authority: COption<Pubkey>,
    /// Total supply of tokens encrypted
    pub supply: Euint128,
    /// Number of base 10 digits to the right of the decimal place
    pub decimals: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Optional authority to freeze token accounts
    pub freeze_authority: COption<Pubkey>,
}

impl IncoMint {
    /// Length of a Mint account
    pub const LEN: usize = 36 + 32 + 1 + 1 + 36; // 106 bytes
}

#[account]
pub struct IncoAccount {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account
    pub owner: Pubkey,
    /// The amount of tokens this account holds (encrypted)
    pub amount: Euint128,
    /// If `delegate` is `Some` then `delegated_amount` represents the amount authorized by the delegate
    pub delegate: COption<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_some, this is a native token, and the value logs the rent-exempt reserve
    pub is_native: COption<u64>,
    /// The amount delegated (encrypted)
    pub delegated_amount: Euint128,
    /// Optional authority to close the account
    pub close_authority: COption<Pubkey>,
}

impl IncoAccount {
    /// Length of an Account
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

    pub fn mint_to(ctx: Context<IncoMintTo>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
        token::mint_to(ctx, ciphertext, input_type)
    }

    pub fn mint_to_with_handle(ctx: Context<IncoMintTo>, amount_handle: Euint128) -> Result<()> {
        token::mint_to_with_handle(ctx, amount_handle)
    }

    pub fn transfer(ctx: Context<IncoTransfer>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
        token::transfer(ctx, ciphertext, input_type)
    }

    pub fn transfer_with_handle(ctx: Context<IncoTransfer>, amount_handle: Euint128) -> Result<()> {
        token::transfer_with_handle(ctx, amount_handle)
    }

    pub fn approve(ctx: Context<IncoApprove>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
        token::approve(ctx, ciphertext, input_type)
    }

    pub fn revoke(ctx: Context<IncoRevoke>) -> Result<()> {
        token::revoke(ctx)
    }

    pub fn burn(ctx: Context<IncoBurn>, ciphertext: Vec<u8>, input_type: u8) -> Result<()> {
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

    pub fn wrap(ctx: Context<Wrap>, amount: u64) -> Result<()> {
        token::wrap(ctx, amount)
    }

    pub fn set_mint_authority(
        ctx: Context<SetMintAuthority>,
        new_authority: Option<Pubkey>
    ) -> Result<()> {
        token::set_mint_authority(ctx, new_authority)
    }

    pub fn set_freeze_authority(
        ctx: Context<SetFreezeAuthority>,
        new_authority: Option<Pubkey>
    ) -> Result<()> {
        token::set_freeze_authority(ctx, new_authority)
    }

    pub fn set_account_owner(ctx: Context<SetAccountOwner>, new_owner: Pubkey) -> Result<()> {
        token::set_account_owner(ctx, new_owner)
    }

    pub fn set_close_authority(
        ctx: Context<SetCloseAuthority>,
        new_authority: Option<Pubkey>
    ) -> Result<()> {
        token::set_close_authority(ctx, new_authority)
    }

    // ========== MEMO INSTRUCTIONS ==========

    pub fn build_memo(
        ctx: Context<BuildMemo>,
        encrypted_memo: Vec<u8>,
        input_type: u8
    ) -> Result<()> {
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

    pub fn create_metadata_account(
        ctx: Context<CreateMetadata>,
        args: CreateMetadataArgs
    ) -> Result<()> {
        metadata::create_metadata_account(ctx, args)
    }

    pub fn update_metadata_account(
        ctx: Context<UpdateMetadata>,
        args: UpdateMetadataArgs
    ) -> Result<()> {
        metadata::update_metadata_account(ctx, args)
    }

    pub fn create_master_edition(
        ctx: Context<CreateMasterEdition>,
        args: CreateMasterEditionArgs
    ) -> Result<()> {
        metadata::create_master_edition(ctx, args)
    }

    pub fn print_edition(
        ctx: Context<PrintEdition>,
        args: PrintEditionArgs
    ) -> Result<()> {
        metadata::print_edition(ctx, args)
    }

    pub fn sign_metadata(ctx: Context<SignMetadata>) -> Result<()> {
        metadata::sign_metadata(ctx)
    }

    pub fn remove_creator_verification(ctx: Context<RemoveCreatorVerification>) -> Result<()> {
        metadata::remove_creator_verification(ctx)
    }

    pub fn set_and_verify_collection(
        ctx: Context<SetAndVerifyCollection>,
        collection: Collection
    ) -> Result<()> {
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

    pub fn transfer_checked_with_handle<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferChecked<'info>>,
        amount_handle: Euint128,
        decimals: u8
    ) -> Result<()> {
        token_2022::transfer_checked_with_handle(ctx, amount_handle, decimals)
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

    pub fn initialize_account3<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeAccount3<'info>>
    ) -> Result<()> {
        token_2022::initialize_account3(ctx)
    }

    pub fn revoke_2022<'info>(ctx: Context<'_, '_, '_, 'info, Revoke2022<'info>>) -> Result<()> {
        token_2022::revoke_2022(ctx)
    }

    pub fn close_account_2022<'info>(
        ctx: Context<'_, '_, '_, 'info, CloseAccount2022<'info>>
    ) -> Result<()> {
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
    #[msg("Fixed supply. Token mint cannot mint additional tokens")]
    FixedSupply,
    #[msg("The account cannot be initialized because it is already being used")]
    AlreadyInUse,
    #[msg("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners,
    #[msg("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners,
    #[msg("State is uninitialized")]
    UninitializedState,
    #[msg("Instruction does not support native tokens")]
    NativeNotSupported,
    #[msg("Non-native account can only be closed if its balance is zero")]
    NonNativeHasBalance,
    #[msg("Invalid instruction")]
    InvalidInstruction,
    #[msg("Invalid state")]
    InvalidState,
    #[msg("Operation overflowed")]
    Overflow,
    #[msg("Account does not support specified authority type")]
    AuthorityTypeNotSupported,
    #[msg("This token mint cannot freeze accounts")]
    MintCannotFreeze,
    #[msg("The account is frozen")]
    AccountFrozen,
    #[msg("The provided decimals value different from the Mint decimals")]
    MintDecimalsMismatch,
    #[msg("Instruction does not support non-native tokens")]
    NonNativeNotSupported,
}

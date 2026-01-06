use anchor_lang::prelude::*;

// ========== INSTRUCTION ARGS ==========

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct CreateMetadataArgs {
    pub name: String,
    pub symbol: String, 
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<Creator>>,
    pub is_mutable: bool,
    pub collection: Option<Collection>,
    pub uses: Option<Uses>,
    pub collection_details: Option<CollectionDetails>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct UpdateMetadataArgs {
    pub new_update_authority: Option<Pubkey>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub seller_fee_basis_points: Option<u16>,
    pub creators: Option<Vec<Creator>>,
    pub primary_sale_happened: Option<bool>,
    pub is_mutable: Option<bool>,
    pub collection: CollectionToggle,
    pub collection_details: CollectionDetailsToggle,
    pub uses: UsesToggle,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct CreateMasterEditionArgs {
    pub max_supply: Option<u64>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct PrintEditionArgs {
    pub edition: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct MintArgs {
    pub amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct TransferArgs {
    pub amount: u64,
}

// ========== TOGGLE TYPES ==========

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CollectionToggle {
    #[default]
    None,
    Clear,
    Set(Collection),
}

impl CollectionToggle {
    pub fn is_some(&self) -> bool {
        matches!(self, CollectionToggle::Clear | CollectionToggle::Set(_))
    }

    pub fn to_option(self) -> Option<Collection> {
        match self {
            CollectionToggle::Set(value) => Some(value),
            CollectionToggle::Clear => None,
            CollectionToggle::None => panic!("Tried to convert 'None' value"),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum UsesToggle {
    #[default]
    None,
    Clear,
    Set(Uses),
}

impl UsesToggle {
    pub fn is_some(&self) -> bool {
        matches!(self, UsesToggle::Clear | UsesToggle::Set(_))
    }

    pub fn to_option(self) -> Option<Uses> {
        match self {
            UsesToggle::Set(value) => Some(value),
            UsesToggle::Clear => None,
            UsesToggle::None => panic!("Tried to convert 'None' value"),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CollectionDetailsToggle {
    #[default]
    None,
    Clear,
    Set(CollectionDetails),
}

impl CollectionDetailsToggle {
    pub fn is_some(&self) -> bool {
        matches!(self, CollectionDetailsToggle::Clear | CollectionDetailsToggle::Set(_))
    }

    pub fn to_option(self) -> Option<CollectionDetails> {
        match self {
            CollectionDetailsToggle::Set(value) => Some(value),
            CollectionDetailsToggle::Clear => None,
            CollectionDetailsToggle::None => panic!("Tried to convert 'None' value"),
        }
    }
}

// ========== CORE INSTRUCTIONS ==========

pub fn create_metadata_account(
    ctx: Context<CreateMetadata>,
    args: CreateMetadataArgs,
) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;
    let mint = &ctx.accounts.mint;

    require!(!metadata.is_initialized, IncoMetadataError::AlreadyInitialized);
    
    // Validate string lengths to prevent excessive storage costs
    require!(args.name.len() <= 32, IncoMetadataError::NameTooLong);
    require!(args.symbol.len() <= 10, IncoMetadataError::SymbolTooLong);
    require!(args.uri.len() <= 200, IncoMetadataError::UriTooLong);

    // Set metadata fields (standard plaintext implementation)
    metadata.key = MetadataKey::MetadataV1;
    metadata.update_authority = ctx.accounts.update_authority.key();
    metadata.mint = mint.key();
    metadata.name = args.name;
    metadata.symbol = args.symbol;
    metadata.uri = args.uri;
    metadata.seller_fee_basis_points = args.seller_fee_basis_points;
    metadata.creators = args.creators;
    metadata.primary_sale_happened = false;
    metadata.is_mutable = args.is_mutable;
    metadata.edition_nonce = None;
    metadata.token_standard = Some(TokenStandard::NonFungible);
    metadata.collection = args.collection;
    metadata.uses = args.uses;
    metadata.collection_details = args.collection_details;
    metadata.is_initialized = true;

    Ok(())
}

pub fn update_metadata_account(
    ctx: Context<UpdateMetadata>,
    args: UpdateMetadataArgs,
) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;
    
    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);
    require!(metadata.update_authority == ctx.accounts.update_authority.key(), IncoMetadataError::UpdateAuthorityMismatch);
    require!(metadata.is_mutable, IncoMetadataError::DataIsImmutable);

    // Update name if provided
    if let Some(name) = args.name {
        require!(name.len() <= 32, IncoMetadataError::NameTooLong);
        metadata.name = name;
    }

    // Update symbol if provided
    if let Some(symbol) = args.symbol {
        require!(symbol.len() <= 10, IncoMetadataError::SymbolTooLong);
        metadata.symbol = symbol;
    }

    // Update URI if provided
    if let Some(uri) = args.uri {
        require!(uri.len() <= 200, IncoMetadataError::UriTooLong);
        metadata.uri = uri;
    }

    // Update other fields
    if let Some(new_authority) = args.new_update_authority {
        metadata.update_authority = new_authority;
    }

    if let Some(fee_basis_points) = args.seller_fee_basis_points {
        metadata.seller_fee_basis_points = fee_basis_points;
    }

    if let Some(creators) = args.creators {
        metadata.creators = Some(creators);
    }

    if let Some(sale_happened) = args.primary_sale_happened {
        metadata.primary_sale_happened = sale_happened;
    }

    if let Some(mutable) = args.is_mutable {
        metadata.is_mutable = mutable;
    }

    if args.collection.is_some() {
        metadata.collection = args.collection.to_option();
    }

    if args.collection_details.is_some() {
        metadata.collection_details = args.collection_details.to_option();
    }

    if args.uses.is_some() {
        metadata.uses = args.uses.to_option();
    }

    Ok(())
}

/// Create master edition account for NFTs
pub fn create_master_edition(
    ctx: Context<CreateMasterEdition>,
    args: CreateMasterEditionArgs,
) -> Result<()> {
    let edition = &mut ctx.accounts.edition;
    let metadata = &ctx.accounts.metadata;

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);
    require!(!edition.is_initialized, IncoMetadataError::AlreadyInitialized);

    edition.key = MetadataKey::MasterEditionV2;
    edition.supply = 0;
    edition.max_supply = args.max_supply;
    edition.is_initialized = true;

    Ok(())
}

/// Print edition from master edition
pub fn print_edition(
    ctx: Context<PrintEdition>,
    args: PrintEditionArgs,
) -> Result<()> {
    let edition = &mut ctx.accounts.edition;
    let master_edition = &mut ctx.accounts.master_edition;

    require!(master_edition.is_initialized, IncoMetadataError::NotInitialized);
    require!(!edition.is_initialized, IncoMetadataError::AlreadyInitialized);

    // Check max supply limit
    if let Some(max_supply) = master_edition.max_supply {
        require!(master_edition.supply < max_supply, IncoMetadataError::MaxSupplyReached);
    }

    // Increment supply
    master_edition.supply = master_edition.supply
        .checked_add(1)
        .ok_or(IncoMetadataError::NumericalOverflow)?;

    // Initialize print edition
    edition.key = MetadataKey::EditionV1;
    edition.parent = master_edition.key();
    edition.edition = args.edition;
    edition.is_initialized = true;

    Ok(())
}

/// Sign metadata as creator
pub fn sign_metadata(ctx: Context<SignMetadata>) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;
    let creator = ctx.accounts.creator.key();

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);

    if let Some(ref mut creators) = metadata.creators {
        for creator_entry in creators.iter_mut() {
            if creator_entry.address == creator {
                creator_entry.verified = true;
                return Ok(());
            }
        }
    }

    Err(IncoMetadataError::CreatorNotFound.into())
}

/// Remove creator verification
pub fn remove_creator_verification(ctx: Context<RemoveCreatorVerification>) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;
    let creator = ctx.accounts.creator.key();

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);

    if let Some(ref mut creators) = metadata.creators {
        for creator_entry in creators.iter_mut() {
            if creator_entry.address == creator {
                creator_entry.verified = false;
                return Ok(());
            }
        }
    }

    Err(IncoMetadataError::CreatorNotFound.into())
}

/// Set and verify collection
pub fn set_and_verify_collection(
    ctx: Context<SetAndVerifyCollection>,
    collection: Collection,
) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);
    require!(metadata.update_authority == ctx.accounts.update_authority.key(), IncoMetadataError::UpdateAuthorityMismatch);

    metadata.collection = Some(Collection {
        verified: true,
        key: collection.key,
    });

    Ok(())
}

/// Verify collection
pub fn verify_collection(ctx: Context<VerifyCollection>) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);

    if let Some(ref mut collection) = metadata.collection {
        collection.verified = true;
    } else {
        return Err(IncoMetadataError::CollectionNotSet.into());
    }

    Ok(())
}

/// Unverify collection
pub fn unverify_collection(ctx: Context<UnverifyCollection>) -> Result<()> {
    let metadata = &mut ctx.accounts.metadata;

    require!(metadata.is_initialized, IncoMetadataError::NotInitialized);

    if let Some(ref mut collection) = metadata.collection {
        collection.verified = false;
    } else {
        return Err(IncoMetadataError::CollectionNotSet.into());
    }

    Ok(())
}

// ========== ACCOUNT STRUCTURES ==========

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum MetadataKey {
    Uninitialized,
    MetadataV1,
    EditionV1,
    MasterEditionV1,
    MasterEditionV2,
    EditionMarker,
}

#[account]
pub struct Metadata {
    pub key: MetadataKey,
    pub update_authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,           
    pub symbol: String,         
    pub uri: String,            
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<Creator>>,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
    pub edition_nonce: Option<u8>,
    pub token_standard: Option<TokenStandard>,
    pub collection: Option<Collection>,
    pub uses: Option<Uses>,
    pub collection_details: Option<CollectionDetails>,
    pub is_initialized: bool,
}

impl Metadata {
    // Increased space allocation to accommodate string storage
    pub const LEN: usize = 1 + 32 + 32 + (4 + 32) + (4 + 10) + (4 + 200) + 2 + 200 + 1 + 1 + 2 + 2 + 50 + 50 + 50 + 1;
}

#[account]
pub struct MasterEdition {
    pub key: MetadataKey,
    pub supply: u64,
    pub max_supply: Option<u64>,
    pub is_initialized: bool,
}

impl MasterEdition {
    pub const LEN: usize = 1 + 8 + 9 + 1;
}

#[account]
pub struct Edition {
    pub key: MetadataKey,
    pub parent: Pubkey,
    pub edition: u64,
    pub is_initialized: bool,
}

impl Edition {
    pub const LEN: usize = 1 + 32 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    pub share: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum TokenStandard {
    NonFungible,
    FungibleAsset,
    Fungible,
    NonFungibleEdition,
    ProgrammableNonFungible,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct Uses {
    pub use_method: UseMethod,
    pub remaining: u64,
    pub total: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum UseMethod {
    Burn,
    Multiple,
    Single,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum CollectionDetails {
    V1 { size: u64 },
}

// ========== ACCOUNT CONTEXTS ==========

#[derive(Accounts)]
pub struct CreateMetadata<'info> {
    #[account(init, payer = payer, space = 8 + Metadata::LEN)]
    pub metadata: Account<'info, Metadata>,
    /// CHECK: Mint account for the metadata
    pub mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Update authority for the metadata
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMetadata<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
        constraint = metadata.update_authority == update_authority.key() @ IncoMetadataError::UpdateAuthorityMismatch,
    )]
    pub metadata: Account<'info, Metadata>,
    #[account(mut)]
    pub update_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateMasterEdition<'info> {
    #[account(init, payer = payer, space = 8 + MasterEdition::LEN)]
    pub edition: Account<'info, MasterEdition>,
    #[account(
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
        constraint = metadata.mint == mint.key() @ IncoMetadataError::MintMismatch,
    )]
    pub metadata: Account<'info, Metadata>,
    /// CHECK: Mint for the master edition
    pub mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Update authority
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PrintEdition<'info> {
    #[account(init, payer = payer, space = 8 + Edition::LEN)]
    pub edition: Account<'info, Edition>,
    #[account(
        mut,
        constraint = master_edition.is_initialized @ IncoMetadataError::NotInitialized,
    )]
    pub master_edition: Account<'info, MasterEdition>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SignMetadata<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
    )]
    pub metadata: Account<'info, Metadata>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveCreatorVerification<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
    )]
    pub metadata: Account<'info, Metadata>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetAndVerifyCollection<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
        constraint = metadata.update_authority == update_authority.key() @ IncoMetadataError::UpdateAuthorityMismatch,
    )]
    pub metadata: Account<'info, Metadata>,
    pub update_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyCollection<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
    )]
    pub metadata: Account<'info, Metadata>,
    pub collection_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnverifyCollection<'info> {
    #[account(
        mut,
        constraint = metadata.is_initialized @ IncoMetadataError::NotInitialized,
    )]
    pub metadata: Account<'info, Metadata>,
    pub collection_authority: Signer<'info>,
}

// ========== ERROR CODES ==========
#[error_code]
pub enum IncoMetadataError {
    #[msg("Metadata account is already initialized")]
    AlreadyInitialized,
    #[msg("Metadata account is not initialized")]
    NotInitialized,
    #[msg("Update authority mismatch")]
    UpdateAuthorityMismatch,
    #[msg("Data is immutable")]
    DataIsImmutable,
    #[msg("Mint mismatch")]
    MintMismatch,
    #[msg("Creator not found")]
    CreatorNotFound,
    #[msg("Collection not set")]
    CollectionNotSet,
    #[msg("Maximum supply reached")]
    MaxSupplyReached,
    #[msg("Numerical overflow")]
    NumericalOverflow,
    #[msg("Name too long (max 32 characters)")]
    NameTooLong,
    #[msg("Symbol too long (max 10 characters)")]
    SymbolTooLong,
    #[msg("URI too long (max 200 characters)")]
    UriTooLong,
}

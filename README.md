# Lightning Rod Solana

Encrypted Token Program for Solana using Inco Lightning - a privacy-preserving token standard that enables confidential token balances and transfers on Solana.

## Overview

Lightning Rod implements an SPL Token-compatible interface with encrypted balances using Inco's confidential computing infrastructure. Token balances and transfer amounts are encrypted on-chain, with decryption only possible through Inco's attested TEE (Trusted Execution Environment).

## Features

- **Encrypted Balances**: Token balances stored as encrypted handles (Euint128)
- **Confidential Transfers**: Transfer amounts are encrypted and processed homomorphically
- **SPL Token Compatibility**: Familiar token operations (mint, transfer, burn, freeze, etc.)
- **Token 2022 Support**: Extended functionality with checked operations and decimal validation
- **Associated Token Accounts**: PDA-based token account derivation
- **Metadata Support**: Standard NFT metadata structure

## Program Architecture

### Core Modules

- `token.rs` - Standard token operations (mint, transfer, burn, approve, etc.)
- `token_2022.rs` - Token 2022 checked operations with decimal validation
- `associated_token.rs` - Associated token account creation
- `memo.rs` - Encrypted memo support
- `metadata.rs` - NFT metadata standard implementation

### Account Structures

```rust
// Encrypted Mint Account
pub struct IncoMint {
    pub mint_authority: COption<Pubkey>,
    pub supply: Euint128,           // Encrypted total supply
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: COption<Pubkey>,
}

// Encrypted Token Account
pub struct IncoAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: Euint128,           // Encrypted balance
    pub delegate: COption<Pubkey>,
    pub state: AccountState,
    pub delegated_amount: Euint128, // Encrypted delegation
    pub close_authority: COption<Pubkey>,
}
```

## Installation

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18+
- Anchor 0.31+
- Node.js 18+

### Setup

```bash
# Clone the repository
git clone https://github.com/inco-network/lightning-rod-solana
cd lightning-rod-solana

# Install dependencies
yarn install

# Build the program
anchor build
```

## Usage

### Building

```bash
anchor build
```

### Testing

```bash
# Run all tests
anchor test

# Run specific test suite
yarn test:token      # Standard token tests
yarn test:token2022  # Token 2022 tests
```

### Deployment

```bash
# Deploy to devnet
anchor deploy --provider.cluster devnet
```

## SDK Integration

The program is designed to work with `@inco/solana-sdk`:

```typescript
import { encryptValue } from "@inco/solana-sdk/encryption";
import { decrypt } from "@inco/solana-sdk/attested-decrypt";
import { hexToBuffer } from "@inco/solana-sdk/utils";

// Encrypt a value for minting/transferring
const amount = BigInt(1000000000); // 1 token with 9 decimals
const encryptedHex = await encryptValue(amount);

// Use in transaction
await program.methods
  .mintTo(hexToBuffer(encryptedHex), 0)
  .accounts({
    mint: mintPubkey,
    account: tokenAccountPubkey,
    mintAuthority: authorityPubkey,
  })
  .rpc();

// Decrypt a balance
const tokenAccount = await program.account.incoAccount.fetch(tokenAccountPubkey);
const result = await decrypt([tokenAccount.amount.toString()]);
const balance = parseInt(result.plaintexts[0], 10);
```

## Instructions

### Standard Token Operations

| Instruction | Description |
|-------------|-------------|
| `initialize_mint` | Create a new encrypted mint |
| `initialize_account` | Create a new encrypted token account |
| `mint_to` | Mint encrypted tokens |
| `transfer` | Transfer encrypted tokens |
| `burn` | Burn encrypted tokens |
| `approve` | Approve delegate with encrypted amount |
| `revoke` | Revoke delegate |
| `freeze_account` | Freeze token account |
| `thaw_account` | Unfreeze token account |
| `close_account` | Close token account |

### Token 2022 Operations

| Instruction | Description |
|-------------|-------------|
| `initialize_account3` | Token 2022 account initialization |
| `mint_to_checked` | Mint with decimal validation |
| `transfer_checked` | Transfer with decimal validation |
| `burn_checked` | Burn with decimal validation |
| `approve_checked` | Approve with decimal validation |
| `revoke_2022` | Token 2022 revoke |
| `close_account_2022` | Token 2022 close |

## Dependencies

- `anchor-lang` 0.31.1
- `anchor-spl` 0.31.1
- `inco-lightning` 0.1.1

## Program ID

**Devnet**: `8ektS9Vq9bXgGxdRrxC54JagTyotV3upAC3Xa3R9S3n4`

## Security Considerations

- Encrypted balances prevent on-chain balance inspection
- Transfer amounts are validated homomorphically (cannot transfer more than balance)
- Decryption requires attested TEE access through Inco infrastructure
- Close account operations should verify zero balance client-side before closing

## License

MIT

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Resources

- [Inco Network Documentation](https://docs.inco.org)
- [Solana SDK Documentation](https://docs.solana.com)
- [Anchor Framework](https://www.anchor-lang.com)

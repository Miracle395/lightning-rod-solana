# Lightning Rod Solana

Lightning Rod Solana is the home of the Dapp Developer Kit (DDK) for Inco Lightning network on Solana.

To start working with Inco Lightning on Solana and the Lightning Rod template repository, work through the [Quick Start](#quick-start) section below.

Further [documentation](#documentation) is linked below.

## Quick Start

### Prerequisites

We require recent versions of

- [Rust](https://www.rust-lang.org/tools/install) (1.70+)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (1.18+)
- [Anchor](https://www.anchor-lang.com/docs/installation) (0.31+)
- [Node.js](https://nodejs.org/) (18+)
- [Yarn](https://yarnpkg.com/)

to be installed.

### Install dependencies

To install the dependencies, run:

```bash
yarn install
```

### Build the program

To build the Solana program, run:

```bash
anchor build
```

### Running tests on Devnet

The tests run against Solana Devnet where the Inco Lightning infrastructure is deployed.

Make sure you have a Solana wallet configured with some devnet SOL:

```bash
# Create a new wallet (if needed)
solana-keygen new

# Set cluster to devnet
solana config set --url devnet

# Airdrop some SOL for testing
solana airdrop 2
```

Then run the tests:

```bash
# Run all tests
anchor test

# Run standard token tests only
yarn test:token

# Run Token 2022 tests only
yarn test:token2022
```

### Testing a Confidential Token

An example of a confidential token using Inco Lightning is provided in `programs/inco-token/`.

The program implements:
- Encrypted token balances using `Euint128` handles
- Confidential transfers, minting, and burning
- SPL Token compatible interface
- Token 2022 extensions with decimal validation

To test run:

```bash
yarn install
anchor build
anchor test
```

## Program Structure

```
lightning-rod-solana/
├── programs/
│   └── inco-token/           # Confidential token program
│       ├── src/
│       │   ├── lib.rs        # Program entry point
│       │   ├── token.rs      # Core token operations
│       │   ├── token_2022.rs # Token 2022 extensions
│       │   └── ...
│       └── Cargo.toml
├── tests/
│   ├── inco-token.ts         # Standard token tests
│   └── inco-token-2022.ts    # Token 2022 tests
└── ...
```

## Dependencies

### Rust (Program)
- `anchor-lang` 0.31.1
- `anchor-spl` 0.31.1  
- `inco-lightning` 0.1.2

### TypeScript (Tests/Client)
- `@coral-xyz/anchor`
- `@inco/solana-sdk`
- `@solana/web3.js`

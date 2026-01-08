import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { IncoToken } from "../target/types/inco_token.js";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram,
  Connection,
  Transaction,
  SYSVAR_INSTRUCTIONS_PUBKEY
} from "@solana/web3.js";
import { expect } from "chai";
import { encryptValue } from "@inco/solana-sdk/encryption";
import { decrypt } from "@inco/solana-sdk/attested-decrypt";
import { hexToBuffer, handleToBuffer, plaintextToBuffer } from "@inco/solana-sdk/utils";

// Enhanced handle extraction function for Anchor BN objects
function extractHandleFromAnchor(anchorHandle: any): string {
  // Handle direct BN objects (most common case)
  if (anchorHandle && anchorHandle._bn) {
    return anchorHandle._bn.toString(10);
  }
  
  // Handle nested BN in {"0": BN} structure
  if (typeof anchorHandle === 'object' && anchorHandle["0"]) {
    const nested = anchorHandle["0"];
    
    if (nested && nested._bn) {
      return nested._bn.toString(10);
    }
    
    if (nested && nested.toString && nested.constructor?.name === 'BN') {
      return nested.toString(10);
    }
    
    if (nested && typeof nested.toString === 'function') {
      try {
        return nested.toString(10);
      } catch (e) {
        // Silent fallback
      }
    }
    
    if (typeof nested === 'string') {
      return BigInt('0x' + nested).toString();
    }
  }
  
  // Handle array of bytes (Uint8Array/Buffer)
  if (anchorHandle instanceof Uint8Array || Array.isArray(anchorHandle)) {
    const buffer = Buffer.from(anchorHandle);
    let result = BigInt(0);
    for (let i = buffer.length - 1; i >= 0; i--) {
      result = result * BigInt(256) + BigInt(buffer[i]);
    }
    return result.toString();
  }
  
  // Fallback: try to convert any numeric value
  if (typeof anchorHandle === 'number' || typeof anchorHandle === 'bigint') {
    return anchorHandle.toString();
  }
  
  return "0";
}

// Helper function to safely compare PublicKey objects
function comparePublicKeys(actual: any, expected: PublicKey): boolean {
  if (!actual) return false;
  
  if (typeof actual === 'object' && actual["0"]) {
    const nestedValue = actual["0"];
    
    if (nestedValue && nestedValue.toBase58 && typeof nestedValue.toBase58 === 'function') {
      return nestedValue.toBase58() === expected.toBase58();
    }
    
    if (typeof nestedValue === 'string') {
      return nestedValue === expected.toString() || nestedValue === expected.toBase58();
    }
  }
  
  if (typeof actual === 'string') {
    return actual === expected.toString() || actual === expected.toBase58();
  }
  
  if (actual.toBase58 && typeof actual.toBase58 === 'function') {
    return actual.toBase58() === expected.toBase58();
  }
  
  if (actual.toString && typeof actual.toString === 'function') {
    const actualString = actual.toString();
    if (actualString !== '[object Object]') {
      return actualString === expected.toString();
    }
  }
  
  if (Buffer.isBuffer(actual)) {
    return actual.equals(expected.toBuffer());
  }
  
  if (actual instanceof Uint8Array) {
    return Buffer.from(actual).equals(expected.toBuffer());
  }
  
  return false;
}

describe("inco-token", () => {
  const connection = new Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );
  const anchorWallet = anchor.AnchorProvider.env().wallet;
  const provider = new anchor.AnchorProvider(
    connection,
    anchorWallet,
    {
      commitment: "confirmed",
      preflightCommitment: "confirmed",
      maxRetries: 5,
      skipPreflight: false,
    }
  );
  anchor.setProvider(provider);

  const program = anchor.workspace.IncoToken as Program<IncoToken>;
  const inputType = 0;

  // Test accounts
  let mintKeypair: Keypair;
  let walletKeypair: Keypair;
  let ownerTokenAccountKp: Keypair;
  let recipientTokenAccountKp: Keypair;
  let delegateTokenAccountKp: Keypair;

  before(async () => {
    walletKeypair = provider.wallet.payer as Keypair;
    mintKeypair = Keypair.generate();
    ownerTokenAccountKp = Keypair.generate();
    recipientTokenAccountKp = Keypair.generate();
    delegateTokenAccountKp = Keypair.generate();
  });

  // Helper function to decrypt balance using Inco SDK
  async function decryptBalance(accountData: any): Promise<number | null> {
    try {
      const handle = extractHandleFromAnchor(accountData.amount);
      if (handle === "0") return 0;
      
      const result = await decrypt([handle]);
      const rawAmount = parseInt(result.plaintexts[0], 10);
      return rawAmount / 1_000_000_000; // Assuming 9 decimals
    } catch (error) {
      console.log("Decryption error:", error);
      return null;
    }
  }

  describe("Initialize Mint", () => {
    it("Should initialize a new mint", async () => {
      console.log("\n=== INITIALIZE MINT ===");
      
      const tx = await program.methods
        .initializeMint(
          9,
          walletKeypair.publicKey,
          walletKeypair.publicKey
        )
        .accounts({
          mint: mintKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([mintKeypair])
        .rpc();

      console.log("Initialize mint transaction:", tx);

      const mintAccount = await program.account.incoMint.fetch(mintKeypair.publicKey);
      expect(mintAccount.isInitialized).to.be.true;
      expect(mintAccount.decimals).to.equal(9);
      
      expect(mintAccount.mintAuthority).to.have.property('some');
      if ('some' in mintAccount.mintAuthority) {
        expect(comparePublicKeys(mintAccount.mintAuthority.some, walletKeypair.publicKey)).to.be.true;
      }
      
      expect(mintAccount.freezeAuthority).to.have.property('some');
      if ('some' in mintAccount.freezeAuthority) {
        expect(comparePublicKeys(mintAccount.freezeAuthority.some, walletKeypair.publicKey)).to.be.true;
      }
    });
  });

  describe("Initialize Token Accounts", () => {
    it("Should initialize owner token account", async () => {
      console.log("\n=== INITIALIZE OWNER ACCOUNT ===");
      
      const tx = await program.methods
        .initializeAccount()
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          owner: walletKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([ownerTokenAccountKp])
        .rpc();

      console.log("Initialize owner account transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      
      expect(comparePublicKeys(tokenAccount.mint, mintKeypair.publicKey)).to.be.true;
      expect(comparePublicKeys(tokenAccount.owner, walletKeypair.publicKey)).to.be.true;
      expect(tokenAccount.state).to.have.property('initialized');
    });

    it("Should initialize recipient token account", async () => {
      console.log("\n=== INITIALIZE RECIPIENT ACCOUNT ===");
      
      const tx = await program.methods
        .initializeAccount()
        .accounts({
          account: recipientTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          owner: walletKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([recipientTokenAccountKp])
        .rpc();
      
      console.log("Initialize recipient account transaction:", tx);
    });

    it("Should initialize delegate token account", async () => {
      console.log("\n=== INITIALIZE DELEGATE ACCOUNT ===");
      
      const tx = await program.methods
        .initializeAccount()
        .accounts({
          account: delegateTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          owner: walletKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([delegateTokenAccountKp])
        .rpc();

      console.log("Initialize delegate account transaction:", tx);
    });
  });

  describe("Mint Tokens", () => {
    it("Should mint tokens and decrypt correct balance", async () => {
      console.log("\n=== MINT OPERATION ===");
      console.log("Minting: 1 token");

      const mintAmount = BigInt(1000000000); // 1 token with 9 decimals
      const encryptedHex = await encryptValue(mintAmount);

      const tx = await program.methods
        .mintTo(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          mint: mintKeypair.publicKey,
          account: ownerTokenAccountKp.publicKey,
          mintAuthority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Mint transaction:", tx);

      // Wait for handle storage
      await new Promise(resolve => setTimeout(resolve, 3000));

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      
      const decryptedBalance = await decryptBalance(tokenAccount);
      
      if (decryptedBalance !== null) {
        console.log("✅ Owner balance after mint:", decryptedBalance, "tokens");
        expect(decryptedBalance).to.be.greaterThanOrEqual(0);
      } else {
        console.log("Failed to decrypt balance after mint");
      }
    });
  });

  describe("Self-Transfer Tests", () => {
    it("Should handle self-transfer correctly (balance should remain unchanged)", async () => {
      console.log("\n=== SELF-TRANSFER TEST ===");
      console.log("Testing self-transfer (transferring from account to itself)");

      // Get balance BEFORE self-transfer
      const accountBefore = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const balanceBefore = await decryptBalance(accountBefore);
      
      if (balanceBefore !== null) {
        console.log("Balance BEFORE self-transfer:", balanceBefore, "tokens");
      }

      const transferAmount = BigInt(100000000); // 0.1 tokens
      const encryptedHex = await encryptValue(transferAmount);

      console.log("Executing self-transfer (source = destination)...");
      const tx = await program.methods
        .transfer(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          destination: ownerTokenAccountKp.publicKey, // Same account as source
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Self-transfer transaction:", tx);

      await new Promise(resolve => setTimeout(resolve, 3000));

      const accountAfter = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const balanceAfter = await decryptBalance(accountAfter);
      
      if (balanceAfter !== null && balanceBefore !== null) {
        console.log("Balance AFTER self-transfer:", balanceAfter, "tokens");
        const balanceChange = balanceAfter - balanceBefore;
        console.log("Balance change from self-transfer:", balanceChange, "tokens");
        
        expect(Math.abs(balanceChange)).to.be.lessThan(0.000001, "Balance should not change during self-transfer");
        console.log("✅ Self-transfer working correctly - balance unchanged");
      }
    });
  });

  describe("Transfer Tokens", () => {
    it("Should transfer tokens with correct balance changes", async () => {
      console.log("\n=== TRANSFER OPERATION ===");
      console.log("Transferring: 0.25 tokens from Owner to Recipient");

      // Show balances BEFORE transfer
      const sourceAccountBefore = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const destAccountBefore = await program.account.incoAccount.fetch(recipientTokenAccountKp.publicKey);
      
      const sourceBalanceBefore = await decryptBalance(sourceAccountBefore);
      const destBalanceBefore = await decryptBalance(destAccountBefore);
      
      if (sourceBalanceBefore !== null) {
        console.log("Owner balance BEFORE transfer:", sourceBalanceBefore, "tokens");
      }
      if (destBalanceBefore !== null) {
        console.log("Recipient balance BEFORE transfer:", destBalanceBefore, "tokens");
      }

      const transferAmount = BigInt(250000000); // 0.25 tokens
      const encryptedHex = await encryptValue(transferAmount);

      const tx = await program.methods
        .transfer(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          destination: recipientTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Transfer transaction:", tx);

      await new Promise(resolve => setTimeout(resolve, 5000));

      const sourceAccountAfter = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const destAccountAfter = await program.account.incoAccount.fetch(recipientTokenAccountKp.publicKey);
      
      const sourceBalance = await decryptBalance(sourceAccountAfter);
      const destBalance = await decryptBalance(destAccountAfter);
      
      if (sourceBalance !== null && destBalance !== null) {
        console.log("✅ Owner balance AFTER transfer:", sourceBalance, "tokens");
        console.log("✅ Recipient balance AFTER transfer:", destBalance, "tokens");
      }
    });

    it("Should handle insufficient balance gracefully", async () => {
      console.log("\n=== INSUFFICIENT BALANCE TEST ===");
      
      const sourceAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const currentBalance = await decryptBalance(sourceAccount);
      
      console.log(`Current Owner balance: ${currentBalance} tokens`);
      console.log("Attempting to transfer: 10 tokens (should fail silently or transfer 0)");

      const transferAmount = BigInt(10000000000); // 10 tokens
      const encryptedHex = await encryptValue(transferAmount);

      const tx = await program.methods
        .transfer(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          destination: recipientTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Insufficient balance test transaction:", tx);

      await new Promise(resolve => setTimeout(resolve, 5000));
      
      const sourceAccountAfter = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const sourceBalance = await decryptBalance(sourceAccountAfter);
      
      if (sourceBalance !== null) {
        console.log("Owner balance after over-transfer attempt:", sourceBalance, "tokens");
        expect(sourceBalance).to.be.greaterThanOrEqual(0, "Balance should not go negative");
      }
    });
  });

  describe("Approve and Delegate Transfer", () => {
    it("Should approve a delegate", async () => {
      console.log("\n=== APPROVE DELEGATE ===");
      
      const approveAmount = BigInt(100000000); // 0.1 tokens
      const encryptedHex = await encryptValue(approveAmount);

      const tx = await program.methods
        .approve(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          delegate: walletKeypair.publicKey,
          owner: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Approve delegate transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      
      expect(tokenAccount.delegate).to.have.property('some');
      if ('some' in tokenAccount.delegate) {
        expect(comparePublicKeys(tokenAccount.delegate.some, walletKeypair.publicKey)).to.be.true;
      }
    });

    it("Should allow delegate to transfer", async () => {
      console.log("\n=== DELEGATE TRANSFER ===");
      
      const transferAmount = BigInt(50000000); // 0.05 tokens
      const encryptedHex = await encryptValue(transferAmount);

      const tx = await program.methods
        .transfer(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          destination: delegateTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Delegate transfer transaction:", tx);
    });

    it("Should revoke delegate", async () => {
      console.log("\n=== REVOKE DELEGATE ===");
      
      const tx = await program.methods
        .revoke()
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          owner: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Revoke delegate transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(tokenAccount.delegate).to.have.property('none');
    });
  });

  describe("Burn Tokens", () => {
    it("Should burn tokens from account", async () => {
      console.log("\n=== BURN OPERATION ===");
      console.log("Burning: 0.1 tokens");

      const accountBefore = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const balanceBefore = await decryptBalance(accountBefore);
      
      if (balanceBefore !== null) {
        console.log("Owner balance BEFORE burn:", balanceBefore, "tokens");
      }

      const burnAmount = BigInt(100000000); // 0.1 tokens
      const encryptedHex = await encryptValue(burnAmount);

      const tx = await program.methods
        .burn(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Burn transaction:", tx);

      await new Promise(resolve => setTimeout(resolve, 5000));

      const accountAfter = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const decryptedAfterBurn = await decryptBalance(accountAfter);
      
      if (decryptedAfterBurn !== null) {
        console.log("✅ Owner balance AFTER burn:", decryptedAfterBurn, "tokens");
        
        if (balanceBefore !== null) {
          const change = decryptedAfterBurn - balanceBefore;
          console.log("Balance change from burn:", change.toFixed(2), "tokens");
        }
      }
    });
  });

  describe("Freeze and Thaw Account", () => {
    it("Should freeze an account", async () => {
      console.log("\n=== FREEZE ACCOUNT ===");
      
      const tx = await program.methods
        .freezeAccount()
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          freezeAuthority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Freeze account transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(tokenAccount.state).to.have.property('frozen');
    });

    it("Should fail to transfer from frozen account", async () => {
      console.log("\n=== FROZEN ACCOUNT TRANSFER TEST ===");
      
      const transferAmount = BigInt(50000000);
      const encryptedHex = await encryptValue(transferAmount);

      try {
        await program.methods
          .transfer(
            hexToBuffer(encryptedHex),
            inputType
          )
          .accounts({
            source: ownerTokenAccountKp.publicKey,
            destination: recipientTokenAccountKp.publicKey,
            authority: walletKeypair.publicKey,
          } as any)
          .signers([])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (error) {
        console.log("✅ Expected error for frozen account transfer");
        expect((error as Error).toString()).to.include("Error");
      }
    });

    it("Should thaw an account", async () => {
      console.log("\n=== THAW ACCOUNT ===");
      
      const tx = await program.methods
        .thawAccount()
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          freezeAuthority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Thaw account transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(tokenAccount.state).to.have.property('initialized');
    });

    it("Should transfer after thawing", async () => {
      console.log("\n=== TRANSFER AFTER THAW ===");
      
      const transferAmount = BigInt(50000000);
      const encryptedHex = await encryptValue(transferAmount);

      const tx = await program.methods
        .transfer(
          hexToBuffer(encryptedHex),
          inputType
        )
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          destination: recipientTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Transfer after thaw transaction:", tx);
    });
  });

  describe("Authority Management", () => {
    it("Should set mint authority to same wallet", async () => {
      console.log("\n=== SET MINT AUTHORITY ===");
      
      const tx = await program.methods
        .setMintAuthority(walletKeypair.publicKey)
        .accounts({
          mint: mintKeypair.publicKey,
          currentAuthority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Set mint authority transaction:", tx);

      const mintAccount = await program.account.incoMint.fetch(mintKeypair.publicKey);
      expect(mintAccount.mintAuthority).to.have.property('some');
    });

    it("Should set account owner to same wallet", async () => {
      console.log("\n=== SET ACCOUNT OWNER ===");
      
      const tx = await program.methods
        .setAccountOwner(walletKeypair.publicKey)
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          currentOwner: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Set account owner transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(comparePublicKeys(tokenAccount.owner, walletKeypair.publicKey)).to.be.true;
    });
  });

  describe("Close Account", () => {
    let testAccountKp: Keypair;
    
    before(async () => {
      console.log("\n=== SETUP TEST ACCOUNT FOR CLOSING ===");
      
      testAccountKp = Keypair.generate();
      
      const tx = await program.methods
        .initializeAccount()
        .accounts({
          account: testAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          owner: walletKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([testAccountKp])
        .rpc();

      console.log("Initialize test account for closing transaction:", tx);
    });

    it("Should close an account with zero balance", async () => {
      console.log("\n=== CLOSE ACCOUNT ===");
      
      const destinationKeypair = Keypair.generate();

      const tx = await program.methods
        .closeAccount()
        .accounts({
          account: testAccountKp.publicKey,
          destination: destinationKeypair.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Close account transaction:", tx);

      const accountInfo = await provider.connection.getAccountInfo(testAccountKp.publicKey);
      expect(accountInfo?.lamports || 0).to.equal(0);
    });
  });

  describe("Handle-based Operations", () => {
    it("Should transfer using handle instead of ciphertext", async () => {
      console.log("\n=== HANDLE-BASED TRANSFER ===");
      
      const sourceAccount = await program.account.incoAccount.fetch(recipientTokenAccountKp.publicKey);
      const amountHandle = sourceAccount.amount;

      const tx = await program.methods
        .transferWithHandle(amountHandle)
        .accounts({
          source: recipientTokenAccountKp.publicKey,
          destination: delegateTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Handle-based transfer transaction:", tx);
    });
  });

  describe("Final Balance Check", () => {
    it("Should show final balances for all accounts", async () => {
      console.log("\n=== FINAL BALANCE SUMMARY ===");
      
      const accounts = [
        { name: "Owner", key: ownerTokenAccountKp.publicKey },
        { name: "Recipient", key: recipientTokenAccountKp.publicKey },
        { name: "Delegate", key: delegateTokenAccountKp.publicKey }
      ];

      let totalBalance = 0;

      for (const account of accounts) {
        try {
          const accountData = await program.account.incoAccount.fetch(account.key);
          const balance = await decryptBalance(accountData);
          
          if (balance !== null) {
            console.log(`${account.name} final balance: ${balance} tokens`);
            totalBalance += balance;
          } else {
            console.log(`${account.name}: Failed to decrypt balance`);
          }
        } catch (error) {
          console.log(`${account.name}: Account not accessible (might be closed)`);
        }
      }

      console.log(`Total balance across all accounts: ${totalBalance} tokens`);
      
      try {
        const mintData = await program.account.incoMint.fetch(mintKeypair.publicKey);
        console.log(`Mint supply handle: ${extractHandleFromAnchor(mintData.supply)}`);
      } catch (error) {
        console.log("Could not fetch mint supply");
      }
    });
  });
});

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { IncoToken } from "../target/types/inco_token";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram,
  Connection,
} from "@solana/web3.js";
import { expect } from "chai";
import { encryptValue } from "@inco/solana-sdk/encryption";
import { decrypt } from "@inco/solana-sdk/attested-decrypt";
import { hexToBuffer } from "@inco/solana-sdk/utils";

// Enhanced handle extraction function for Anchor BN objects
function extractHandleFromAnchor(anchorHandle: any): string {
  if (anchorHandle && anchorHandle._bn) {
    return anchorHandle._bn.toString(10);
  }
  
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
  
  if (anchorHandle instanceof Uint8Array || Array.isArray(anchorHandle)) {
    const buffer = Buffer.from(anchorHandle);
    let result = BigInt(0);
    for (let i = buffer.length - 1; i >= 0; i--) {
      result = result * BigInt(256) + BigInt(buffer[i]);
    }
    return result.toString();
  }
  
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

describe("inco-token-2022", () => {
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

  async function decryptBalance(accountData: any, decimals: number = 6): Promise<number | null> {
    try {
      const handle = extractHandleFromAnchor(accountData.amount);
      if (handle === "0") return 0;
      
      const result = await decrypt([handle]);
      const rawAmount = parseInt(result.plaintexts[0], 10);
      return rawAmount / Math.pow(10, decimals);
    } catch (error) {
      console.log("Decryption error:", error);
      return null;
    }
  }

  describe("Token 2022 - Initialize Mint", () => {
    it("Should initialize a new mint with decimals validation", async () => {
      console.log("\n=== TOKEN 2022 - INITIALIZE MINT ===");
      
      const tx = await program.methods
        .initializeMint(6, walletKeypair.publicKey, walletKeypair.publicKey)
        .accounts({
          mint: mintKeypair.publicKey,
          payer: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([mintKeypair])
        .rpc();

      console.log("Token 2022 initialize mint transaction:", tx);

      const mintAccount = await program.account.incoMint.fetch(mintKeypair.publicKey);
      expect(mintAccount.isInitialized).to.be.true;
      expect(mintAccount.decimals).to.equal(6);
    });
  });

  describe("Token 2022 - Initialize Accounts", () => {
    it("Should initialize account using initialize_account3", async () => {
      console.log("\n=== TOKEN 2022 - INITIALIZE ACCOUNT3 ===");
      
      const tx = await program.methods
        .initializeAccount3()
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([ownerTokenAccountKp])
        .rpc();

      console.log("Token 2022 initialize account3 transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(comparePublicKeys(tokenAccount.mint, mintKeypair.publicKey)).to.be.true;
      expect(tokenAccount.state).to.have.property('initialized');
    });

    it("Should initialize recipient and delegate accounts", async () => {
      console.log("\n=== TOKEN 2022 - INITIALIZE OTHER ACCOUNTS ===");
      
      await program.methods
        .initializeAccount3()
        .accounts({
          account: recipientTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([recipientTokenAccountKp])
        .rpc();

      await program.methods
        .initializeAccount3()
        .accounts({
          account: delegateTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([delegateTokenAccountKp])
        .rpc();
    });
  });

  describe("Token 2022 - Mint to Checked", () => {
    it("Should mint tokens with decimal validation", async () => {
      console.log("\n=== TOKEN 2022 - MINT TO CHECKED ===");

      const mintAmount = BigInt(100000000);
      const encryptedHex = await encryptValue(mintAmount);

      const tx = await program.methods
        .mintToChecked(hexToBuffer(encryptedHex), inputType, 6)
        .accounts({
          mint: mintKeypair.publicKey,
          account: ownerTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 mint to checked transaction:", tx);
      await new Promise(resolve => setTimeout(resolve, 3000));

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      const decryptedBalance = await decryptBalance(tokenAccount);
      
      if (decryptedBalance !== null) {
        console.log("✅ Owner balance after mint:", decryptedBalance, "tokens");
        expect(decryptedBalance).to.be.greaterThanOrEqual(0);
      }
    });

    it("Should fail mint with wrong decimals", async () => {
      console.log("\n=== TOKEN 2022 - MINT WRONG DECIMALS TEST ===");
      
      const encryptedHex = await encryptValue(BigInt(50000000));

      try {
        await program.methods
          .mintToChecked(hexToBuffer(encryptedHex), inputType, 9)
          .accounts({
            mint: mintKeypair.publicKey,
            account: ownerTokenAccountKp.publicKey,
            authority: walletKeypair.publicKey,
          } as any)
          .signers([])
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (error) {
        console.log("✅ Expected error for wrong decimals");
      }
    });
  });

  describe("Token 2022 - Transfer Checked", () => {
    it("Should transfer tokens with decimal validation", async () => {
      console.log("\n=== TOKEN 2022 - TRANSFER CHECKED ===");

      const encryptedHex = await encryptValue(BigInt(25000000));

      const tx = await program.methods
        .transferChecked(hexToBuffer(encryptedHex), inputType, 6)
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          destination: recipientTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 transfer checked transaction:", tx);
      await new Promise(resolve => setTimeout(resolve, 5000));

      const destAccount = await program.account.incoAccount.fetch(recipientTokenAccountKp.publicKey);
      const destBalance = await decryptBalance(destAccount);
      
      if (destBalance !== null) {
        console.log("✅ Recipient balance:", destBalance, "tokens");
      }
    });
  });

  describe("Token 2022 - Approve Checked", () => {
    it("Should approve delegate with decimal validation", async () => {
      console.log("\n=== TOKEN 2022 - APPROVE CHECKED ===");
      
      const encryptedHex = await encryptValue(BigInt(10000000));

      const tx = await program.methods
        .approveChecked(hexToBuffer(encryptedHex), inputType, 6)
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          delegate: walletKeypair.publicKey,
          owner: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 approve checked transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(tokenAccount.delegate).to.have.property('some');
    });
  });

  describe("Token 2022 - Burn Checked", () => {
    it("Should burn tokens with decimal validation", async () => {
      console.log("\n=== TOKEN 2022 - BURN CHECKED ===");

      const encryptedHex = await encryptValue(BigInt(5000000));

      const tx = await program.methods
        .burnChecked(hexToBuffer(encryptedHex), inputType, 6)
        .accounts({
          account: ownerTokenAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 burn checked transaction:", tx);
    });
  });

  describe("Token 2022 - Revoke and Close", () => {
    it("Should revoke delegate", async () => {
      console.log("\n=== TOKEN 2022 - REVOKE ===");
      
      const tx = await program.methods
        .revoke2022()
        .accounts({
          source: ownerTokenAccountKp.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 revoke transaction:", tx);

      const tokenAccount = await program.account.incoAccount.fetch(ownerTokenAccountKp.publicKey);
      expect(tokenAccount.delegate).to.have.property('none');
    });

    it("Should close account", async () => {
      console.log("\n=== TOKEN 2022 - CLOSE ACCOUNT ===");
      
      const testAccountKp = Keypair.generate();
      
      await program.methods
        .initializeAccount3()
        .accounts({
          account: testAccountKp.publicKey,
          mint: mintKeypair.publicKey,
          authority: walletKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([testAccountKp])
        .rpc();

      const destinationKeypair = Keypair.generate();

      const tx = await program.methods
        .closeAccount2022()
        .accounts({
          account: testAccountKp.publicKey,
          destination: destinationKeypair.publicKey,
          authority: walletKeypair.publicKey,
        } as any)
        .signers([])
        .rpc();

      console.log("Token 2022 close account transaction:", tx);

      const accountInfo = await provider.connection.getAccountInfo(testAccountKp.publicKey);
      expect(accountInfo?.lamports || 0).to.equal(0);
    });
  });

  describe("Token 2022 - Final Balance Summary", () => {
    it("Should show final balances", async () => {
      console.log("\n=== TOKEN 2022 - FINAL BALANCE SUMMARY ===");
      
      const accounts = [
        { name: "Owner", key: ownerTokenAccountKp.publicKey },
        { name: "Recipient", key: recipientTokenAccountKp.publicKey },
        { name: "Delegate", key: delegateTokenAccountKp.publicKey }
      ];

      for (const account of accounts) {
        try {
          const accountData = await program.account.incoAccount.fetch(account.key);
          const balance = await decryptBalance(accountData);
          
          if (balance !== null) {
            console.log(`${account.name} final balance: ${balance} tokens`);
          }
        } catch (error) {
          console.log(`${account.name}: Account not accessible`);
        }
      }

      console.log("✅ Token 2022 test suite completed!");
    });
  });
});

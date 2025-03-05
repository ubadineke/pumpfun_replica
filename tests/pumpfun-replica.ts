import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PumpfunReplica } from "../target/types/pumpfun_replica";
import { assert, expect } from "chai";
import {
  AuthorityType,
  createMint,
  getOrCreateAssociatedTokenAccount,
  MintLayout,
  setAuthority,
} from "@solana/spl-token";
import { getAccount } from "@solana/spl-token";

describe("pumpfun-replica", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  const program = anchor.workspace.PumpfunReplica as Program<PumpfunReplica>;

  const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
  interface globalSettingsInput {
    initialVirtualTokenReserves: anchor.BN;
    initialVirtualSolReserves: anchor.BN;
    initialRealTokenReserves: anchor.BN;
    tokenTotalSupply: anchor.BN;
    mintDecimals: number;
    migrateFeeAmount: anchor.BN;
    migrationTokenAllocation: anchor.BN;
    feeReceiver: PublicKey;
    lpConfig: PublicKey;
  }

  let globalPDA;
  let bondingCurvePDA;
  let bondingCurveSolEscrowPDA;
  let metadataPDA;
  let bondingCurveTokenAccount;
  let userTokenAccount;
  // let FEE_RECEIVER = new PublicKey("Bf8PxxWt7UTvNGcrDyNwQiERSwNroa4pEo1pxwKo17Uh");
  let FEE_RECEIVER = anchor.web3.Keypair.generate();
  let admin = anchor.web3.Keypair.generate();
  let creator1 = anchor.web3.Keypair.generate();
  let user1 = anchor.web3.Keypair.generate();

  console.log("Admin Publickey", admin.publicKey.toString());
  console.log("Creator Publickey", creator1.publicKey.toString());
  let tokenMint1: PublicKey;
  let SOL_FOR_BUY = new anchor.BN(0.2 * 1_000_000_000);
  let TOKEN_FOR_SELL = new anchor.BN(5000000000000);

  before(async () => {
    //Airdrop SOL
    async function airdropSOL(publicKey: PublicKey, amount_in_sol: number) {
      const signature = await provider.connection.requestAirdrop(
        publicKey,
        amount_in_sol * 1000000000 //convert to lamports
      );
      await provider.connection.confirmTransaction(signature, "confirmed");
    }

    function derivePDA(seeds: (string | PublicKey | number | Buffer)[]): PublicKey {
      const seedBuffers = seeds.map((seed) => {
        if (typeof seed == "string") {
          return Buffer.from(seed);
        } else if (seed instanceof PublicKey) {
          return seed.toBuffer();
        } else if (typeof seed == "number") {
          return Buffer.from(Uint8Array.of(seed));
        } else if (Buffer.isBuffer(seed)) {
          return seed;
        } else {
          throw new Error(
            `Invalid seed type: ${typeof seed}. Expected string, PublicKey, Buffer, or number.`
          );
        }
      });

      const [derivedPDA] = PublicKey.findProgramAddressSync(seedBuffers, program.programId);

      return derivedPDA;
    }

    //DERIVE GLOBAL SETTING
    globalPDA = derivePDA(["global"]);
    console.log("Global PDA", globalPDA);

    //AIRDROP ACCOUNTS
    await airdropSOL(admin.publicKey, 1);
    await airdropSOL(creator1.publicKey, 1);
    await airdropSOL(user1.publicKey, 5);

    //CREATE TOKEN MINT 1
    tokenMint1 = await createMint(provider.connection, creator1, creator1.publicKey, null, 6);
    console.log("token mint", tokenMint1.toString());

    //DERIVE BONDING CURVE PDA
    bondingCurvePDA = derivePDA(["bonding-curve", tokenMint1]);
    console.log("Bonding Curve PDA", bondingCurvePDA);

    //DERIVE BONDING CURVE SOL ESCROW PDA
    bondingCurveSolEscrowPDA = derivePDA(["sol-escrow", tokenMint1]);
    console.log("bonding curve sol escrow", bondingCurveSolEscrowPDA);

    //GET OR CREATE BONDING CURVE TOKEN ACCOUNT
    bondingCurveTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      creator1,
      tokenMint1,
      bondingCurvePDA,
      true
    );
    // console.log("Bonding Curve Token Account", bondingCurveTokenAccount);

    //DERIVE USER TOKEN ACCOUNT
    userTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user1,
      tokenMint1,
      user1.publicKey,
      true
    );
    // console.log("User Token Account", userTokenAccount);

    const initialTokenAccountInfo = await getAccount(provider.connection, userTokenAccount.address);
    console.log("INITIAL USER TOKEN ACCOUNT INFO", initialTokenAccountInfo.amount.toString());
    //DERIVE METADATA PDA
    // metadataPDA = derivePDA(["metadata", TOKEN_METADATA_PROGRAM_ID, tokenMint1]);
    [metadataPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), tokenMint1.toBuffer()],
      TOKEN_METADATA_PROGRAM_ID
    );
    console.log("Metadata PDA", metadataPDA);

    //CHANGE TOKEN MINT AUTHORITY
    await setAuthority(
      provider.connection,
      creator1,
      tokenMint1,
      creator1,
      AuthorityType.MintTokens,
      bondingCurvePDA
    );

    let mintAccountInfo = await provider.connection.getAccountInfo(tokenMint1);
    // console.log("Mint account info", mintAccountInfo.data);
    const mintData = MintLayout.decode(mintAccountInfo.data);
    console.log("Mint information", mintData);
  });

  it("Is initialized!", async () => {
    // Add your test here.
    let initializeParams: globalSettingsInput = {
      initialVirtualTokenReserves: new anchor.BN(1073000000000000),
      initialVirtualSolReserves: new anchor.BN(30 * 1_000_000_000),
      initialRealTokenReserves: new anchor.BN(793100000000000),
      tokenTotalSupply: new anchor.BN(1000000000000000),
      mintDecimals: 6,
      migrateFeeAmount: new anchor.BN(500),
      migrationTokenAllocation: new anchor.BN(50000000000000),
      feeReceiver: FEE_RECEIVER.publicKey,
      lpConfig: PublicKey.default,
    };

    const tx = await program.methods
      .initialize(initializeParams)
      .accounts({
        authority: admin.publicKey,
        global: globalPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const state = await program.account.global.fetch(globalPDA);
    assert.strictEqual(state.tokenTotalSupply.toNumber(), 1000000000000000);
    assert.strictEqual(state.feeReceiver.toString(), FEE_RECEIVER.publicKey.toString());
    assert.strictEqual(state.initialized, true);
    console.log("Your transaction signature", tx);
  });

  it("Bonding Curve Created", async () => {
    let bondingCurveParams = {
      name: "Biboshi",
      symbol: "BSH",
      uri: "https://gateway.pinata.cloud/ipfs/bafkreig2zeo4l3suy3tlaqzhw4u5zsdkmmq7nnjfkbpt3qfjycna5hrgxm",
    };

    const tx = await program.methods
      .createBondingCurve(bondingCurveParams)
      .accounts({
        mint: tokenMint1,
        creator: creator1.publicKey,
        bondingCurve: bondingCurvePDA,
        bondingCurveTokenAccount: bondingCurveTokenAccount.address,
        bondingCurveSolEscrow: bondingCurveSolEscrowPDA,
        global: globalPDA,
        metadata: metadataPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([creator1])
      .rpc();

    const bonding_curve = await program.account.bondingCurve.fetch(bondingCurvePDA);
    assert.strictEqual(bonding_curve.realSolReserves.toNumber(), 0);
    assert.strictEqual(bonding_curve.complete, false);
    assert.strictEqual(bonding_curve.virtualTokenReserves.toNumber(), 1073000000000000);
  });

  it("Buy and sell from/to bonding curve", async () => {
    const tx = await program.methods
      .buy(SOL_FOR_BUY)
      .accounts({
        user: user1.publicKey,
        global: globalPDA,
        feeReceiver: FEE_RECEIVER.publicKey,
        mint: tokenMint1,
        bondingCurve: bondingCurvePDA,
        bondingCurveTokenAccount: bondingCurveTokenAccount.address,
        bondingCurveSolEscrow: bondingCurveSolEscrowPDA,
        userTokenAccount: userTokenAccount.address,
        systemProgram: SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([user1])
      .rpc();

    const userTokenAccountInfo = await getAccount(provider.connection, userTokenAccount.address);
    const bonding_curve = await program.account.bondingCurve.fetch(bondingCurvePDA);
    const globalState = await program.account.global.fetch(globalPDA);
    const feeReceiverBalance = await provider.connection.getBalance(FEE_RECEIVER.publicKey);
    const userBalanceAfterBuy = await provider.connection.getBalance(user1.publicKey);

    //EQUIVALENT FOR FIRST BUY ON CURVE USING 0.2 SOL
    assert.strictEqual(userTokenAccountInfo.amount.toString(), "7105960264900");
    assert.strictEqual(bonding_curve.realSolReserves.toNumber(), 200000000);
    assert.strictEqual(
      bonding_curve.virtualTokenReserves.toNumber(),
      globalState.initialVirtualTokenReserves.toNumber() - 7105960264900
    );
    assert.strictEqual(feeReceiverBalance, 1000000);
    // await provider.connection.getBalance(user1.publicKey);

    // console.log(
    //   `User Balance after everything",
    //   ${(await provider.connection.getBalance(user1.publicKey)) / 1000000000} SOL`
    // );

    const tx2 = await program.methods
      .sell(TOKEN_FOR_SELL)
      .accounts({
        user: user1.publicKey,
        global: globalPDA,
        feeReceiver: FEE_RECEIVER.publicKey,
        mint: tokenMint1,
        bondingCurve: bondingCurvePDA,
        bondingCurveTokenAccount: bondingCurveTokenAccount.address,
        bondingCurveSolEscrow: bondingCurveSolEscrowPDA,
        userTokenAccount: userTokenAccount.address,
        systemProgram: SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([user1])
      .rpc();

    const userTokenAccountInfoAfterSell = await getAccount(
      provider.connection,
      userTokenAccount.address
    );
    const bondingCurveAfterSell = await program.account.bondingCurve.fetch(bondingCurvePDA);
    const userBalanceAfterSell = await provider.connection.getBalance(user1.publicKey);

    console.log(
      `Bonding curve after sell ,
      ${bondingCurveAfterSell.realSolReserves.toNumber() / 1000000000} SOL`
    );
    // console.log("Token Amount", userTokenAccountInfo2.amount.toString());

    assert.strictEqual(userTokenAccountInfoAfterSell.amount.toString(), "2105960264900");
    expect(userBalanceAfterSell).to.be.greaterThan(userBalanceAfterBuy);
  });
});

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PumpfunReplica } from "../target/types/pumpfun_replica";
import { assert } from "chai";
import {
  AuthorityType,
  createMint,
  getOrCreateAssociatedTokenAccount,
  MintLayout,
  setAuthority,
} from "@solana/spl-token";
import { findMetadataPda } from "@metaplex-foundation/mpl-token-metadata";

import { glob } from "fs";

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
  let FEE_RECEIVER = new PublicKey("Bf8PxxWt7UTvNGcrDyNwQiERSwNroa4pEo1pxwKo17Uh");
  let admin = anchor.web3.Keypair.generate();
  let creator1 = anchor.web3.Keypair.generate();
  console.log("Admin Publickey", admin.publicKey.toString());
  console.log("Creator Publickey", creator1.publicKey.toString());
  let tokenMint1: PublicKey;

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
    // [globalPDA] = PublicKey.findProgramAddressSync(
    //   [Buffer.from("global")],
    //   program.programId
    // );

    globalPDA = derivePDA(["global"]);
    console.log("Global PDA", globalPDA);

    //AIRDROP ACCOUNTS
    await airdropSOL(admin.publicKey, 1);
    await airdropSOL(creator1.publicKey, 1);

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
    console.log("Bonding Curve Token Account", bondingCurveTokenAccount);

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
      feeReceiver: FEE_RECEIVER,
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
    assert.strictEqual(state.feeReceiver.toString(), FEE_RECEIVER.toString());
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
});

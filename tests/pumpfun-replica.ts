import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PumpfunReplica } from "../target/types/pumpfun_replica";
import { assert } from "chai";

describe("pumpfun-replica", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  const program = anchor.workspace.PumpfunReplica as Program<PumpfunReplica>;

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
  let FEE_RECEIVER = new PublicKey("Bf8PxxWt7UTvNGcrDyNwQiERSwNroa4pEo1pxwKo17Uh")
  let creator1 = anchor.web3.Keypair.generate();

  before(async () => {
    //Airdrop SOL 
    async function airdropSOL(publicKey: PublicKey, amount_in_sol: number) {
      const signature = await provider.connection.requestAirdrop(
        publicKey,
        amount_in_sol * 1000000000 //convert to lamports
      );
      await provider.connection.confirmTransaction(signature, "confirmed");
    }

    //DERIVE GLOBAL SETTING 
    [globalPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    );

    await airdropSOL(creator1.publicKey, 1)

  })

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

    const tx = await program.methods.initialize(initializeParams)
      .accounts({
        authority: creator1.publicKey,
        global: globalPDA.publicKey,
        systemProgram: SystemProgram.programId
      })
      .signers([creator1])
      .rpc();

    const state = await program.account.global.fetch(globalPDA);
    assert.strictEqual(state.tokenTotalSupply.toNumber(), 1000000000000000)
    assert.strictEqual(state.feeReceiver.toString(), FEE_RECEIVER.toString())
    assert.strictEqual(state.initialized, true)
    console.log("Your transaction signature", tx);
  });
});

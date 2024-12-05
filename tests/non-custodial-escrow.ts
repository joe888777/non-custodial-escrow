import * as anchor from "@coral-xyz/anchor";
import * as splToken from "@solana/spl-token";
import { Program } from "@coral-xyz/anchor";
import { NonCustodialEscrow } from "../target/types/non_custodial_escrow";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  Connection,
} from '@solana/web3.js';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

describe("non-custodial-escrow", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  const program = anchor.workspace.NonCustodialEscrow as Program<NonCustodialEscrow>;

  const seller = provider.wallet.publicKey; // account who sell x to get y
  const wallet = (provider.wallet as NodeWallet).payer;
  const payer = wallet; //fee payer

  const buyer = anchor.web3.Keypair.generate(); //account who use y to buy x
  const escrowedXTokens = anchor.web3.Keypair.generate(); //account store X token in escrow

  let x_mint; //x mint account
  let y_mint; //y mint account
  let sellers_x_token; // ATA seller & mintX
  let sellers_y_token; // ATA seller & mintY
  let buyer_x_token; // ATA buyer & mintX
  let buyer_y_token; // ATA buyer & mintY
  let escrow = anchor.web3.PublicKey;

  let connection = provider.connection;

  console.log("connection", connection);
  before(async () => {
    await provider.connection.requestAirdrop(buyer.publicKey, 1*LAMPORTS_PER_SOL);
    //get escrow account
    escrow = anchor.web3.PublicKey.findProgramAddressSync([
        anchor.utils.bytes.utf8.encode("escrow"),
        seller.toBuffer()
      ], //seeds
      program.programId
    )[0] as any;

    // create x token mint account
    x_mint = await splToken.createMint(
      connection, //Connection
      payer, //payer
      provider.wallet.publicKey, //mint authority
      provider.wallet.publicKey, //freeze authority
      6, //decimal
      undefined, //keypair
      undefined, //confirm options
      splToken.TOKEN_PROGRAM_ID //program id
    );

    // create y token mint account
    y_mint = await splToken.createMint(
      connection, //Connection
      payer, //payer
      provider.wallet.publicKey, //mint authority
      provider.wallet.publicKey, //freeze authority
      6, //decimal
      undefined, //keypair
      undefined, //confirm options
      splToken.TOKEN_PROGRAM_ID //program id
    );

    //create ATA of seller & mintX
    sellers_x_token = await splToken.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      x_mint,
      seller
    );

    // mint x token to seller
    await splToken.mintTo(
      connection,
      payer,
      x_mint,
      sellers_x_token.address,
      payer,
      10_000_000_000,
      [],
      undefined,
      splToken.TOKEN_PROGRAM_ID
    );

    //create ATA of seller & mintY
    sellers_y_token = await splToken.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      y_mint,
      seller
    );

    //buyer's x and y token account
    // buyer_x_token = await x_mint.createAccount(buyer);
    
    //create ATA of buyer & mintX
    buyer_x_token = await splToken.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      x_mint,
      buyer.publicKey
    );

    //create ATA of buyer & mintY
    buyer_y_token = await splToken.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      y_mint,
      buyer.publicKey
    );

    //mint y token to buyer
    await splToken.mintTo(
      connection,
      payer,
      y_mint,
      buyer_y_token.address,
      payer,
      10_000_000_000,
      [],
      undefined,
      splToken.TOKEN_PROGRAM_ID
    );
  })

  it("Initialize escrow!", async () => {
    // Add your test here.
    const x_amount = new anchor.BN(40);
    const y_amount = new anchor.BN(40);

    const tx = await program.methods.initialize(
      x_amount,
      y_amount
    )
    .accounts({
      seller: seller,
      xMint: x_mint, // x token mint account
      yMint: y_mint, // y token mint account
      sellerXToken: sellers_x_token.address, // ATA of seller and mintX
      escrow: escrow, // escrow
      escrowedXTokens: escrowedXTokens.publicKey, // an account to store X token in escrow
      tokenProgram: splToken.TOKEN_PROGRAM_ID,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: anchor.web3.SystemProgram.programId
    })
    .signers([escrowedXTokens])
    .rpc();
    console.log("Your transaction signature", tx);
  });
  it("Execute the trade", async () => {
    const tx = await program.methods
    .accept()
    .accounts({
      buyer: buyer.publicKey,
      escrow: escrow,
      escrowedXTokens: escrowedXTokens.publicKey,
      sellersYTokens: sellers_y_token.address,
      buyerXToken: buyer_x_token.address,
      buyerYToken: buyer_y_token.address,
      tokenProgram: splToken.TOKEN_PROGRAM_ID
    })
    .signers([buyer])
    .rpc()
  });
  it("Cancel the trade", async () => {
    const tx = await program.methods.cancel()
    .accounts({
      seller: seller,
      escrow: escrow,
      escrowedXTokens: escrowedXTokens.publicKey,
      sellerXToken: sellers_x_token.address,
      tokenProgram: splToken.TOKEN_PROGRAM_ID
    })
    .rpc();
  });
});

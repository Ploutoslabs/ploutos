import { Program } from '@coral-xyz/anchor';
import * as anchor from '@coral-xyz/anchor';
import { PublicKey, SystemProgram, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { createMint, mintTo, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { assert } from 'chai';
import { Ploutoslabs } from '../target/types/ploutoslabs';

describe('ploutoslabs', () => {
    anchor.setProvider(anchor.AnchorProvider.env());

    const program = anchor.workspace.Ploutoslabs as Program<Ploutoslabs>;
    const provider = anchor.AnchorProvider.local();

    it('Is initialized!', async () => {
      const admin = Keypair.generate();
      const feeReceiver = Keypair.generate().publicKey;
      const feeAmount = new anchor.BN(1000000);
      const tokenMint = Keypair.generate().publicKey;
      const reserveAmount = new anchor.BN(10000000);
      const airdropAmount = new anchor.BN(1000);
  
      // Fund the user account to pay for transactions
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(admin.publicKey, 1000000000), // Requesting 1 SOL
        "confirmed"
      );
  
      const [dataAccount, _] = await PublicKey.findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode('PLOUTOS_ROOT'),
          admin.publicKey.toBuffer(),
        ],
        program.programId
      );
  
      await program.rpc.initialize(
        feeReceiver,
        feeAmount,
        tokenMint,
        reserveAmount,
        airdropAmount,
        {
          accounts: {
            data: dataAccount,
            user: admin.publicKey,
            systemProgram: SystemProgram.programId,
          },
          signers: [admin],
        }
      );
  
      const data = await program.account.ploutosData.fetch(dataAccount);
  
      assert.equal(data.adminWallet.toString(), feeReceiver.toString());
      assert.equal(data.feeAmount.toNumber(), feeAmount.toNumber());
      assert.equal(data.tokenMint.toString(), tokenMint.toString());
      assert.equal(data.reserveAmount.toNumber(), reserveAmount.toNumber());
      assert.equal(data.airdropAmount.toNumber(), airdropAmount.toNumber());
      assert.equal(data.initialized, true);
    });

    it('Allows a user to claim an airdrop', async () => {
        // Fund admin and user
        const admin = Keypair.generate();
        const user = Keypair.generate();

        const feeReceiver = Keypair.generate().publicKey;
        const feeAmount = new anchor.BN(1000000);
        const reserveAmount = new anchor.BN(10000000);
        const airdropAmount = new anchor.BN(1000);

        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(admin.publicKey, LAMPORTS_PER_SOL * 10),
            'confirmed',
        );

        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(user.publicKey, LAMPORTS_PER_SOL * 10),
            'confirmed',
        );

        // Create mint
        const tokenMint = await createMint(
            provider.connection,
            admin,
            admin.publicKey,
            null,
            9 // Assuming 9 decimal places for the token.
        );
        
        const [dataAccount, _] = await PublicKey.findProgramAddress(
          [
            anchor.utils.bytes.utf8.encode('PLOUTOS_ROOT'),
            admin.publicKey.toBuffer(),
          ],
          program.programId
        );

        // Create the program's token account which will hold tokens for airdrop
        const airdropSourceAccount = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            admin,
            tokenMint,
            dataAccount,
            true
        );

        // Mint to the program's account
        await mintTo(
            provider.connection,
            admin,
            tokenMint,
            airdropSourceAccount.address,
            admin,
            1000000000 // 10 million tokens, adjust the amount as necessary
        );

        // Create a token account for the user to receive the airdrop
        const userTokenAccount = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            user,
            tokenMint,
            user.publicKey
        );
    
        await program.rpc.initialize(
          feeReceiver,
          feeAmount,
          tokenMint,
          reserveAmount,
          airdropAmount,
          {
            accounts: {
              data: dataAccount,
              user: admin.publicKey,
              systemProgram: SystemProgram.programId,
            },
            signers: [admin],
          }
        );

        // Claim Airdrop
        await program.rpc.claimAirdrop({
            accounts: {
                user: user.publicKey,
                adminWallet: admin.publicKey,
                userTokenAccount: userTokenAccount.address,
                programTokenAccount: airdropSourceAccount.address,
                tokenProgram: TOKEN_PROGRAM_ID,
                airdropData: dataAccount,
                systemProgram: SystemProgram.programId,
            },
            signers: [user],
        });

        // Fetch and assert the new token balance of the user's account
        const userAccountInfo = await provider.connection.getTokenAccountBalance(userTokenAccount.address);
        assert.equal(userAccountInfo.value.amount, '1000'); // Assert user received the correct airdrop amount
    });
});


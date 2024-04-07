use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer, Token};

declare_id!("9J3vvSh8r7TYxRUKKgskGMaexMj1BCVnmm2j8LBGVeS5");

#[program]
pub mod ploutoslabs {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_receiver: Pubkey, fee_amount: u64, token_mint: Pubkey, reserve_amount: u64, airdrop_amount: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        if data.initialized {
            return err!(ErrorCode::AlreadyInitialized);
        }
        data.admin_wallet = fee_receiver;
        data.fee_amount = fee_amount;
        data.token_mint = token_mint;
        data.reserve_amount = reserve_amount;
        data.airdrop_amount = airdrop_amount;

        data.initialized = true;
        Ok(())
    }

    pub fn claim_airdrop(ctx: Context<ClaimAirdrop>) -> Result<()> {
        // Derive the PDA (dataAccount) that is the authority of the token account
        let (data_account_pda, bump_seed) = Pubkey::find_program_address(
            &[
                b"PLOUTOS_ROOT".as_ref(), 
                ctx.accounts.admin_wallet.key().as_ref()
            ], 
            ctx.program_id
        );
    
        // Assert the derived PDA matches the expected `airdrop_data` account
        require!(data_account_pda == ctx.accounts.airdrop_data.to_account_info().key(), ErrorCode::PdaMismatch);
    
        let admin_wallet_key = ctx.accounts.admin_wallet.key();
        let seeds = &[
            b"PLOUTOS_ROOT".as_ref(),
            admin_wallet_key.as_ref(),
            &[bump_seed]
        ];
        let signer = &[&seeds[..]];

    
        // Set up the CPI to the SPL Token program's `Transfer` instruction
        let cpi_accounts = Transfer {
            from: ctx.accounts.program_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.airdrop_data.to_account_info(),
        };
    
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    
        // Perform the transfer
        token::transfer(cpi_ctx, ctx.accounts.airdrop_data.airdrop_amount)?;
    
        Ok(())
    }
    

    pub fn claim_airdropa(ctx: Context<ClaimAirdrop>) -> Result<()> {
        let airdrop_data = &ctx.accounts.airdrop_data;

        let _fee_amount = airdrop_data.fee_amount;
        let claim_amount = airdrop_data.airdrop_amount;

        // // Transfer the fee from the user to the admin wallet
        // **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? -= fee_amount;
        // **ctx.accounts.admin_wallet.to_account_info().try_borrow_mut_lamports()? += fee_amount;

        // Transfer the airdrop SPL token from the program's account to the user's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.program_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.program_token_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_context, claim_amount)?;

        Ok(())
    }
}

#[account]
pub struct PloutosData {
    pub admin_wallet: Pubkey,
    pub fee_amount: u64,
    pub token_mint: Pubkey,
    pub program_token_account: Pubkey,
    pub reserve_amount: u64,
    pub airdrop_amount: u64,
    pub initialized: bool,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer=user, space=9000, seeds=[b"PLOUTOS_ROOT".as_ref(), user.key().as_ref()], bump)]
    pub data: Account<'info, PloutosData>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: this is checked
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimAirdrop<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: The `admin_wallet` is only used for transferring SOL. No further validation needed here.
    #[account(mut)]
    pub admin_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = airdrop_data.token_mint == program_token_account.mint)]
    pub program_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub airdrop_data: Account<'info, PloutosData>,
    pub system_program: Program<'info, System>,
}


#[error_code]
pub enum ErrorCode {
    #[msg("The program has already been initialized")]
    AlreadyInitialized,
    #[msg("Invalid token mint")]
    MintMismatch,
    #[msg("PDA mismatch")]
    PdaMismatch,
}

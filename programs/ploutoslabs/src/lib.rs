use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer, Token};
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::program::invoke;

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

        let clock = Clock::get().unwrap();
        ctx.accounts.user_data.claim_timestamp = clock.unix_timestamp;
        ctx.accounts.user_data.claimed = true;

        let claim_amount = airdrop_amount;
        ctx.accounts.user_data.total_allocation = claim_amount;

        data.initialized = true;
        Ok(())
    }

    pub fn claim_airdrop(ctx: Context<ClaimAirdrop>) -> Result<()> {
        if ctx.accounts.user_data.claimed {
            return Err(ErrorCode::AirdropAlreadyClaimed.into());
        }
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


        let clock = Clock::get().unwrap();
        ctx.accounts.user_data.claim_timestamp = clock.unix_timestamp;
        ctx.accounts.user_data.claimed = true;

        let claim_amount = ctx.accounts.airdrop_data.airdrop_amount ;
        ctx.accounts.user_data.total_allocation = claim_amount;

        let transfer_fee_instruction = system_instruction::transfer(
            ctx.accounts.user.to_account_info().key,
            ctx.accounts.admin_wallet.key,
            ctx.accounts.airdrop_data.fee_amount,
        );
    
        invoke(
            &transfer_fee_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin_wallet.to_account_info(),
            ],
        )?;
    
        let admin_wallet_key = ctx.accounts.admin_wallet.key();
        let seeds = &[
            b"PLOUTOS_ROOT".as_ref(),
            admin_wallet_key.as_ref(),
            &[bump_seed]
        ];
        let signer = &[&seeds[..]];

    
        // Transfer 1% of claim to user's token account
        let cpi_user_token_accounts = Transfer {
            from: ctx.accounts.program_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.airdrop_data.to_account_info(),
        };
    
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_cpi_user_token_accounts_ctx = CpiContext::new_with_signer(cpi_program, cpi_user_token_accounts, signer);
    
        // Perform the transfer of 1% to the user
        token::transfer(cpi_cpi_user_token_accounts_ctx, claim_amount/100)?;

        ctx.accounts.user_data.total_claimed = claim_amount/100;

        // update upline
        ctx.accounts.upline_data.referral_count += 1;
        ctx.accounts.upline_data.total_allocation += claim_amount/10;
    
        Ok(())
    }

    pub fn increase_allocation(ctx: Context<IncreaseAllocation>, additional_amount: u64) -> Result<()> {
        let user_data = &mut ctx.accounts.user_data;
        user_data.total_allocation += additional_amount;
        Ok(())
    }
    
    pub fn unlock_allocation(ctx: Context<UnlockAllocation>) -> Result<()> {
        let user_data = &mut ctx.accounts.user_data;
        let airdrop_data = &ctx.accounts.airdrop_data;
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;
    
        // Ensure the unlock period has been met
        require!(
            current_timestamp - user_data.claim_timestamp >= 30 * 86400,
            ErrorCode::UnlockPeriodNotMet
        );
    
        // Calculate the amount to unlock
        let allocation_to_unlock = user_data.total_allocation / 100; 
    
        // Derive the PDA that is the authority of the token account, using admin_wallet from airdrop_data
        let (data_account_pda, bump_seed) = Pubkey::find_program_address(
            &[
                b"PLOUTOS_ROOT".as_ref(), 
                airdrop_data.admin_wallet.as_ref()
            ],
            &ctx.program_id,
        );
    
        // Ensure derived PDA matches the expected authority
        require!(
            data_account_pda == *ctx.accounts.program_token_account.to_account_info().key,
            ErrorCode::PdaMismatch
        );
    
        let seeds = &[
            b"PLOUTOS_ROOT".as_ref(), 
            airdrop_data.admin_wallet.as_ref(), 
            &[bump_seed]
        ];
        let signer_seeds = &[&seeds[..]];
    
        // Prepare the transfer from the program's account to the user's token account
        let cpi_accounts = Transfer {
            from: ctx.accounts.program_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.program_token_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, allocation_to_unlock)?;


        // Update the user data
        user_data.claim_timestamp = current_timestamp;
        user_data.total_claimed += allocation_to_unlock;

        Ok(())
    }
    
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer=user, space=9000, seeds=[b"PLOUTOS_ROOT".as_ref(), user.key().as_ref()], bump)]
    pub data: Account<'info, PloutosData>,
    #[account(init, payer = user, space = 8 + 64, seeds = [b"POUTOS_USER_DATA", user.key().as_ref()], bump)]
    pub user_data: Account<'info, UserData>,
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
    #[account(init, payer = user, space = 8 + 64, seeds = [b"POUTOS_USER_DATA", user.key().as_ref()], bump)]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub upline_data: Account<'info, UserData>,
    #[account(mut, constraint = airdrop_data.token_mint == program_token_account.mint)]
    pub program_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub airdrop_data: Account<'info, PloutosData>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct IncreaseAllocation<'info> {
    #[account(has_one = admin_wallet, constraint = admin_wallet.key() == user.key())]
    pub ploutos_data: Account<'info, PloutosData>,
    #[account(mut)]
    pub user_data: Account<'info, UserData>,
    /// CHECK: This is a system account and its ownership is verified through the `has_one = admin_wallet` constraint.
    pub admin_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnlockAllocation<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub airdrop_data: Account<'info, PloutosData>,
    /// CHECK: This account is the SPL token account owned by the program used for distributing airdrops. It is expected to match the token mint specified in `PloutosData`.
    #[account(mut)]
    pub program_token_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
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

#[account]
pub struct UserData {
    pub claim_timestamp: i64,
    pub claimed: bool,
    pub total_allocation: u64,
    pub total_claimed: u64,
    pub referral_count: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The program has already been initialized")]
    AlreadyInitialized,
    #[msg("Invalid token mint")]
    MintMismatch,
    #[msg("PDA mismatch")]
    PdaMismatch,
    #[msg("The airdrop has already been claimed by this user")]
    AirdropAlreadyClaimed,
    #[msg("The unlock period has not yet been met")]
    UnlockPeriodNotMet,
}

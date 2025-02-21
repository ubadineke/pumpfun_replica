use crate::errors::*;
use crate::states::{bonding_curve::*, global::*};
use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, system_instruction},
};

use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    user: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX.as_bytes()],
        constraint = global.initialized == true @ ContractError::NotInitialized,
        bump,
    )]
    global: Box<Account<'info, Global>>,

    #[account(mut)]
    ///CHECK: Receiver for FEES
    fee_receiver: AccountInfo<'info>,

    mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [BondingCurve::SEED_PREFIX.as_bytes(), mint.to_account_info().key.as_ref()],
        constraint = bonding_curve.complete == false @ ContractError::BondingCurveComplete,
        bump,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [BondingCurve::SOL_ESCROW_SEED_PREFIX.as_bytes(), mint.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA to hold SOL for bonding curve
    pub bonding_curve_sol_escrow: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,

    token_program: Program<'info, Token>,

    associated_token_program: Program<'info, AssociatedToken>,
}

impl Buy<'_> {
    pub fn validate(&self, amount: u64) -> Result<()> {
        require!(amount > 0, ContractError::MinBuy);

        require!(
            self.fee_receiver.key() == self.global.fee_receiver,
            ContractError::InvalidFeeReceiver
        );

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(0); // 0 for data size since this is just a native SOL account
        require!(
            self.user.get_lamports() >= amount.checked_add(min_rent).unwrap(),
            ContractError::InsufficientUserSOL,
        );

        Ok(())
    }
    pub fn handler(ctx: Context<Buy>, sol_amount: u64) -> Result<()> {
        //validate
        ctx.accounts.validate(sol_amount)?;

        let bonding_curve = &mut ctx.accounts.bonding_curve;
        //calculate tokens to be bought
        let mut token_amount = bonding_curve
            .get_tokens_for_buy_with_sol(sol_amount)
            .ok_or(ContractError::CalculationError)?;

        let mut last_buy = false;
        let mut calc_sol_amount = sol_amount;
        let fee_lamports = 1_000_000_000;

        if token_amount >= bonding_curve.real_token_reserves {
            last_buy = true;
            //set token to existing value in reserve and compute new sol_amount to be paid
            token_amount = bonding_curve.real_token_reserves;
            calc_sol_amount = bonding_curve
                .recompute_sol_amount_for_last_buy()
                .ok_or(ContractError::CalculationError)?;
        }
        //Collect Fee
        // Transfer SOL to fee recipient
        let fee_transfer_instruction = system_instruction::transfer(
            ctx.accounts.user.key,
            &ctx.accounts.fee_receiver.key(),
            fee_lamports,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &fee_transfer_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        //Deduct SOL
        let transfer_instruction = system_instruction::transfer(
            ctx.accounts.user.key,
            ctx.accounts.bonding_curve_sol_escrow.to_account_info().key,
            sol_amount,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.bonding_curve_sol_escrow.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        //Send Token
        // Transfer tokens to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.bonding_curve_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: bonding_curve.to_account_info(),
        };
        let mint_key = ctx.accounts.mint.key();
        let mint_auth_signer_seeds: &[&[&[u8]]] = &[&[
            b"bonding-curve",
            &mint_key.as_ref(),
            &[ctx.bumps.bonding_curve],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                mint_auth_signer_seeds,
            ),
            token_amount,
        )?;

        //Update Reserves
        if last_buy == true {
            bonding_curve.complete = true;
        }

        //Update Reserves
        bonding_curve.update_reserves(token_amount, calc_sol_amount);

        Ok(())
    }
}

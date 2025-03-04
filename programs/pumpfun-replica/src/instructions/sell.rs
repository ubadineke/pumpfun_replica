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
pub struct Sell<'info> {
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
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,

    token_program: Program<'info, Token>,

    associated_token_program: Program<'info, AssociatedToken>,
}

impl Sell<'_> {
    pub fn validate(&self, amount: u64) -> Result<()> {
        require!(amount > 0, ContractError::MinSell);

        require!(
            self.fee_receiver.key() == self.global.fee_receiver,
            ContractError::InvalidFeeReceiver
        );

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(0); // 0 for data size since this is just a native SOL account

        require!(
            self.user_token_account.amount >= amount,
            ContractError::InsufficientUserTokens,
        );

        require!(
            self.user.get_lamports() >= amount.checked_add(min_rent).unwrap(),
            ContractError::InsufficientUserSOL,
        );

        Ok(())
    }

    pub fn handler(ctx: Context<Sell>, token_amount: u64) -> Result<()> {
        //validate
        ctx.accounts.validate(token_amount)?;

        let bonding_curve = &mut ctx.accounts.bonding_curve;

        //calculate sol to be received for selling
        let sol_amount = bonding_curve
            .get_sol_for_sale_on_tokens(token_amount)
            .ok_or(ContractError::CalculationError)?;

        msg!("This is the sol amount {}", sol_amount);

        let fee_lamports = 1_000_000;

        let sell_amount_minus_fee = sol_amount - fee_lamports;

        //Collect fees
        //Transfer SOL to fee recipient
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

        //Transfer TOKEN TO BONDING CURVE
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.bonding_curve_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
            token_amount,
        )?;

        //Transfer sol to user
        let transfer_instruction = system_instruction::transfer(
            &ctx.accounts.bonding_curve_sol_escrow.to_account_info().key,
            &ctx.accounts.user.to_account_info().key,
            sell_amount_minus_fee,
        );

        //GENERATE SIGNER SEEDS
        let sol_escrow_signer_seeds: &[&[&[u8]]] = &[&[
            b"sol-escrow",
            &[ctx.bumps.bonding_curve_sol_escrow],
        ]];

        invoke_signed(
            &transfer_instruction,
            &[
                ctx.accounts
                    .bonding_curve_sol_escrow
                    .to_account_info()
                    .clone(),
                ctx.accounts.user.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
            sol_escrow_signer_seeds,
        )?;

        //update reserves
        bonding_curve.update_reserves_after_sell(token_amount, sol_amount);
        Ok(())
    }
}


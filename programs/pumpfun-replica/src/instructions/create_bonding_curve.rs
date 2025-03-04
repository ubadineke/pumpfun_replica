use crate::errors::*;
use crate::states::{bonding_curve::*, global::*};
use anchor_lang::prelude::*;

use anchor_spl::metadata::Metadata;
use anchor_spl::token_interface::spl_token_metadata_interface::state::TokenMetadata;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct CreateBondingCurve<'info> {
    #[account(
        mut,
        constraint = mint.decimals == global.mint_decimals @ ContractError::InvalidMintDecimals,
        constraint = mint.mint_authority == Some(bonding_curve.key()).into() @ContractError::WrongAuthority,
        // mint::authority = bonding_curve,
        // mint::freeze_authority = bonding_curve
    )]
    mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        seeds = [b"bonding-curve", mint.key().as_ref()],
        space = 8 + BondingCurve::INIT_SPACE,
        bump
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [b"sol-escrow", mint.key().as_ref()],
        bump,
    )]
    bonding_curve_sol_escrow: SystemAccount<'info>,

    #[account(
        seeds = [b"global"],
        constraint = global.initialized == true @ ContractError::NotInitialized,
        bump,
    )]
    global: Box<Account<'info, Global>>,

    #[account(mut)]
    //research about adding this part, current implementation
    ///CHECK: Using seed to validate metadata account
    metadata: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: token metadata program account
    pub token_metadata_program: Program<'info, Metadata>,
    // research if this rent account is necessary
    /// CHECK: rent account
    pub rent: UncheckedAccount<'info>,
}

impl CreateBondingCurve<'_> {
    pub fn handler(
        ctx: Context<CreateBondingCurve>,
        params: CreateBondingCurveParams,
    ) -> Result<()> {
        //Initialize Bonding Curve
        ctx.accounts.bonding_curve.update_from_params(
            ctx.accounts.mint.key(),
            ctx.accounts.creator.key(),
            &ctx.accounts.global,
            ctx.bumps.bonding_curve,
        );

        let mint_key = ctx.accounts.mint.key();
        let mint_auth_signer_seeds: &[&[&[u8]]] = &[&[
            b"bonding-curve",
            &mint_key.as_ref(),
            &[ctx.bumps.bonding_curve],
        ]];

        //Create Token Metadata
        ctx.accounts.set_metadata(mint_auth_signer_seeds, &params)?;

        //Mint Tokens
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.bonding_curve.to_account_info(),
                    to: ctx.accounts.bonding_curve_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                mint_auth_signer_seeds,
            ),
            ctx.accounts.bonding_curve.token_total_supply,
        )?;
        //Lock Curve and Revoke Authorities
        Ok(())
    }

    pub fn set_metadata(
        &mut self,
        mint_auth_signer_seeds: &[&[&[u8]]],
        params: &CreateBondingCurveParams,
    ) -> Result<()> {
        let token_data: DataV2 = DataV2 {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };
        let metadata_ctx = CpiContext::new_with_signer(
            self.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: self.creator.to_account_info(),
                mint: self.mint.to_account_info().clone(),
                metadata: self.metadata.to_account_info().clone(),
                update_authority: self.bonding_curve.to_account_info().clone(),
                mint_authority: self.bonding_curve.to_account_info().clone(),
                system_program: self.system_program.to_account_info(),
                rent: self.rent.to_account_info(),
            },
            mint_auth_signer_seeds,
        );
        create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;
        //msg!("CreateBondingCurve::intialize_meta: done");
        Ok(())
    }
}

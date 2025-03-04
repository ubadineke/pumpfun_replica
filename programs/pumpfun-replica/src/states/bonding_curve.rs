use crate::states::global::*;
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct BondingCurve {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub initial_real_token_reserves: u64,

    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,

    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,

    pub token_total_supply: u64,
    pub complete: bool,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct CreateBondingCurveParams {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

impl BondingCurve {
    pub const SEED_PREFIX: &'static str = "bonding-curve";
    pub const SOL_ESCROW_SEED_PREFIX: &'static str = "sol-escrow";
    pub fn update_from_params(
        &mut self,
        mint: Pubkey,
        creator: Pubkey,
        global: &Global,
        bump: u8,
    ) -> &mut Self {
        self.clone_from(&BondingCurve {
            mint,
            creator,
            initial_real_token_reserves: global.initial_real_token_reserves,
            virtual_sol_reserves: global.initial_virtual_sol_reserves,
            virtual_token_reserves: global.initial_virtual_token_reserves,
            real_sol_reserves: 0,
            real_token_reserves: global.initial_real_token_reserves,
            token_total_supply: global.token_total_supply,
            complete: false,
            bump,
        });
        self
    }

    //TOKENS TO BE RECEIVED FOR DEPOSITING A PARTICULAR AMOUNT OF SOL
    pub fn get_tokens_for_buy_with_sol(&self, sol_amount: u64) -> Option<u64> {
        if sol_amount == 0 {
            return None;
        }

        //convert to common decimal basis(using 9 decimals as base)
        let current_sol = self.virtual_sol_reserves as u128;
        let current_tokens = (self.virtual_token_reserves as u128)
            .checked_mul(1_000_000_000)?
            .checked_div(1_000_000)?;
        //calculate new reserves using the constant product formula
        let new_sol = current_sol.checked_add(sol_amount as u128)?;
        let new_tokens = (current_sol.checked_mul(current_tokens)?).checked_div(new_sol)?;

        let tokens_out = current_tokens.checked_sub(new_tokens)?;

        //Convert back to 6 decimal places for tokens
        let tokens_out = tokens_out
            .checked_mul(1_000_000)?
            .checked_div(1_000_000_000)?;

        //Return Tokens
        <u128 as TryInto<u64>>::try_into(tokens_out).ok()
    }

    pub fn get_sol_for_sale_on_tokens(&self, token_amount: u64) -> Option<u64> {
        if token_amount == 0 {
            return None;
        }

        // Convert to common decimal basis (using 9 decimals as base)
        let current_sol = self.virtual_sol_reserves as u128;
        let current_tokens = (self.virtual_token_reserves as u128)
            .checked_mul(1_000_000_000)? // Scale tokens up to 9 decimals
            .checked_div(1_000_000)?; // From 6 decimals

        // Calculate new reserves using constant product formula
        let new_tokens = current_tokens.checked_add(
            (token_amount as u128)
                .checked_mul(1_000_000_000)? // Scale input tokens to 9 decimals
                .checked_div(1_000_000)?, // From 6 decimals
        )?;

        let new_sol = (current_sol.checked_mul(current_tokens)?).checked_div(new_tokens)?;

        let sol_out = current_sol.checked_sub(new_sol)?;

        msg!("GetSolForSellTokens: sol_out: {}", sol_out);
        <u128 as TryInto<u64>>::try_into(sol_out).ok()
    }

    pub fn recompute_sol_amount_for_last_buy(&mut self) -> Option<u64> {
        let token_amount = self.real_token_reserves;

        //Temporarily store current state
        let current_virtual_token_reserves = self.virtual_token_reserves;
        let current_virtual_sol_reserves = self.virtual_sol_reserves;

        //Update self with new amount for the temporary calculation with constant product formula
        self.virtual_token_reserves = (current_virtual_token_reserves as u128)
            .checked_sub(token_amount as u128)?
            .try_into()
            .ok()?;
        self.virtual_sol_reserves = 115_005_359_056;

        let recomputed_sol_amount = self.get_sol_for_sale_on_tokens(token_amount);

        // Restore the state with the recomputed sol_amount
        self.virtual_token_reserves = current_virtual_token_reserves;
        self.virtual_sol_reserves = current_virtual_sol_reserves;

        recomputed_sol_amount
    }

    pub fn update_reserves_after_buy(&mut self, token_amount: u64, sol_amount: u64) -> Option<()> {
        // Adjusting token reserve values
        // New Virtual Token Reserves
        let new_virtual_token_reserves =
            (self.virtual_token_reserves as u128).checked_sub(token_amount as u128)?;
        msg!(
            "ApplyBuy: new_virtual_token_reserves: {}",
            new_virtual_token_reserves
        );

        // New Real Token Reserves
        let new_real_token_reserves =
            (self.real_token_reserves as u128).checked_sub(token_amount as u128)?;
        msg!(
            "ApplyBuy: new_real_token_reserves: {}",
            new_real_token_reserves
        );

        // Adjusting sol reserve values
        // New Virtual Sol Reserves
        let new_virtual_sol_reserves =
            (self.virtual_sol_reserves as u128).checked_add(sol_amount as u128)?;
        msg!(
            "ApplyBuy: new_virtual_sol_reserves: {}",
            new_virtual_sol_reserves
        );

        // New Real Sol Reserves
        let new_real_sol_reserves =
            (self.real_sol_reserves as u128).checked_add(sol_amount as u128)?;
        msg!("ApplyBuy: new_real_sol_reserves: {}", new_real_sol_reserves);

        self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
        self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
        self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
        self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;

        Some(())
    }

pub fn update_reserves_after_sell(&mut self, token_amount:u64, sol_amount: u64) -> Option<()>{

    // Adjusting token reserve values
    // New Virtual Token Reserves
    let new_virtual_token_reserves =
        (self.virtual_token_reserves as u128).checked_add(token_amount as u128)?;
    msg!(
        "apply_sell: new_virtual_token_reserves: {}",
        new_virtual_token_reserves
    );

    // New Real Token Reserves
    let new_real_token_reserves =
        (self.real_token_reserves as u128).checked_add(token_amount as u128)?;
    msg!(
        "apply_sell: new_real_token_reserves: {}",
        new_real_token_reserves
    );

    // Adjusting sol reserve values
    // New Virtual Sol Reserves
    let new_virtual_sol_reserves =
        (self.virtual_sol_reserves as u128).checked_sub(sol_amount as u128)?;
    msg!(
        "apply_sell: new_virtual_sol_reserves: {}",
        new_virtual_sol_reserves
    );

    // New Real Sol Reserves
    let new_real_sol_reserves = self.real_sol_reserves.checked_sub(sol_amount)?;
    msg!(
        "apply_sell: new_real_sol_reserves: {}",
        new_real_sol_reserves
    );

    self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
    self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
    self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
    self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;

    Some(())
    
}
}

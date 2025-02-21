use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Global {
    pub initialized: bool,
    pub global_authority: Pubkey,
    pub migration_authority: Pubkey,
    pub migrate_fee_amount: u64,
    pub migration_token_allocation: u64,
    pub fee_receiver: Pubkey,

    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub mint_decimals: u8,

    pub lp_config: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct GlobalSettingsInput {
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub mint_decimals: u8,
    pub migrate_fee_amount: u64,
    pub migration_token_allocation: u64,
    pub fee_receiver: Pubkey,
    pub lp_config: Pubkey,
}

impl Global {
    pub const SEED_PREFIX: &'static str = "global";
    pub fn update_settings(&mut self, params: GlobalSettingsInput) {
        self.initial_virtual_token_reserves = params.initial_virtual_token_reserves;
        self.initial_virtual_sol_reserves = params.initial_virtual_sol_reserves;
        self.initial_real_token_reserves = params.initial_real_token_reserves;
        self.token_total_supply = params.token_total_supply;
        self.mint_decimals = params.mint_decimals;
        self.migrate_fee_amount = params.migrate_fee_amount;
        self.migration_token_allocation = params.migration_token_allocation;
        self.fee_receiver = params.fee_receiver;
        self.lp_config = params.lp_config;
    }
}

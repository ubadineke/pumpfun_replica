use crate::instructions::*;
use crate::states::*;
use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod states;

declare_id!("32amtTMiGyJSm84RhinR8WbnUTU5BFMSo5FGFxQXKtJd");

#[program]
pub mod pumpfun_replica {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: GlobalSettingsInput) -> Result<()> {
        initialize::initialize(ctx, params)
    }

    pub fn create_bonding_curve(
        ctx: Context<CreateBondingCurve>,
        params: CreateBondingCurveParams,
    ) -> Result<()> {
        CreateBondingCurve::handler(ctx, params)
    }
    pub fn buy(ctx: Context<Buy>, sol_amount: u64) -> Result<()> {
        Buy::handler(ctx, sol_amount)
    }
}

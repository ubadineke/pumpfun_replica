//configuration

use crate::states::global::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Global::INIT_SPACE,
        seeds = [b"global"],
        bump
    )]
    global: Box<Account<'info, Global>>,

    system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>, params: GlobalSettingsInput) -> Result<()> {
    let global = &mut ctx.accounts.global;

    global.update_settings(params);
    global.migration_authority = ctx.accounts.authority.key();
    global.global_authority = ctx.accounts.authority.key();
    global.initialized = true;
    Ok(())
}

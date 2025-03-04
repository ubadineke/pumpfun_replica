use anchor_lang::error_code;

#[error_code]
pub enum ContractError {
    #[msg("Global Not Initialized")]
    NotInitialized,

    #[msg("Bonding Curve Complete")]
    BondingCurveComplete,

    #[msg("Invalid Fee Receiver")]
    InvalidFeeReceiver,

    #[msg("Buy amount is 0")]
    MinBuy,

    #[msg("Sell amount is 0")]
    MinSell,

    #[msg("Insufficient User Tokens")]
    InsufficientUserTokens,

    #[msg("Insufficient user SOL")]
    InsufficientUserSOL,

    #[msg("Error while performing calculation")]
    CalculationError,

    #[msg("Invalid Mint Decimals")]
    InvalidMintDecimals,

    #[msg("Wrong Authority")]
    WrongAuthority
}

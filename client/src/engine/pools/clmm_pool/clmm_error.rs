use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
}

#[derive(Debug, Error)]
pub enum SwapError {
}


#[derive(Debug, Error, PartialEq)]
pub enum ErrorCode {
    #[error("sqrtPrice out of bounds")]
    SqrtPriceOutOfBounds,

    #[error("sqrtPrice is 0")]
    SqrtPriceIsZero,

    #[error("sqrtRatio is 0")]
    SqrtRatioIsZero,

    #[error("tick out of bounds")]
    TickOutOfBounds,

    #[error("liquidity is 0")]
    LiquidityIsZero,

    #[error("requested amount exceeds pool reserves")]
    InsufficientReserves,

    #[error("overflow")]
    Overflow,

    #[error("underflow")]
    Underflow,

    #[error("out of bounds")]
    OutOfBounds,

    #[error("division by zero")]
    DivisionByZero,

    #[error("zero input value")]
    ZeroValue,

    #[error("RequireGtViolated")]
    RequireGtViolated,

    #[error("RequireGteViolated")]
    RequireGteViolated,

    #[error("RequireViolated")]
    RequireViolated,

    #[error("input value must be greater than 0")]
    InputValueIsZero,
}
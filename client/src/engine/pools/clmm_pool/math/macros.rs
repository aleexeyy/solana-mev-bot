#[macro_export]
macro_rules! require_gt {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 <= $value2 {
            return Err($error_code);
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 <= $value2 {
            return Err(ErrorCode::RequireGtViolated);
        }
    };
}


#[macro_export]
macro_rules! require_gte {
    ($value1: expr, $value2: expr, $error_code: expr $(,)?) => {
        if $value1 < $value2 {
            return Err($error_code);
        }
    };
    ($value1: expr, $value2: expr $(,)?) => {
        if $value1 < $value2 {
            return Err(ErrorCode::RequireGteViolated);
        }
    };
}


#[macro_export]
macro_rules! require {
    ($condition: expr, $error_code: expr $(,)?) => {
        if !$condition {
            return Err($error_code);
        }
    };
    ($condition: expr $(,)?) => {
        if !$condition {
            return Err(ErrorCode::RequireViolated);
        }
    };
}
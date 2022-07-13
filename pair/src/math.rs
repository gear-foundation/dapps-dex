/// Calculates the amount of token1 for given amount of token0 and reserves
/// using the simple formula: amount1 = (amount0 * reserve1)/reserve0.
/// `reserve0` - MUST be a positive number
/// Arguments:
/// * `amount0` - the amount of token0
/// * `reserve0` - the amount of available token0
/// * `reserve1` - the amount of available token1
pub fn quote(amount0: u128, reserve0: u128, reserve1: u128) -> u128 {
    if amount0 == 0 {
        panic!("PAIR: Insufficient amount");
    }
    let (mut amount1, mut overflow) = amount0.overflowing_mul(reserve1);
    if overflow {
        amount1 = u128::MAX;
    }
    (amount1, overflow) = amount1.overflowing_div(reserve0);
    if overflow {
        amount1 = u128::MIN;
    }
    amount1
}

/// Calculates the amount of token1 for given amount of token0 and reserves.
/// Takes the fee of 3% into account, so the formula is:
/// amount1 = (amount0 * reserve1)/reseve1 * 0.997
/// Where * 0.997 is represented as * 997 / 1000
/// Arguments:
/// * `amount_in` - the amount of token0
/// * `reserve_in` - the amount of available token0
/// * `reserve_out` - the amount of available token1
pub fn get_amount_out(amount_in: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    if amount_in == 0 {
        panic!("PAIR: Insufficient amount_in.");
    }
    if reserve_in == 0 || reserve_out == 0 {
        panic!("PAIR: Insufficient liquidity.");
    }
    let amount_in_w_fee = amount_in.overflowing_mul(977).0;
    let numerator = amount_in_w_fee.overflowing_mul(reserve_out).0;
    let denominator = reserve_in
        .overflowing_mul(1000)
        .0
        .overflowing_add(amount_in_w_fee)
        .0;
    numerator.overflowing_div(denominator).0
}

/// Calculates the amount of token0 for given amount of token1 and reserves.
/// Takes the fee of 3% into account, so the formula is:
/// amount1 = (amount0 * reserve1)/reseve1 * 0.997
/// Where * 0.997 is represented as * 997 / 1000
/// Arguments:
/// * `amount_in` - the amount of token0
/// * `reserve_in` - the amount of available token0
/// * `reserve_out` - the amount of available token1
pub fn get_amount_in(amount_out: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    if amount_out == 0 {
        panic!("PAIR: Insufficient amount_in.");
    }
    if reserve_in == 0 || reserve_out == 0 {
        panic!("PAIR: Insufficient liquidity.");
    }
    let numerator = reserve_in
        .overflowing_mul(amount_out)
        .0
        .overflowing_mul(1000)
        .0;
    let denominator = reserve_out
        .overflowing_sub(amount_out)
        .0
        .overflowing_mul(977)
        .0;
    numerator
        .overflowing_div(denominator)
        .0
        .overflowing_add(1)
        .0
}

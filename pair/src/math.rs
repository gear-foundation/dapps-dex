use primitive_types::U256;

use super::Error;

pub fn quote(amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
    if let Err(error) = perform_precalculate_check(amount, reserve) {
        Err(error)
    } else {
        let U256PairTuple(reserve) = reserve.into();

        if let Ok(result) = (U256::from(amount) * reserve.1 / reserve.0).try_into() {
            Ok(result)
        } else {
            Err(Error::Overflow)
        }
    }
}

pub fn calculate_out_amount(in_amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
    perform_precalculate_check(in_amount, reserve)?;

    let amount_with_fee: U256 = U256::from(in_amount) * 997;
    if let Some(numerator) = amount_with_fee.checked_mul(reserve.1.into()) {
        // Shouldn't overflow.
        let denominator = U256::from(reserve.0) * 1000 + amount_with_fee;

        // Shouldn't be more than u128::MAX, so casting doesn't lose data.
        Ok((numerator / denominator).low_u128())
    } else {
        Err(Error::Overflow)
    }
}

pub fn calculate_in_amount(out_amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
    perform_precalculate_check(out_amount, reserve)?;

    // The `u64` suffix is needed for a faster conversion.
    let numerator = (U256::from(reserve.0) * U256::from(out_amount)).checked_mul(1000u64.into());

    if let (Some(numerator), Some(amount)) = (numerator, reserve.1.checked_sub(out_amount)) {
        if amount == 0 {
            Err(Error::Overflow)
        } else {
            let denominator = U256::from(amount) * 997;

            // Adding 1 here to avoid abuse of the case when a calculated input
            // amount will equal 0.
            if let Ok(in_amount) = (numerator / denominator + 1).try_into() {
                Ok(in_amount)
            } else {
                Err(Error::Overflow)
            }
        }
    } else {
        Err(Error::Overflow)
    }
}

fn perform_precalculate_check(amount: u128, reserve: (u128, u128)) -> Result<(), Error> {
    if reserve.0 == 0 || reserve.1 == 0 {
        Err(Error::InsufficientLiquidity)
    } else if amount == 0 {
        Err(Error::InsufficientAmount)
    } else {
        Ok(())
    }
}

pub(crate) struct U256PairTuple(pub(crate) (U256, U256));

impl From<(u128, u128)> for U256PairTuple {
    fn from(value: (u128, u128)) -> Self {
        Self((value.0.into(), value.1.into()))
    }
}

use alloy_sol_types::{sol, SolError};
use std::string::FromUtf8Error;
use stylus_sdk::call::MethodError;

sol! {
    error UserNotExist();
    error InvalidPrice();
    error GeneralError(string msg);
}

pub enum BitsaveErrors {
    UserNotExist(UserNotExist),
    GeneralError(GeneralError),
    FromUtf8Error(FromUtf8Error),
    InvalidPrice(InvalidPrice),
}

impl From<BitsaveErrors> for Vec<u8> {
    fn from(val: BitsaveErrors) -> Self {
        match val {
            BitsaveErrors::InvalidPrice(err) => err.encode(),
            BitsaveErrors::UserNotExist(err) => err.encode(),
            BitsaveErrors::GeneralError(err) => err.encode(),
            BitsaveErrors::FromUtf8Error(err) => err.into_bytes(),
        }
    }
}

impl From<FromUtf8Error> for BitsaveErrors {
    fn from(err: FromUtf8Error) -> Self {
        Self::FromUtf8Error(err)
    }
}

impl From<GeneralError> for BitsaveErrors {
    fn from(err: GeneralError) -> Self {
        Self::GeneralError(err)
    }
}

pub type BResult<T, E = BitsaveErrors> = core::result::Result<T, E>;
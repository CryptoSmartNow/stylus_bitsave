use alloy_sol_types::sol;
use std::string::FromUtf8Error;
use stylus_sdk::call::MethodError;

sol! {
    error InvalidUser();
    error InvalidPrice();
    error GeneralError(string msg);
    error InvalidCall();
    error NotSupported();
    error InvalidSaving();
}

pub enum BitsaveErrors {
    InvalidUser(InvalidUser),
    GeneralError(GeneralError),
    InvalidCall(InvalidCall),
    FromUtf8Error(FromUtf8Error),
    InvalidPrice(InvalidPrice),
    InvalidSaving(InvalidSaving),
    NotSupported(NotSupported),
}

impl From<BitsaveErrors> for Vec<u8> {
    fn from(val: BitsaveErrors) -> Self {
        match val {
            BitsaveErrors::InvalidPrice(err) => err.encode(),
            BitsaveErrors::InvalidUser(err) => err.encode(),
            BitsaveErrors::GeneralError(err) => err.encode(),
            BitsaveErrors::FromUtf8Error(err) => err.into_bytes(),
            BitsaveErrors::NotSupported(err) => err.encode(),
            BitsaveErrors::InvalidCall(err) => err.encode(),
            BitsaveErrors::InvalidSaving(err) => err.encode(),
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
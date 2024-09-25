//!
//! Stylus bitsave
//!
//! The following contract implements the Counter example from Foundry.
//!
//!
//! The program is ABI-equivalent with Solidity, which means you can call it from both Solidity and Rust.
//! To do this, run `cargo stylus export-abi`.
//!
//! Note: this code is a template-only and has not been audited.
//!

// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(feature = "export-abi"), no_main)]

extern crate alloc;

use crate::constants::{BS_SAVING_FEE, MIN_BS_JOIN_FEE};
use crate::errors::BitsaveErrors::InvalidCall;
use crate::errors::{
    BResult, BitsaveErrors, GeneralError, InvalidPrice, NotSupported, InvalidUser,
};
use alloy_primitives::{Address, U256};
use stylus_sdk::call::{call, transfer_eth, Call};
/// Import items from the SDK. The prelude contains common traits and macros.
use stylus_sdk::{msg, prelude::*};
/// Import user library and other fns
use user_data::UserData;

mod constants;
mod errors;
mod user_data;

pub type RResult<T, E = Vec<u8>> = core::result::Result<T, E>;

// Define some persistent storage using the Solidity ABI.
// `Bitsave` will be the entrypoint.
sol_storage! {
    #[entrypoint]
    pub struct Bitsave {
        // Maintenance details
        bool initialized;
        address master_address;
        address collector_address;

        // SWAP Details
        address router_address;
        address stablecoin_address;

        // collection details
        uint256 vault_state;
        uint256 total_value_locked;

        uint256 user_count;
        uint256 accumulated_pool_balance;
        uint256 general_fund;
        mapping(address => UserData) users_mapping;
    }
}

// sol_interface! {
//     interface IUniswapV2Router {
//         function swapExactETHForTokens(
//             uint amountOutMin,
//             address[] calldata path,
//             address to,
//             uint deadline
//         ) external payable returns (uint[] memory amounts);
//     }
//
//     interface IERC20 {
//         function transfer(address recipient, uint256 amount) external returns (bool);
//         function balanceOf(address account) external view returns (uint256);
//     }
// }

/// Declare that `Bitsave` is a contract with the following external methods.
#[public]
impl Bitsave {
    fn require_master(&self, sender: Address) -> RResult<()> {
        if sender != self.master_address.get() {
            return Err(BitsaveErrors::GeneralError(GeneralError {
                msg: "Not authorized".to_string(),
            })
            .into());
        }
        Ok(())
    }

    /// Initialize data
    pub fn init(&mut self) {
        if !self.initialized.get() {
            self.master_address.set(msg::sender());
            self.collector_address.set(msg::sender());
            self.general_fund.set(U256::from(0));
            self.initialized.set(true);
        }
    }

    pub fn change_data(
        &mut self,
        router_address: Address,
        stablecoin_address: Address,
        collector_address: Address,
    ) {
        self.require_master(msg::sender()).unwrap();

        self.router_address.set(router_address);
        self.stablecoin_address.set(stablecoin_address);
        self.collector_address.set(collector_address);
    }

    pub fn update_vault(&mut self, v_state: U256, total_locked: U256) {
        self.require_master(msg::sender()).unwrap();

        self.vault_state.set(v_state);
        self.total_value_locked.set(total_locked);
    }

    /// My gathered points
    pub fn get_user_points(&self) -> BResult<U256> {
        Ok(self.users_mapping.get(msg::sender()).total_point.get())
    }

    /// Join the space
    #[payable]
    pub fn join_bitsave(&mut self, user_name: String) -> RResult<Address> {
        // check user doesn't exist
        let fetched_user = self.users_mapping.get(msg::sender());
        if fetched_user.user_exists.get() {
            return Err(BitsaveErrors::InvalidUser(InvalidUser {}).into())
        };

        // check for joining fee
        if msg::value() < U256::from(MIN_BS_JOIN_FEE) {
            return Err(BitsaveErrors::InvalidPrice(InvalidPrice {}).into());
        }

        // incr user count
        let new_user_count = self.user_count.get() + U256::from(1);
        self.user_count.set(new_user_count);

        let mut fetched_user = self.users_mapping.setter(msg::sender());
        // update user data
        fetched_user.create_user(msg::sender(), new_user_count, user_name);

        // return user exists txn
        Ok(self.users_mapping.get(msg::sender()).user_address.get())
    }

    /// Create a new saving
    #[payable]
    pub fn create_saving(
        &mut self,
        name_of_saving: String,
        maturity_time: U256,
        penalty_perc: u8,
        use_safe_mode: bool,
    ) -> RResult<()> {
        // retrieve some data
        // fetch user's data

        let amount_received = msg::value();
        let saving_fee = U256::from(BS_SAVING_FEE);
        if amount_received < saving_fee {
            return Err(BitsaveErrors::InvalidPrice(InvalidPrice {}).into());
        }

        // Send fee to collector address
        transfer_eth(self.collector_address.get(), saving_fee)?;

        let amount_of_saving = amount_received - saving_fee;
        // Update pool

        let token_id = Address::ZERO; // todo: fix in token address

        // TODO: add safe mode fn;
        if use_safe_mode {
            return Err(BitsaveErrors::NotSupported(NotSupported {}).into());
        }

        // user setter
        let mut user_updater = self.users_mapping.setter(msg::sender());
        user_updater.create_saving_data(
            name_of_saving,
            amount_of_saving,
            token_id,
            maturity_time,
            penalty_perc,
            use_safe_mode,
            self.vault_state.get(),
            self.total_value_locked.get(),
        )?;

        Ok(())
    }

    /// Increment saving
    pub fn increment_saving(&mut self, name_of_saving: String) -> Result<(), Vec<u8>> {
        // retrieve some data
        // fixme fetch user's data

        let amount_to_add = msg::value();
        let token_id = Address::ZERO; // todo: fix in token address

        // user setter
        let mut user_updater = self.users_mapping.setter(msg::sender());
        user_updater.increment_saving_data(
            name_of_saving,
            amount_to_add,
            token_id,
            self.vault_state.get(),
            self.total_value_locked.get(),
        )?;
        Ok(())
    }

    /// Withdraw savings
    pub fn withdraw_savings(&mut self, name_of_saving: String) -> Result<U256, Vec<u8>> {
        if msg::reentrant() {
            return Err(
                // Should be a general error but saving size
                BitsaveErrors::NotSupported(NotSupported {}).into());
        }

        // user updater
        let mut user_updater = self.users_mapping.setter(msg::sender());
        let with_amount = user_updater.withdraw_saving_data(name_of_saving)?;

        // transfer funds
        call(Call::new_in(self).value(with_amount), msg::sender(), &[])?;

        Ok(with_amount)
    }
}

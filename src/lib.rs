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

/// Import items from the SDK. The prelude contains common traits and macros.
use stylus_sdk::{msg, prelude::*};
use alloy_primitives::{address, Address, U256};
use ethers::prelude::Call;
use stylus_sdk::call::call;
/// Import user library and other fns
use user_data::UserData;
use crate::constants::{BS_SAVING_FEE, MIN_BS_JOIN_FEE};
use crate::errors::{BResult, BitsaveErrors, GeneralError, InvalidPrice, NotSupported, UserNotExist};

mod user_data;
mod errors;
mod constants;


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

sol_interface! {
    interface IUniswapV2Router {
        function swapExactETHForTokens(
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external payable returns (uint[] memory amounts);
    }

    interface IERC20 {
        function transfer(address recipient, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}

/// Declare that `Counter` is a contract with the following external methods.
#[public]
impl Bitsave {

    fn require_master(&self, sender: Address) -> RResult<()> {
        if sender != self.master_address.get() {
                panic!(Err(
                    BitsaveErrors::GeneralError(GeneralError {
                        msg: "Not authorized".to_string()
                    }).into()
                ))
            }
        Ok(())
    }

    /// Initialize data
    pub fn init(&mut self) {
        if !self.initialized.get() {
            self.master_address.set(msg::sender());
            self.collector_address.set(msg::sender());
            self.general_fund.set(
                U256::from(0)
            );
            self.initialized.set(true);
        }
    }

    pub fn change_data(&mut self, router_address: Option<Address>, stablecoin_address: Option<Address>, collector_address: Option<Address>) {
        self.require_master(msg::sender())?;

        if let Some(_router_address) = router_address {
            self.router_address.set(_router_address);
        }
        if let Some(_stablecoin_address) = stablecoin_address {
            self.stablecoin_address.set(_stablecoin_address);
        }
        if let Some(_collector_address) = collector_address {
            self.collector_address.set(_collector_address);
        }
    }

    pub fn update_vault(&mut self, v_state: Option<U256>, total_locked: Option<U256>) {
        self.require_master(msg::sender())?;

        if let Some(_vault_state) = v_state {
            self.vault_state.set(_vault_state);
        }
        if let Some(_total_locked) = total_locked {
            self.total_value_locked.set(_total_locked);
        }
    }

    /// Pool manager
    fn update_pool_details(&mut self) {
    }

    /// Metric data
    pub fn get_bitsave_user_count(&self) -> U256 {
        self.user_count.get()
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
            return Err(
                BitsaveErrors::from(GeneralError {
                    msg: "User exists already!".to_string()
                }).into()
            );
        };

        // check for joining fee
        if msg::value() < U256::from(constants::MIN_BS_JOIN_FEE) {
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
        let fetched_user = self.users_mapping.get(msg::sender());
        if !fetched_user.user_exists.get() {
            println!("User not found");
            panic!(Err(BitsaveErrors::UserNotExist(UserNotExist {})));
        }

        let amount_received = msg::value();
        let saving_fee = U256::from(BS_SAVING_FEE);
        if amount_received < saving_fee {
            panic!(
                Err(
                    BitsaveErrors::InvalidPrice(InvalidPrice {})
                )
            )
        }

        let amount_of_saving = amount_received - saving_fee;
        // Update pool


        let mut token_id = Address::ZERO; // todo: fix in token address

        // TODO: add safe mode fn;
        if use_safe_mode {
            panic!(
                Err(
                    BitsaveErrors::NotSupported(NotSupported {}).into()
                )
            )
        }


        // user setter
        let mut user_updater = self.users_mapping.setter(msg::sender());
        let res = user_updater.create_saving_data(
            name_of_saving,
            amount_of_saving,
            token_id,
            maturity_time,
            penalty_perc,
            use_safe_mode,
            self.vault_state.get(),
            self.total_value_locked.get()
        );

        if let Err(res_err) = res {
            panic!(Err(res_err.into()));
        }

        Ok(())
    }

    /// Increment saving
    pub fn increment_saving(&mut self, name_of_saving: String) -> Result<(), Vec<u8>> {
        // retrieve some data
        // fetch user's data
        let fetched_user = self.users_mapping.get(msg::sender());
        if !fetched_user.user_exists.get() {
            panic!(Err("User doesn't exist"));
        }

        let amount_to_add = msg::value();
        let token_id = Address::ZERO; // todo: fix in token address

        // user setter
        let mut user_updater = self.users_mapping.setter(msg::sender());
        user_updater.increment_saving_data(
            name_of_saving,
            amount_to_add,
            token_id,
            self.vault_state.get(),
            self.total_value_locked.get()
        )?;
        Ok(())
    }

    /// Withdraw savings
    pub fn withdraw_savings(&mut self, name_of_saving: String) -> Result<U256, Vec<u8>> {

        if (msg::reentrant()) {
            panic!(Err("Reentrant call not allowed!"));
        }

        let fetched_user = self.users_mapping.get(msg::sender());
        if !fetched_user.user_exists.get() {
            return Err("User doesn't exist".into());
        }

        // user updater
        let mut user_updater = self.users_mapping.setter(msg::sender());
        let with_amount = user_updater.withdraw_saving_data(name_of_saving)?;

        // transfer funds
        call(Call::new_in(self).value(with_amount), msg::sender(), &[])?;

        Ok(with_amount)
    }
}

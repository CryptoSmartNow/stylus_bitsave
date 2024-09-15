use std::ops::Add;
use alloy_primitives::{Address, U256, U8};
use stylus_sdk::{block, stylus_proc::sol_storage};
use crate::constants::{DIVISOR, HUNDRED, MAX_SUPPLY, TOTAL_SUPPLY, YEARS_IN_SECS};
use crate::errors::{BitsaveErrors, GeneralError};
use crate::RResult;

sol_storage! {
    pub struct UserData {
        bool user_exists;
        address user_address;
        uint256 user_id;
        string user_name;
        uint8 savings_count;
        mapping(string => SavingData) savings_map;
        string[] savings_names;
        uint256 total_point;
    }

    pub struct SavingData {
        bool is_valid;
        uint256 amount;
        uint256 maturity_time;
        uint256 start_time;
        address token_id;
        bool is_safe_mode;
        uint256 interest_accumulated;
        uint8 penalty_perc;
    }
}

impl UserData {
    /// Create user details
    pub fn create_user(&mut self, address: Address, user_id: U256, user_name: String) -> bool {
        self.user_address.set(address);
        self.user_exists.set(true);
        self.user_id.set(user_id);
        self.user_name.set_str(user_name);
        self.total_point.set(U256::from(0));
        self.user_exists.get()
    }

    pub fn get_user_id(&self) -> U256 {
        self.user_id.get()
    }

    /// bitsave interest calculator:
    /// Uses bitsave formulae; to be integrated through the bitsave's token
    fn calculate_new_interest(
        &self,
        amount: U256,
        end_time: U256,
        // internal data
        vault_state: U256,
        total_value_locked: U256
    ) -> U256 {

        let total_supply: U256 = U256::from(TOTAL_SUPPLY);
        let max_supply: U256 = U256::from(MAX_SUPPLY);
        let years_in_second: U256 = U256::from(YEARS_IN_SECS);
        let hundred: U256 = U256::from(HUNDRED);
        let divisor: U256 = U256::from(DIVISOR);


        amount * U256::from(1) / U256::from(100);
        let crp = ((total_supply - vault_state) / vault_state) * hundred;
        let bs_rate = max_supply / (crp * total_value_locked);
        let years_taken = (end_time - U256::from(block::timestamp())) / years_in_second;
        ((amount * bs_rate * years_taken) / (hundred * divisor))
    }

    fn calculate_balance_from_penalty(amount: U256, penalty_perc: U8) -> U256 {
        let perc_value = amount * U256::from(penalty_perc) / U256::from(100);
        amount - perc_value
    }

    pub fn create_saving_data(
        &mut self,
        name_of_saving: String,
        amount_of_saving: U256,
        token_id: Address,
        maturity_time: U256,
        penalty_perc: u8,
        use_safe_mode: bool,
        vault_state: U256,
        total_value_locked: U256
    ) -> RResult<()> {
        let fetched_saving = self.savings_map.get(name_of_saving.clone());

        // error if saving exists
        if fetched_saving.is_valid.get() {
            return Err(BitsaveErrors::GeneralError(GeneralError {
                msg: "Saving exists already".to_string()
            }).into());
        };

        let new_interest = self.calculate_new_interest(
            amount_of_saving,
            maturity_time,
            vault_state,
            total_value_locked
        );

        let mut new_saving = self.savings_map.setter(name_of_saving);

        self.total_point.add(new_interest);

        // update saving data
        new_saving.is_safe_mode.set(use_safe_mode);
        new_saving.is_valid.set(true);
        new_saving.token_id.set(token_id);
        new_saving.maturity_time.set(maturity_time);
        new_saving.start_time.set(U256::from(block::timestamp()));
        new_saving.interest_accumulated.set(new_interest);
        new_saving.amount.set(amount_of_saving);
        new_saving.penalty_perc.set(U8::from(penalty_perc));

        Ok(())
    }

    pub fn increment_saving_data(
        &mut self,
        name_of_saving: String,
        new_amount: U256,
        token_id: Address,
        vault_state: U256,
        total_value_locked: U256
    ) -> Result<(), Vec<u8>> {
        let saving_data = self.savings_map.get(name_of_saving.clone());
        if !saving_data.is_valid.get() {
            return Err(format!("Saving `{}` doesn't exist", name_of_saving).into());
        };

        if !saving_data.token_id.eq(&token_id) {
            // token not same with one being saved
            return Err("Different token being saved, create new saving".into());
        }

        let old_interest = saving_data.interest_accumulated.get();
        let old_amount = saving_data.amount.get();

        // saving is valid, increment the saving data
        let new_interest = self.calculate_new_interest(
            new_amount,
            saving_data.maturity_time.get(),
            vault_state,
            total_value_locked
        );
        self.total_point.add(new_interest);

        let mut saving_updater = self.savings_map.setter(name_of_saving);

        // increment amount and interest
        saving_updater
            .interest_accumulated
            .set(old_interest + new_interest);
        saving_updater.amount.set(old_amount + new_amount);

        // saving updated
        Ok(())
    }

    pub fn withdraw_saving_data(&mut self, name_of_saving: String) -> Result<U256, Vec<u8>> {
        let saving_data = self.savings_map.get(name_of_saving.clone());
        if !saving_data.is_valid.get() {
            return Err(format!("Saving `{}` doesn't exist", name_of_saving).into());
        }

        let mut withdraw_amount: U256 = U256::from(0);

        // check if maturity is complete
        let saving_amount = saving_data.amount.get();
        if saving_data.maturity_time.get() < U256::from(block::timestamp()) {
            // saving isn't complete, remove percentage
            withdraw_amount =
                Self::calculate_balance_from_penalty(saving_amount, saving_data.penalty_perc.get());
        } else {
            // saving complete, send interest
            withdraw_amount = saving_amount;
            // todo: send interest
        }

        // clear saving data
        // is_valid, amount, interest_accumulated, penalty_perc
        let mut saving_updater = self.savings_map.setter(name_of_saving);

        saving_updater.is_valid.set(false);
        saving_updater.amount.set(U256::from(0));
        saving_updater.interest_accumulated.set(U256::from(0));
        saving_updater.penalty_perc.set(U8::from(0));

        Ok(withdraw_amount)
    }
}

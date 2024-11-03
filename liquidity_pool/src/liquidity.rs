multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::*;

use super::liq_math;
use super::liq_storage;
use super::liq_utils;
use super::tokens;

#[multiversx_sc::module]
pub trait LiquidityModule:
    liq_storage::StorageModule
    + tokens::TokensModule
    + common_tokens::AccountTokenModule
    + liq_utils::UtilsModule
    + liq_math::MathModule
    + price_aggregator_proxy::PriceAggregatorModule
    + common_checks::ChecksModule
{
    #[only_owner]
    #[payable("*")]
    #[endpoint(updateCollateralWithInterest)]
    fn update_collateral_with_interest(
        &self,
        mut deposit_position: DepositPosition<Self::Api>,
    ) -> DepositPosition<Self::Api> {
        let round = self.blockchain().get_block_round();
        let supply_index = self.supply_index().get();

        self.update_interest_indexes();

        let accrued_interest = self.compute_interest(
            &deposit_position.amount,
            &supply_index,
            &deposit_position.initial_supply_index,
        );

        deposit_position.amount += accrued_interest;
        deposit_position.round = round;
        deposit_position.initial_supply_index = supply_index;

        deposit_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(updateBorrowsWithDebt)]
    fn update_borrows_with_debt(
        &self,
        mut borrow_position: BorrowPosition<Self::Api>,
    ) -> BorrowPosition<Self::Api> {
        let round = self.blockchain().get_block_round();
        let borrow_index = self.borrow_index().get();

        self.update_interest_indexes();

        let accumulated_debt = self.get_debt_interest(
            &borrow_position.amount,
            &borrow_position.initial_borrow_index,
        );

        borrow_position.amount += accumulated_debt;
        borrow_position.round = round;
        borrow_position.initial_borrow_index = borrow_index;

        borrow_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(addCollateral)]
    fn add_collateral(
        &self,
        deposit_position: DepositPosition<Self::Api>,
    ) -> DepositPosition<Self::Api> {
        let (deposit_asset, deposit_amount) = self.call_value().single_fungible_esdt();
        let pool_asset = self.pool_asset().get();
        let round = self.blockchain().get_block_round();
        let supply_index = self.supply_index().get();
        let mut ret_deposit_position = deposit_position.clone();

        require!(
            deposit_asset == pool_asset,
            "asset not supported for this liquidity pool"
        );

        self.update_interest_indexes();

        if deposit_position.amount != 0 {
            ret_deposit_position = self.update_collateral_with_interest(deposit_position);
        }
        ret_deposit_position.amount += &deposit_amount;
        ret_deposit_position.round = round;
        ret_deposit_position.initial_supply_index = supply_index;

        self.reserves().update(|x| *x += &deposit_amount);
        self.supplied_amount().update(|x| *x += deposit_amount);
        ret_deposit_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint]
    fn borrow(
        &self,
        initial_caller: ManagedAddress,
        borrow_amount: BigUint,
        existing_borrow_position: BorrowPosition<Self::Api>,
    ) -> BorrowPosition<Self::Api> {
        let pool_token_id = self.pool_asset().get();

        let asset_reserve = self.reserves().get();
        let mut ret_borrow_position = existing_borrow_position.clone();
        self.require_non_zero_address(&initial_caller);
        require!(
            asset_reserve >= borrow_amount,
            "insufficient funds to perform loan"
        );

        self.update_interest_indexes();
        if ret_borrow_position.amount != 0 && ret_borrow_position.nft.is_none() {
            ret_borrow_position = self.update_borrows_with_debt(existing_borrow_position);
        }

        let round = self.blockchain().get_block_round();
        let borrow_index = self.borrow_index().get();
        ret_borrow_position.amount += &borrow_amount;
        ret_borrow_position.round = round;
        ret_borrow_position.initial_borrow_index = borrow_index;

        self.borrowed_amount()
            .update(|total| *total += &borrow_amount);

        self.reserves().update(|total| *total -= &borrow_amount);

        self.send()
            .direct_esdt(&initial_caller, &pool_token_id, 0, &borrow_amount);

        ret_borrow_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(borrowWithNFTs)]
    fn borrow_bulk_nfts(
        &self,
        initial_caller: ManagedAddress,
        borrow_amount: BigUint,
        existing_borrow_positions: ManagedVec<BorrowPosition<Self::Api>>,
    ) -> ManagedVec<BorrowPosition<Self::Api>> {
        let pool_token_id = self.pool_asset().get();

        let asset_reserve = self.reserves().get();
        self.require_non_zero_address(&initial_caller);
        require!(
            asset_reserve >= borrow_amount,
            "insufficient funds to perform loan"
        );

        self.update_interest_indexes();

        let borrow_index = self.borrow_index().get();
        for mut pos in &existing_borrow_positions {
            pos.initial_borrow_index = borrow_index.clone();
        }

        self.borrowed_amount()
            .update(|total| *total += &borrow_amount);

        self.reserves().update(|total| *total -= &borrow_amount);

        self.send()
            .direct_esdt(&initial_caller, &pool_token_id, 0, &borrow_amount);

        existing_borrow_positions
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint]
    fn remove_collateral(
        &self,
        initial_caller: ManagedAddress,
        amount: BigUint,
        mut deposit_position: DepositPosition<Self::Api>,
    ) -> DepositPosition<Self::Api> {
        let pool_asset = self.pool_asset().get();

        self.require_non_zero_address(&initial_caller);
        self.require_amount_greater_than_zero(&amount);

        self.update_interest_indexes();

        // Withdrawal amount = initial_deposit + Interest
        let withdrawal_amount = self.compute_withdrawal_amount(
            &amount,
            &self.supply_index().get(),
            &deposit_position.initial_supply_index,
        );

        self.reserves().update(|asset_reserve| {
            require!(*asset_reserve >= withdrawal_amount, "insufficient funds");
            *asset_reserve -= &withdrawal_amount;
        });

        self.supplied_amount().update(|asset_supplied_amount| {
            require!(*asset_supplied_amount >= amount, "insufficient funds");
            *asset_supplied_amount -= &amount;
        });
        deposit_position.amount -= &amount;

        self.send()
            .direct_esdt(&initial_caller, &pool_asset, 0, &withdrawal_amount);

        deposit_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint]
    fn repay(
        &self,
        initial_caller: ManagedAddress,
        borrow_position: BorrowPosition<Self::Api>,
    ) -> BorrowPosition<Self::Api> {
        let (received_asset, mut received_amount) = self.call_value().single_fungible_esdt();
        let pool_asset = self.pool_asset().get();

        self.require_non_zero_address(&initial_caller);
        self.require_amount_greater_than_zero(&received_amount);
        require!(
            received_asset == pool_asset,
            "asset not supported for this liquidity pool"
        );

        self.update_interest_indexes();

        let accumulated_debt = self.get_debt_interest(
            &borrow_position.amount,
            &borrow_position.initial_borrow_index,
        );

        let mut ret_borrow_position = self.update_borrows_with_debt(borrow_position);

        let total_owed_with_interest = ret_borrow_position.amount.clone();

        if received_amount >= total_owed_with_interest {
            let extra_amount = &received_amount - &total_owed_with_interest;
            self.send()
                .direct_esdt(&initial_caller, &received_asset, 0, &extra_amount);
            received_amount -= &extra_amount;
            ret_borrow_position.amount = BigUint::zero();
        } else {
            let principal_amount = &received_amount - &accumulated_debt;
            ret_borrow_position.amount -= &principal_amount;
        }

        let amount_without_interest = &received_amount - &accumulated_debt;
        self.borrowed_amount()
            .update(|total| *total -= amount_without_interest);

        self.reserves().update(|total| *total += &received_amount);

        ret_borrow_position
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(repayNFTs)]
    fn repay_nfts(
        &self,
        initial_caller: ManagedAddress,
        borrow_positions: &MultiValueEncoded<
            MultiValue2<EsdtTokenPayment, BorrowPosition<Self::Api>>,
        >,
    ) -> MultiValueEncoded<MultiValue2<EsdtTokenPayment, BorrowPosition<Self::Api>>> {
        let (received_asset, received_amount) = self.call_value().single_fungible_esdt();
        let pool_asset = self.pool_asset().get();

        self.require_non_zero_address(&initial_caller);
        self.require_amount_greater_than_zero(&received_amount);
        require!(
            received_asset == pool_asset,
            "Asset not supported for this liquidity pool"
        );

        self.update_interest_indexes();

        let mut total_received_amount = received_amount.clone();
        let mut total_accumulated_debt = BigUint::zero();
        let mut total_amount_paid = BigUint::zero();
        let mut vec_borrow_positions: MultiValueEncoded<
            MultiValue2<EsdtTokenPayment<Self::Api>, BorrowPosition<Self::Api>>,
        > = MultiValueEncoded::new();
        for data in borrow_positions.clone().into_iter() {
            let (token, borrow_position) = data.into_tuple();
            let accumulated_debt = self.get_debt_interest(
                &borrow_position.amount,
                &borrow_position.initial_borrow_index,
            );
            total_accumulated_debt += &accumulated_debt;

            let mut ret_borrow_position = self.update_borrows_with_debt(borrow_position);

            if &total_received_amount >= &ret_borrow_position.amount {
                total_received_amount -= &ret_borrow_position.amount;
                total_amount_paid += &ret_borrow_position.amount;
                ret_borrow_position.amount = BigUint::zero();
            } else if total_received_amount > accumulated_debt
                && total_received_amount > BigUint::zero()
            {
                let remaining_amount = &total_received_amount - &accumulated_debt;
                ret_borrow_position.amount -= &remaining_amount;
                total_amount_paid += total_received_amount;
                total_received_amount = BigUint::zero();
            }
            vec_borrow_positions.push(MultiValue2::from((token, ret_borrow_position)));
        }

        if received_amount > total_amount_paid {
            self.send().direct_esdt(
                &initial_caller,
                &received_asset,
                0,
                &(received_amount - &total_amount_paid),
            );
        }

        self.borrowed_amount()
            .update(|total| *total -= &total_amount_paid - &total_accumulated_debt);

        self.reserves().update(|total| *total += &total_amount_paid);
        vec_borrow_positions
    }

    #[only_owner]
    #[endpoint(sendTokens)]
    fn send_tokens(&self, initial_caller: ManagedAddress, payment_amount: BigUint) {
        let pool_asset = self.pool_asset().get();

        self.send()
            .direct_esdt(&initial_caller, &pool_asset, 0, &payment_amount);
    }
}

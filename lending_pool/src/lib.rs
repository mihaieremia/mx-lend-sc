#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod factory;
mod math;
mod proxy;
pub mod router;
pub mod storage;
pub mod utils;

pub use common_structs::*;
pub use common_tokens::*;

use liquidity_pool::liquidity::ProxyTrait as _;
use multiversx_sc::codec::Empty;

#[multiversx_sc::contract]
pub trait LendingPool:
    factory::FactoryModule
    + router::RouterModule
    + common_checks::ChecksModule
    + common_tokens::AccountTokenModule
    + proxy::ProxyModule
    + storage::LendingStorageModule
    + utils::LendingUtilsModule
    + math::LendingMathModule
    + price_aggregator_proxy::PriceAggregatorModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[init]
    fn init(&self, lp_template_address: ManagedAddress) {
        self.liq_pool_template_address().set(&lp_template_address);
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerAccountToken)]
    fn register_account_token(&self, token_name: ManagedBuffer, ticker: ManagedBuffer) {
        let payment_amount = self.call_value().egld_value();
        self.account_token().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            payment_amount.clone_value(),
            token_name,
            ticker,
            1,
            None,
        );
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerDebtNFTToken)]
    fn register_debt_token(&self, token_name: ManagedBuffer, ticker: ManagedBuffer) {
        let payment_amount = self.call_value().egld_value();
        self.debt_nft_token().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            payment_amount.clone_value(),
            token_name,
            ticker,
            1,
            None,
        );
    }

    #[endpoint(enterMarket)]
    fn enter_market(&self) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let nft_account_amount = BigUint::from(1u64);

        let nft_token_payment =
            self.account_token()
                .nft_create_and_send(&caller, nft_account_amount, &Empty);
        self.account_positions()
            .insert(nft_token_payment.token_nonce);

        nft_token_payment
    }

    #[endpoint(exitMarket)]
    fn exit_market(&self) {
        let (_nft_account_token_id, nft_account_nonce, nft_account_amount) =
            self.call_value().single_esdt().into_tuple();

        self.account_token()
            .nft_burn(nft_account_nonce, &nft_account_amount);
        self.account_positions().swap_remove(&nft_account_nonce);
    }

    #[payable("*")]
    #[endpoint(addCollateral)]
    fn add_collateral(&self) {
        let [nft_account_token, collateral_payment] = self.call_value().multi_esdt();
        let (nft_account_token_id, nft_account_nonce, nft_account_amount) =
            nft_account_token.into_tuple();
        let (collateral_token_id, collateral_nonce, collateral_amount) =
            collateral_payment.into_tuple();
        let pool_address = self.get_pool_address(&collateral_token_id);
        let initial_caller = self.blockchain().get_caller();

        self.require_asset_supported(&collateral_token_id);
        self.lending_account_in_the_market(nft_account_nonce);
        self.lending_account_token_valid(nft_account_token_id.clone());
        self.require_amount_greater_than_zero(&collateral_amount);
        self.require_non_zero_address(&initial_caller);

        let initial_or_new_deposit_position = self.get_existing_or_new_deposit_position_for_token(
            nft_account_nonce,
            collateral_token_id.clone(),
        );

        let return_deposit_position = self
            .liquidity_pool_proxy(pool_address)
            .add_collateral(initial_or_new_deposit_position)
            .with_esdt_transfer((
                collateral_token_id.clone(),
                collateral_nonce,
                collateral_amount,
            ))
            .execute_on_dest_context();

        self.deposit_positions(nft_account_nonce)
            .insert(collateral_token_id, return_deposit_position);

        // Return NFT to owner
        self.send().direct_esdt(
            &initial_caller,
            &nft_account_token_id,
            nft_account_nonce,
            &nft_account_amount,
        );
    }

    #[payable("*")]
    #[endpoint(removeCollateral)]
    fn remove_collateral(&self, withdraw_token_id: TokenIdentifier, amount: BigUint) {
        let (nft_account_token_id, nft_account_nonce, nft_account_amount) =
            self.call_value().single_esdt().into_tuple();
        let initial_caller = self.blockchain().get_caller();
        let pool_address = self.get_pool_address(&withdraw_token_id);

        self.require_asset_supported(&withdraw_token_id);
        self.lending_account_in_the_market(nft_account_nonce);
        self.lending_account_token_valid(nft_account_token_id.clone());
        self.require_amount_greater_than_zero(&amount);
        self.require_non_zero_address(&initial_caller);
        require!(
            amount
                > self.get_collateral_amount_for_token(
                    nft_account_nonce,
                    nft_account_token_id.clone()
                ),
            "Not enough tokens deposited for this account!"
        );

        let mut dep_pos_map = self.deposit_positions(nft_account_nonce);
        match dep_pos_map.get(&withdraw_token_id) {
            Some(dp) => {
                let deposit_position: DepositPosition<<Self as ContractBase>::Api> = self
                    .liquidity_pool_proxy(pool_address)
                    .remove_collateral(&initial_caller, amount, dp)
                    .execute_on_dest_context();

                if deposit_position.amount == 0 {
                    dep_pos_map.remove(&withdraw_token_id);
                } else {
                    dep_pos_map.insert(withdraw_token_id, deposit_position);
                }
            }
            None => panic!(
                "Tokens {} are not available for this account", // maybe was liquidated already
                withdraw_token_id
            ),
        };
        // Return NFT to owner
        self.send().direct_esdt(
            &initial_caller,
            &nft_account_token_id,
            nft_account_nonce,
            &nft_account_amount,
        );
    }

    #[payable("*")]
    #[endpoint]
    fn borrow(&self, asset_to_borrow: TokenIdentifier, amount: BigUint) {
        let (nft_account_token_id, nft_account_nonce, nft_account_amount) =
            self.call_value().single_esdt().into_tuple();
        let initial_caller = self.blockchain().get_caller();
        let borrow_token_pool_address = self.get_pool_address(&asset_to_borrow);
        let loan_to_value = self.get_loan_to_value_exists_and_non_zero(&asset_to_borrow);

        self.require_asset_supported(&asset_to_borrow);
        self.lending_account_in_the_market(nft_account_nonce);
        self.lending_account_token_valid(nft_account_token_id.clone());
        self.require_amount_greater_than_zero(&amount);
        self.require_non_zero_address(&initial_caller);

        self.update_collateral_with_interest(nft_account_nonce);
        self.update_borrows_with_debt(nft_account_nonce);

        let collateral_in_dollars = self.get_total_collateral_in_dollars(nft_account_nonce);
        let borrowed_amount_in_dollars = self.get_total_borrow_in_dollars(nft_account_nonce);
        let amount_to_borrow_in_dollars =
            amount.clone() * self.get_token_price_data(&asset_to_borrow).price;

        require!(
            collateral_in_dollars * loan_to_value
                > (borrowed_amount_in_dollars + amount_to_borrow_in_dollars),
            "Not enough collateral available for this loan!"
        );

        let initial_borrow_position = self.get_existing_or_new_borrow_position_for_token(
            nft_account_nonce,
            asset_to_borrow.clone(),
        );

        let borrow_position: BorrowPosition<Self::Api> = self
            .liquidity_pool_proxy(borrow_token_pool_address)
            .borrow(&initial_caller, amount, initial_borrow_position)
            .execute_on_dest_context();

        if borrow_position.amount == 0 {
            // Update BorrowPosition
            self.borrow_positions(nft_account_nonce)
                .remove(&asset_to_borrow);
        } else {
            // Update BorrowPosition if it's not empty
            self.borrow_positions(nft_account_nonce)
                .insert(asset_to_borrow, borrow_position);
        }

        // Return NFT account to owner
        self.send().direct_esdt(
            &initial_caller,
            &nft_account_token_id,
            nft_account_nonce,
            &nft_account_amount,
        );
    }

    #[payable("*")]
    #[endpoint(borrowWithNFTs)]
    fn borrow_with_nfts(
        &self,
        asset_to_borrow: TokenIdentifier<Self::Api>,
        amount: BigUint<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let payments = self.call_value().all_esdt_transfers();
        let initial_caller = self.blockchain().get_caller();
        let borrow_token_pool_address = self.get_pool_address(&asset_to_borrow);

        self.require_asset_supported(&asset_to_borrow);
        self.require_amount_greater_than_zero(&amount);
        self.require_non_zero_address(&initial_caller);

        let map_collections = self.collections();

        let mut payments_out: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();
        let borrow_amount_usd = self.get_token_price_data(&asset_to_borrow).price;
        let amount_to_borrow_in_dollars = borrow_amount_usd * amount.clone();
        let egld_usd_price = self.get_egld_price_data().price;
        let mut total_collateral_nfts = BigUint::zero();
        let mut borrow_positions: ManagedVec<BorrowPosition<Self::Api>> = ManagedVec::new();
        let mut original_total_amount = amount.clone();
        for payment in payments.iter() {
            let collection_exists = map_collections.contains(&payment.token_identifier);
            require!(collection_exists, "Collection is not allowed as collateral");
            let collection_params = self.collection_params(&payment.token_identifier).get();
            let max_borrow = (collection_params.floor * &payment.amount) * collection_params.ltv;
            if max_borrow <= original_total_amount {
                total_collateral_nfts += &max_borrow;
                // reduce the amount to borrow with the amount borrowed from the NFT
                original_total_amount -= &max_borrow;

                // borrow the full capacity of the NFT until the amount to borrow is covered
                borrow_positions.push(BorrowPosition::new(
                    asset_to_borrow.clone(),
                    max_borrow,
                    0,
                    self.blockchain().get_block_round(),
                    BigUint::from(BP),
                    Option::Some(payment),
                ));
            } else {
                if original_total_amount == BigUint::zero() {
                    // return extra NFTs to owner, because the amount to borrow is already covered
                    payments_out.push(payment.clone());
                } else {
                    // borrow the rest of the amount and not the full capacity of the NFT
                    borrow_positions.push(BorrowPosition::new(
                        asset_to_borrow.clone(),
                        original_total_amount,
                        0,
                        self.blockchain().get_block_round(),
                        BigUint::from(BP),
                        Option::Some(payment),
                    ));

                    total_collateral_nfts += &max_borrow;
                    // reset the original amount to zero, because the amount to borrow is already covered
                    original_total_amount = BigUint::zero();
                }
            }
        }

        require!(
            total_collateral_nfts * egld_usd_price > amount_to_borrow_in_dollars,
            "Not enough collateral available for this loan!"
        );

        let borrow_positions: ManagedVec<BorrowPosition<Self::Api>> = self
            .liquidity_pool_proxy(borrow_token_pool_address)
            .borrow_bulk_nfts(&initial_caller, amount, borrow_positions)
            .execute_on_dest_context();
        let sc = self.blockchain().get_sc_address();
        let debt_token = self.debt_nft_token().get_token_id();
        for last_position in &borrow_positions {
            let real_nft = last_position.nft.as_ref().unwrap();
            let nft_data = self.blockchain().get_esdt_token_data(
                &sc,
                &real_nft.token_identifier,
                real_nft.token_nonce,
            );

            let nft_nonce = self.send().esdt_nft_create::<BorrowPosition<Self::Api>>(
                &debt_token,
                &BigUint::from(1u32),
                &sc_format!("xDebt - {}", nft_data.name),
                &BigUint::from(0u32),
                &nft_data.hash,
                &last_position,
                &nft_data.uris,
            );

            self.nft_borrow_positions(nft_nonce).set(last_position);
            payments_out.push(EsdtTokenPayment::new(
                debt_token.clone(),
                nft_nonce,
                BigUint::from(1u32),
            ));
        }

        self.send().direct_multi(&initial_caller, &payments_out);
        payments_out
    }

    #[payable("*")]
    #[endpoint]
    fn repay(&self) {
        let [nft_account_token, payment_repay] = self.call_value().multi_esdt();
        let (nft_account_token_id, nft_account_nonce, nft_account_amount) =
            nft_account_token.into_tuple();
        let (repay_token_id, repay_nonce, repay_amount) = payment_repay.into_tuple();
        let initial_caller = self.blockchain().get_caller();
        let asset_address = self.get_pool_address(&repay_token_id);

        self.lending_account_in_the_market(nft_account_nonce);
        self.lending_account_token_valid(nft_account_token_id.clone());
        self.require_asset_supported(&repay_token_id);
        self.require_amount_greater_than_zero(&repay_amount);
        self.require_non_zero_address(&initial_caller);

        match self
            .borrow_positions(nft_account_nonce)
            .get(&repay_token_id)
        {
            Some(bp) => {
                let borrow_position: BorrowPosition<Self::Api> = self
                    .liquidity_pool_proxy(asset_address)
                    .repay(&initial_caller, bp)
                    .with_esdt_transfer((repay_token_id.clone(), repay_nonce, repay_amount))
                    .execute_on_dest_context();

                // Update BorrowPosition
                self.borrow_positions(nft_account_nonce)
                    .remove(&repay_token_id);
                if borrow_position.amount != 0 {
                    self.borrow_positions(nft_account_nonce)
                        .insert(repay_token_id, borrow_position);
                }
            }
            None => panic!(
                "Borrowed tokens {} are not available for this account",
                repay_token_id
            ),
        };
        // Return NFT to owner
        self.send().direct_esdt(
            &initial_caller,
            &nft_account_token_id,
            nft_account_nonce,
            &nft_account_amount,
        );
    }

    // Retrieve information about the repayment and the NFTs
    fn get_repay_and_nft_info(
        &self,
        all_tokens: ManagedRef<ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>>,
    ) -> (
        TokenIdentifier<Self::Api>,
        u64,
        BigUint<Self::Api>,
        ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) {
        require!(
            all_tokens.len() > 1,
            "Minimum 2 tokens required for this operation"
        );
        let (repay_token_id, repay_nonce, repay_amount) = all_tokens.get(0).into_tuple();
        let nft_tokens = all_tokens.slice(1, all_tokens.len()).unwrap();
        (repay_token_id, repay_nonce, repay_amount, nft_tokens)
    }

    // Process the NFTs
    fn process_nfts(
        &self,
        nft_tokens: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
        repay_token_id: &TokenIdentifier<Self::Api>,
    ) -> MultiValueEncoded<MultiValue2<EsdtTokenPayment<Self::Api>, BorrowPosition<Self::Api>>>
    {
        let mut vec_borrow_positions = MultiValueEncoded::new();
        for debt_nft in nft_tokens.iter() {
            let (debt_token, debt_nonce, _) = debt_nft.clone().into_tuple();
            self.debt_nft_token().require_same_token(&debt_token);
            let debt_data = self
                .debt_nft_token()
                .get_token_attributes::<BorrowPosition<Self::Api>>(debt_nonce);
            require!(
                debt_data.token_id == *repay_token_id,
                "Repayment token must be the same as the debt token"
            );
            vec_borrow_positions.push(MultiValue2::from((debt_nft, debt_data)));
        }
        vec_borrow_positions
    }

    #[payable("*")]
    #[endpoint(repayNFT)]
    fn repay_nft_debt(&self) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let all_tokens = self.call_value().all_esdt_transfers();
        let (repay_token_id, repay_nonce, repay_amount, nft_tokens) =
            self.get_repay_and_nft_info(all_tokens);

        let initial_caller = self.blockchain().get_caller();
        let asset_address = self.get_pool_address(&repay_token_id);

        self.require_asset_supported(&repay_token_id);
        self.require_amount_greater_than_zero(&repay_amount);
        self.require_non_zero_address(&initial_caller);

        let vec_borrow_positions = self.process_nfts(&nft_tokens, &repay_token_id);

        let processed_positions: MultiValueEncoded<
            MultiValue2<EsdtTokenPayment<Self::Api>, BorrowPosition<Self::Api>>,
        > = self
            .liquidity_pool_proxy(asset_address)
            .repay_nfts(&initial_caller, vec_borrow_positions)
            .with_esdt_transfer((repay_token_id, repay_nonce, repay_amount))
            .execute_on_dest_context();

        let mut payments_out: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();
        for positions in processed_positions.into_iter() {
            let (token, borrow_pos) = positions.into_tuple();
            if borrow_pos.amount != 0 {
                self.nft_borrow_positions(token.token_nonce).set(borrow_pos);
                payments_out.push(token);
            } else {
                self.nft_borrow_positions(token.token_nonce).clear();
                self.debt_nft_token()
                    .nft_burn(token.token_nonce, &token.amount);
                let real_nft = borrow_pos.nft.unwrap();
                payments_out.push(EsdtTokenPayment::new(
                    real_nft.token_identifier,
                    real_nft.token_nonce,
                    real_nft.amount,
                ));
            }
        }
        self.send().direct_multi(&initial_caller, &payments_out);
        payments_out
    }

    #[payable("*")]
    #[endpoint(liquidate)]
    fn liquidate(
        &self,
        liquidatee_account_nonce: u64,
        liquidation_threshold: BigUint,
        token_to_liquidate: TokenIdentifier,
    ) {
        let (liquidator_asset_token_id, liquidator_asset_amount) =
            self.call_value().single_fungible_esdt();
        let bp = BigUint::from(BP);

        let initial_caller = self.blockchain().get_caller();

        // Liquidatee is in the market; Liquidator doesn't have to be in the Lending Protocol
        self.lending_account_in_the_market(liquidatee_account_nonce);
        self.require_asset_supported(&liquidator_asset_token_id);
        self.require_amount_greater_than_zero(&liquidator_asset_amount);
        self.require_non_zero_address(&initial_caller);
        require!(
            token_to_liquidate == liquidator_asset_token_id,
            "Token sent is not the same as the liquidation token!"
        );

        require!(
            liquidation_threshold <= MAX_THRESHOLD,
            MAX_THRESHOLD_ERROR_MSG
        );

        let liq_bonus = self.get_liquidation_bonus_non_zero(&liquidator_asset_token_id);
        let total_collateral_in_dollars =
            self.get_total_collateral_in_dollars(liquidatee_account_nonce);
        let borrowed_value_in_dollars = self.get_total_borrow_in_dollars(liquidatee_account_nonce);

        let health_factor = self.compute_health_factor(
            &total_collateral_in_dollars,
            &borrowed_value_in_dollars,
            &liquidation_threshold,
        );
        require!(health_factor < BP, "health not low enough for liquidation");

        let liquidator_asset_data = self.get_token_price_data(&liquidator_asset_token_id);
        let liquidator_asset_value_in_dollars =
            liquidator_asset_amount.clone() * liquidator_asset_data.price;

        let amount_needed_for_liquidation = borrowed_value_in_dollars * liquidation_threshold / &bp;
        require!(
            liquidator_asset_value_in_dollars >= amount_needed_for_liquidation,
            "insufficient funds for liquidation"
        );

        // amount_liquidated (1 + liq_bonus)
        let amount_to_return_to_liquidator_in_dollars =
            (liquidator_asset_amount * (&bp + &liq_bonus)) / bp;

        // Go through all DepositPositions and send amount_to_return_in_dollars to Liquidator
        let amount_to_send = self.compute_amount_in_tokens(
            liquidatee_account_nonce,
            token_to_liquidate.clone(),
            amount_to_return_to_liquidator_in_dollars,
        );

        let asset_address = self.get_pool_address(&token_to_liquidate);

        let _: IgnoreValue = self
            .liquidity_pool_proxy(asset_address)
            .send_tokens(&initial_caller, &amount_to_send)
            .execute_on_dest_context();
    }

    #[endpoint(updateCollateralWithInterest)]
    fn update_collateral_with_interest(&self, account_position: u64) {
        let deposit_positions = self.deposit_positions(account_position);

        for dp in deposit_positions.values() {
            let asset_address = self.get_pool_address(&dp.token_id);
            let _: IgnoreValue = self
                .liquidity_pool_proxy(asset_address)
                .update_collateral_with_interest(dp)
                .execute_on_dest_context();
        }
    }

    #[endpoint(updateBorrowsWithDebt)]
    fn update_borrows_with_debt(&self, account_position: u64) {
        let borrow_positions = self.borrow_positions(account_position);

        for bp in borrow_positions.values() {
            let asset_address = self.get_pool_address(&bp.token_id);
            let _: IgnoreValue = self
                .liquidity_pool_proxy(asset_address)
                .update_borrows_with_debt(bp)
                .execute_on_dest_context();
        }
    }

    fn caller_from_option_or_sender(
        &self,
        caller: OptionalValue<ManagedAddress>,
    ) -> ManagedAddress {
        caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller())
    }

    fn require_asset_supported(&self, asset: &TokenIdentifier) {
        require!(self.pools_map().contains_key(asset), "asset not supported");
    }

    fn lending_account_in_the_market(&self, nonce: u64) {
        require!(
            self.account_positions().contains(&nonce),
            "Account not in Lending Protocol!"
        );
    }

    fn lending_account_token_valid(&self, account_token_id: TokenIdentifier) {
        require!(
            account_token_id == self.account_token().get_token_id(),
            "Account token not valid!"
        );
    }
}

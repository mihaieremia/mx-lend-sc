#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod factory;
mod proxy_common;
mod router;

pub use common_structs::*;

use liquidity_pool::multi_transfer;

use liquidity_pool::liquidity::ProxyTrait as _;
use liquidity_pool::tokens::ProxyTrait as _;

#[elrond_wasm::contract]
pub trait LendingPool:
    factory::FactoryModule
    + router::RouterModule
    + multi_transfer::MultiTransferModule
    + proxy_common::ProxyCommonModule
{
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(deposit)]
    fn deposit_endpoint(
        &self,
        #[var_args] caller: OptionalArg<Address>,
        #[payment_token] asset: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
    ) -> SCResult<()> {
        let initial_caller = caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());

        require!(amount > 0, "amount must be greater than 0");
        require!(!initial_caller.is_zero(), "invalid address provided");

        let pool_address = self.get_pool_address(&asset);
        require!(!pool_address.is_zero(), "invalid liquidity pool address");

        self.liquidity_pool_proxy(pool_address)
            .deposit_asset(initial_caller, asset, amount)
            .execute_on_dest_context();

        Ok(())
    }

    #[payable("*")]
    #[endpoint(withdraw)]
    fn withdraw_endpoint(
        &self,
        #[payment_token] lend_token: TokenIdentifier,
        #[payment_nonce] token_nonce: u64,
        #[payment_amount] amount: Self::BigUint,
        #[var_args] caller: OptionalArg<Address>,
    ) -> SCResult<()> {
        let initial_caller = caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());

        require!(amount > 0, "amount must be greater than 0");
        require!(!initial_caller.is_zero(), "invalid address provided");

        let pool_address = self.get_pool_address(&lend_token);
        require!(!pool_address.is_zero(), "invalid liquidity pool address");

        self.liquidity_pool_proxy(pool_address)
            .withdraw(initial_caller, lend_token, token_nonce, amount)
            .execute_on_dest_context();

        Ok(())
    }

    #[payable("*")]
    #[endpoint(repay)]
    fn repay_endpoint(
        &self,
        asset_to_repay: TokenIdentifier,
        #[var_args] caller: OptionalArg<Address>,
    ) -> SCResult<()> {
        let initial_caller = caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());

        let asset_address = self.get_pool_address(&asset_to_repay);
        require!(
            self.pools_map().contains_key(&asset_to_repay),
            "asset not supported"
        );

        // TODO: Use SC Proxy instead of manual call in 0.19.0

        let transfers = self.get_all_esdt_transfers();
        let raw_results = self.multi_transfer_via_execute_on_dest_context(
            &asset_address,
            &transfers,
            &b"repay"[..].into(),
            &[initial_caller.as_bytes().into()],
        );

        let collateral_id = TokenIdentifier::top_decode(raw_results[0].as_slice())?;
        let collateral_amount_repaid = Self::BigUint::top_decode(raw_results[1].as_slice())?;
        let borrow_timestamp = u64::top_decode(raw_results[2].as_slice())?;

        let collateral_token_address = self.get_pool_address(&collateral_id);
        require!(
            !collateral_token_address.is_zero(),
            "Collateral not supported"
        );

        self.liquidity_pool_proxy(collateral_token_address)
            .mint_l_tokens(
                collateral_id,
                collateral_amount_repaid,
                initial_caller,
                borrow_timestamp,
            )
            .execute_on_dest_context_ignore_result();

        Ok(())
    }

    #[payable("*")]
    #[endpoint(liquidate)]
    fn liquidate_endpoint(
        &self,
        liquidate_unique_id: BoxedBytes,
        #[var_args] initial_caller: OptionalArg<Address>,
        #[payment_token] asset: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
    ) -> SCResult<()> {
        let caller = initial_caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());

        require!(amount > 0, "amount must be greater than 0");
        require!(!caller.is_zero(), "invalid address provided");
        require!(self.pools_map().contains_key(&asset), "asset not supported");

        let asset_address = self.get_pool_address(&asset);

        let results = self
            .liquidity_pool_proxy(asset_address)
            .liquidate(liquidate_unique_id, asset, amount)
            .execute_on_dest_context();

        let collateral_token_address = self.get_pool_address(&results.collateral_token);

        require!(
            collateral_token_address != Address::zero(),
            "asset is not supported"
        );

        self.liquidity_pool_proxy(collateral_token_address)
            .mint_l_tokens(
                results.collateral_token,
                results.amount,
                caller,
                self.blockchain().get_block_timestamp(),
            )
            .execute_on_dest_context_ignore_result();

        Ok(())
    }

    #[payable("*")]
    #[endpoint(borrow)]
    fn borrow_endpoint(
        &self,
        asset_collateral: TokenIdentifier,
        asset_to_borrow: TokenIdentifier,
        #[var_args] caller: OptionalArg<Address>,
        #[payment_token] payment_lend_id: TokenIdentifier,
        #[payment_nonce] payment_nonce: u64,
        #[payment_amount] payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        let initial_caller = caller
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());

        require!(payment_amount > 0, "amount must be greater than 0");
        require!(!initial_caller.is_zero(), "invalid address provided");
        require!(payment_nonce != 0, "payment token be a lend token");

        let collateral_token_pool_address = self.get_pool_address_non_zero(&asset_collateral)?;
        let borrow_token_pool_address = self.get_pool_address_non_zero(&asset_to_borrow)?;
        let lend_token_pool_address = self.get_pool_address_non_zero(&payment_lend_id)?;
        require!(
            collateral_token_pool_address == lend_token_pool_address,
            "Collateral and lend pool addresses differ"
        );
        require!(
            collateral_token_pool_address != borrow_token_pool_address,
            "Collateral and borrow pool addresses are the same"
        );

        let metadata = self.get_interest_metadata(&payment_lend_id, payment_nonce)?;
        self.liquidity_pool_proxy(borrow_token_pool_address)
            .borrow(
                initial_caller.clone(),
                asset_collateral,
                payment_amount.clone(),
                metadata.timestamp,
            )
            .execute_on_dest_context_ignore_result();

        self.liquidity_pool_proxy(collateral_token_pool_address)
            .burn_l_tokens(
                payment_lend_id,
                payment_nonce,
                payment_amount,
                initial_caller,
            )
            .execute_on_dest_context_ignore_result();

        Ok(())
    }

    fn get_interest_metadata(
        &self,
        token_id: &TokenIdentifier,
        nonce: u64,
    ) -> SCResult<InterestMetadata> {
        let esdt_nft_data = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            nonce,
        );
        esdt_nft_data.decode_attributes().into()
    }
}

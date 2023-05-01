use multiversx_sc::{storage::mappers::StorageTokenWrapper, types::Address};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    whitebox::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use price_aggregator_proxy::PriceAggregatorModule;

use crate::constants::*;
use aggregator_mock::PriceAggregatorMock;
use lending_pool::{AccountTokenModule, LendingPool};

pub fn setup_price_aggregator<PriceAggregatorObjBuilder>(
    owner_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    builder: PriceAggregatorObjBuilder,
) -> ContractObjWrapper<aggregator_mock::ContractObj<DebugApi>, PriceAggregatorObjBuilder>
where
    PriceAggregatorObjBuilder: 'static + Copy + Fn() -> aggregator_mock::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let price_aggregator_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(owner_addr),
        builder,
        PRICE_AGGREGATOR_WASM_PATH,
    );

    b_mock
        .execute_tx(owner_addr, &price_aggregator_wrapper, &rust_zero, |sc| {
            sc.set_latest_price_feed(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(DOLLAR_TICKER),
                managed_biguint!(EGLD_PRICE_IN_DOLLARS),
            );
            sc.set_latest_price_feed(
                managed_buffer!(USDC_TICKER),
                managed_buffer!(DOLLAR_TICKER),
                managed_biguint!(USDC_PRICE_IN_DOLLARS),
            );
        })
        .assert_ok();

    price_aggregator_wrapper
}

pub fn setup_lending_pool<LendingPoolObjBuilder, PriceAggregatorObjBuilder>(
    owner_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    builder: LendingPoolObjBuilder,
    price_aggregator_builder: PriceAggregatorObjBuilder,
    _template: &Address,
) -> ContractObjWrapper<lending_pool::ContractObj<DebugApi>, LendingPoolObjBuilder>
where
    LendingPoolObjBuilder: 'static + Copy + Fn() -> lending_pool::ContractObj<DebugApi>,
    PriceAggregatorObjBuilder: 'static + Copy + Fn() -> aggregator_mock::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let lending_pool_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(owner_addr),
        builder,
        LENDING_POOL_WASM_PATH,
    );
    let lp_template_addr =
        setup_liquidity_pool_template(owner_addr, b_mock, liquidity_pool::contract_obj);
    let price_aggregator_wrapper =
        setup_price_aggregator(&owner_addr, b_mock, price_aggregator_builder);

    b_mock
        .execute_tx(&owner_addr, &lending_pool_wrapper, &rust_zero, |sc| {
            sc.init(managed_address!(&lp_template_addr));
            sc.set_price_aggregator_address(managed_address!(
                &price_aggregator_wrapper.address_ref()
            ));

            let account_token_id = managed_token_id!(ACCOUNT_TOKEN);
            sc.account_token().set_token_id(account_token_id);
        })
        .assert_ok();

    lending_pool_wrapper
}

pub fn setup_liquidity_pool_template<LiquidityPoolObjBuilder>(
    owner_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    builder: LiquidityPoolObjBuilder,
) -> Address
where
    LiquidityPoolObjBuilder: 'static + Copy + Fn() -> liquidity_pool::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let liquidity_pool_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(owner_addr),
        builder,
        LIQUIDITY_POOL_WASM_PATH,
    );

    liquidity_pool_wrapper.address_ref().clone()
}

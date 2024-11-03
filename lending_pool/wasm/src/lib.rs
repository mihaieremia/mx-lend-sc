// Code generated by the multiversx-sc multi-contract system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Endpoints:                           37
// Async Callback:                       1
// Total number of exported functions:  39

#![no_std]
#![feature(lang_items)]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    lending_pool
    (
        init => init
        registerAccountToken => register_account_token
        registerDebtNFTToken => register_debt_token
        enterMarket => enter_market
        exitMarket => exit_market
        addCollateral => add_collateral
        removeCollateral => remove_collateral
        borrow => borrow
        borrowWithNFTs => borrow_with_nfts
        repay => repay
        repayNFT => repay_nft_debt
        liquidate => liquidate
        updateCollateralWithInterest => update_collateral_with_interest
        updateBorrowsWithDebt => update_borrows_with_debt
        getLiqPoolTemplateAddress => liq_pool_template_address
        createLiquidityPool => create_liquidity_pool
        upgradeLiquidityPool => upgrade_liquidity_pool
        setAggregator => set_aggregator
        setAssetLoanToValue => set_asset_loan_to_value
        setAssetLiquidationBonus => set_asset_liquidation_bonus
        addCollection => add_collection
        getPoolAddress => get_pool_address
        getPoolAllowed => pools_allowed
        getAssetLoanToValue => asset_loan_to_value
        getAssetLiquidationBonus => asset_liquidation_bonus
        getAccountToken => account_token
        getDebtNFT => debt_nft_token
        getAccountPositions => account_positions
        getDepositPositions => deposit_positions
        getBorrowPositions => borrow_positions
        getNFTBorrowPositions => nft_borrow_positions
        getCollections => collections
        getCollectionParam => collection_params
        getCollateralAmountForToken => get_collateral_amount_for_token
        getTotalCollateralAvailable => get_total_collateral_in_dollars
        getTotalBorrowInDollars => get_total_borrow_in_dollars
        setPriceAggregatorAddress => set_price_aggregator_address
        getAggregatorAddress => price_aggregator_address
    )
}

multiversx_sc_wasm_adapter::async_callback! { lending_pool }

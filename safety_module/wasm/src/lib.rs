// Code generated by the multiversx-sc multi-contract system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Endpoints:                           12
// Async Callback:                       1
// Total number of exported functions:  14

#![no_std]
#![feature(alloc_error_handler, lang_items)]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    safety_module
    (
        addPool
        removePool
        fund
        fundFromPool
        takeFunds
        withdraw
        setLocalRolesNftToken
        pools
        wegld_token
        deposit_apy
        nftToken
        lastErrorMessage
        callBack
    )
}

multiversx_sc::imports!();

use common_structs::{BorrowPosition, CollectionParams, DepositPosition};

#[multiversx_sc::module]
pub trait LendingStorageModule {
    #[view(getDepositPositions)]
    #[storage_mapper("deposit_positions")]
    fn deposit_positions(
        &self,
        owner_nonce: u64,
    ) -> MapMapper<TokenIdentifier, DepositPosition<Self::Api>>;

    #[view(getBorrowPositions)]
    #[storage_mapper("borrow_positions")]
    fn borrow_positions(
        &self,
        owner_nonce: u64,
    ) -> MapMapper<TokenIdentifier, BorrowPosition<Self::Api>>;

    #[view(getNFTBorrowPositions)]
    #[storage_mapper("nft_borrow_positions")]
    fn nft_borrow_positions(&self, nft_nonce: u64) -> SingleValueMapper<BorrowPosition<Self::Api>>;

    #[view(getCollections)]
    #[storage_mapper("collections")]
    fn collections(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[view(getCollectionParam)]
    #[storage_mapper("collection_params")]
    fn collection_params(
        &self,
        token: &TokenIdentifier,
    ) -> SingleValueMapper<CollectionParams<Self::Api>>;
}

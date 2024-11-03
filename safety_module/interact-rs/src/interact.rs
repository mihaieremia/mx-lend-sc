#![allow(non_snake_case)]

use safety_module::ProxyTrait as _;
use safety_module::*;
use multiversx_sc_snippets::{
    multiversx_sc::{
        codec::multi_types::*,
        types::*,
    },
    env_logger,
    erdrs::wallet::Wallet,
    tokio, Interactor,
};
use multiversx_sc_scenario::scenario_model::*;
use multiversx_chain_vm::{
    bech32, scenario_format::interpret_trait::InterpreterContext, ContractInfo, DebugApi,
};


const GATEWAY: &str = multiversx_sdk::blockchain::DEVNET_GATEWAY;
const PEM: &str = "alice.pem";
const SC_ADDRESS: &str = "";

const SYSTEM_SC_BECH32: &str = "erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u";
const DEFAULT_ADDRESS_EXPR: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
const DEFAULT_GAS_LIMIT: u64 = 100_000_000;
const TOKEN_ISSUE_COST: u64 = 50_000_000_000_000_000;

type ContractType = ContractInfo<safety_module::Proxy<DebugApi>>;

#[tokio::main]
async fn main() {
    env_logger::init();
    let _ = DebugApi::dummy();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut state = State::new().await;
    match cmd.as_str() {
        "deploy" => state.deploy().await,
        "addPool" => state.add_pool().await,
        "removePool" => state.remove_pool().await,
        "fund" => state.fund().await,
        "fundFromPool" => state.fund_from_pool().await,
        "takeFunds" => state.take_funds().await,
        "withdraw" => state.withdraw().await,
        "setLocalRolesNftToken" => state.set_local_roles_nft_token().await,
        "pools" => state.pools().await,
        "wegld_token" => state.wegld_token().await,
        "deposit_apy" => state.deposit_apy().await,
        "nftToken" => state.nft_token().await,
        "lastErrorMessage" => state.last_error_message().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

struct State {
    interactor: Interactor,
    wallet_address: Address,
    contract: ContractType,
}

impl State {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(Wallet::from_pem_file(PEM).unwrap());
        let sc_addr_expr = if SC_ADDRESS == "" {
            DEFAULT_ADDRESS_EXPR.to_string()
        } else {
            "bech32:".to_string() + SC_ADDRESS
        };
        let contract = ContractType::new(sc_addr_expr);

        State {
            interactor,
            wallet_address,
            contract,
        }
    }

    async fn deploy(&mut self) {
        let wegld_token = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let depositors_apy = BigUint::<DebugApi>::from(0u128);

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_deploy(
                self.contract
                    .init(wegld_token, depositors_apy)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .code_metadata(CodeMetadata::all())
                    .contract_code("file:../output/safety-module.wasm", &InterpreterContext::default())
                    .gas_limit(DEFAULT_GAS_LIMIT),
            )
            .await;

        let new_address = result.new_deployed_address();
        let new_address_bech32 = bech32::encode(&new_address);
        println!("new address: {}", new_address_bech32);
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn add_pool(&mut self) {
        let token = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let address = bech32::decode("");

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .add_pool(token, address)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn remove_pool(&mut self) {
        let token = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .remove_pool(token)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn fund(&mut self) {
        let token_id = b"";
        let token_nonce = 0u64;
        let token_amount = BigUint::<DebugApi>::from(0u128);

        let caller = OptionalValue::Some(bech32::decode(""));

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .fund(caller)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
            .esdt_transfer(token_id.to_vec(), token_nonce, token_amount)

                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn fund_from_pool(&mut self) {
        let token_id = b"";
        let token_nonce = 0u64;
        let token_amount = BigUint::<DebugApi>::from(0u128);

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .fund_from_pool()
                    .into_blockchain_call()
                    .from(&self.wallet_address)
            .esdt_transfer(token_id.to_vec(), token_nonce, token_amount)

                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn take_funds(&mut self) {
        let pool_token = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let amount = BigUint::<DebugApi>::from(0u128);

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .take_funds(pool_token, amount)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn withdraw(&mut self) {
        let token_id = b"";
        let token_nonce = 0u64;
        let token_amount = BigUint::<DebugApi>::from(0u128);

        let result: multiversx_sc_snippets::InteractorResult<BigUint<DebugApi>> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .withdraw()
                    .into_blockchain_call()
                    .from(&self.wallet_address)
            .esdt_transfer(token_id.to_vec(), token_nonce, token_amount)

                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn set_local_roles_nft_token(&mut self) {
        let roles = PlaceholderInput;

        let result: multiversx_sc_snippets::InteractorResult<()> = self
            .interactor
            .sc_call_get_result(
                self.contract
                    .set_local_roles_nft_token(roles)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .gas_limit(DEFAULT_GAS_LIMIT)
                    .into(),
            )
            .await;
        let result_value = result.value();

        println!("Result: {:?}", result_value);
    }

    async fn pools(&mut self) {
        let token = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let result_value: ManagedAddress<DebugApi> = self
            .interactor
            .vm_query(self.contract.pools(token))
            .await;

        println!("Result: {:?}", result_value);
    }

    async fn wegld_token(&mut self) {
        let result_value: TokenIdentifier<DebugApi> = self
            .interactor
            .vm_query(self.contract.wegld_token())
            .await;

        println!("Result: {:?}", result_value);
    }

    async fn deposit_apy(&mut self) {
        let result_value: BigUint<DebugApi> = self
            .interactor
            .vm_query(self.contract.deposit_apy())
            .await;

        println!("Result: {:?}", result_value);
    }

    async fn nft_token(&mut self) {
        let result_value: TokenIdentifier<DebugApi> = self
            .interactor
            .vm_query(self.contract.nft_token())
            .await;

        println!("Result: {:?}", result_value);
    }

    async fn last_error_message(&mut self) {
        let result_value: ManagedBuffer<DebugApi> = self
            .interactor
            .vm_query(self.contract.last_error_message())
            .await;

        println!("Result: {:?}", result_value);
    }

}

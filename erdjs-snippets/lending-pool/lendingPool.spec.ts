import { BigIntValue, TokenPayment } from "@elrondnetwork/erdjs";
import { createAirdropService, FiveMinutesInMilliseconds, createESDTInteractor, INetworkProvider, ITestSession, ITestUser, TestSession } from "@elrondnetwork/erdjs-snippets";
import { assert } from "chai";
import { helperAddLiquidityPool, helperAirdropTokens, helperIssueBorrowToken, helperIssueLendToken, helperIssueToken, helperSetAggregatorForLP, helperSetAssetLiquidationBonus, helperSetAssetLoanToValue, helperSetBorrowRoles, helperSetLendRoles } from "./lendingPoolHelper";
import { createLendingInteractor } from "./lendingPoolInteractor";
import { createLiquidityInteractor } from "./liquidityPoolInteractor";
import { createPriceAggregatorInteractor } from "./priceAggregatorPoolInteractor";

describe("lending snippet", async function () {
    this.bail(true);

    let suite = this;
    let session: ITestSession;
    let provider: INetworkProvider;
    let whale: ITestUser;
    let firstUser: ITestUser;
    let secondUser: ITestUser;

    this.beforeAll(async function () {
        session = await TestSession.load("devnet", __dirname);
        provider = session.networkProvider;
        whale = session.users.getUser("whale");
        firstUser = session.users.getUser("firstUser");
        secondUser = session.users.getUser("secondUser");
        await session.syncNetworkConfig();
    });

    this.beforeEach(async function () {
        session.correlation.step = this.currentTest?.fullTitle() || "";
    });

    it("Airdrop EGLD", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        
        let payment = TokenPayment.egldFromAmount(0.1);
        await session.syncUsers([whale]);
        await createAirdropService(session).sendToEachUser(whale, [firstUser, secondUser], [payment]);
    });

    it("Issue Pool Token USDC", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);

        let interactor = await createESDTInteractor(session);
        await session.syncUsers([whale]);
        let token = await interactor.issueFungibleToken(whale, { name: "USDC", ticker: "USD", decimals: 18, supply: "1000000000000000000000" })
        await session.saveToken({ name: "tokenUSD", token: token });
    });

    it("Issue Pool Token EGLD", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);

        let interactor = await createESDTInteractor(session);
        await session.syncUsers([whale]);
        let token = await interactor.issueFungibleToken(whale, { name: "EGLD", ticker: "EGLD", decimals: 18, supply: "1000000000000000000000" })
        await session.saveToken({ name: "tokenEGLD", token: token });
    });

    it("airdrop pool_tokens to users", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        await helperAirdropTokens(session, whale, firstUser, secondUser, "tokenUSD");
        await helperAirdropTokens(session, whale, firstUser, secondUser, "tokenEGLD");

    });


    it("Deploy", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);

        await session.syncUsers([whale]);

        let token = await session.loadToken("tokenUSD");
        let interactor = await createLendingInteractor(session);

        // Deploy dummy liquidity pool
        let { address: dummyAddress, returnCode: dummyReturnCode } = await interactor.deployDummyLiquidityPool(whale, token.identifier);
        assert.isTrue(dummyReturnCode.isSuccess());

        // Deploy lending pool
        let { address, returnCode } = await interactor.deploy(whale, dummyAddress);
        assert.isTrue(returnCode.isSuccess());
        await session.saveAddress({name: "lendingAddr", address: address});
    });

    it("Issue Account Token", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);

        await session.syncUsers([whale]);

        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);

        // Deploy dummy liquidity pool
        let returnCode = await interactor.registerAccountToken(whale, "LAccount", "LACC");
        assert.isTrue(returnCode.isSuccess());
        
    });

    it("Set price aggregator", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        await session.syncUsers([whale, firstUser, secondUser]);
        let tokenUSD = await session.loadToken("tokenUSD");
        let tokenEGLD = await session.loadToken("tokenEGLD");

        let priceAggregatorInteractor = await createPriceAggregatorInteractor(session);
        let { address: priceAggregatorAddress, returnCode: returnCode } = await priceAggregatorInteractor.deployAggregator(whale);

        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);
        await interactor.setPriceAggregatorAddress(whale, priceAggregatorAddress);

        await priceAggregatorInteractor.unpausePoolAggregator(whale);
        await priceAggregatorInteractor.submitPriceAggregator(whale, "USD", "USD", 1000000000000000000);
        await priceAggregatorInteractor.submitPriceAggregator(whale, "EGLD", "USD", 50000000000000000000);
        await session.saveAddress({name: "priceAggregatorAddress", address: priceAggregatorAddress});

    });


    it("Create Liquidity Pool", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);

        await session.syncUsers([whale]);

        let isSuccess = await helperAddLiquidityPool(session, whale, "tokenUSD");
        assert.isTrue(isSuccess);

        isSuccess = await helperAddLiquidityPool(session, whale, "tokenEGLD");
        assert.isTrue(isSuccess);
    });

    it("Setup Liquidity Pools", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        this.retries(5);
        let isSuccess;

        await session.syncUsers([whale]);

        isSuccess = await helperSetAssetLoanToValue(session, whale, "tokenUSD");
        assert.isTrue(isSuccess);

        isSuccess = await helperSetAssetLoanToValue(session, whale, "tokenEGLD");
        assert.isTrue(isSuccess);

        isSuccess = await helperSetAssetLiquidationBonus(session, whale, "tokenUSD");
        assert.isTrue(isSuccess);

        isSuccess = await helperSetAssetLiquidationBonus(session, whale, "tokenEGLD");
        assert.isTrue(isSuccess);

        isSuccess = await helperSetAggregatorForLP(session, whale, "tokenUSD");
        assert.isTrue(isSuccess);

        isSuccess = await helperSetAggregatorForLP(session, whale, "tokenEGLD");

    });

    it("enter market First User", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        
        await session.syncUsers([whale, firstUser]);

        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);
        let { returnCode: returnCodeDeposit, accountNonce: accountNonceFirstUser, accountTokenId: accountTokenIdFirstUser } = await interactor.enter_market(firstUser);
        assert.isTrue(returnCodeDeposit.isSuccess());

        session.saveBreadcrumb({name: "accountNonceFirstUser", value: accountNonceFirstUser})
        session.saveBreadcrumb({name: "accountTokenIdFirstUser", value: accountTokenIdFirstUser})
    });

    it("enter market Second User", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        
        await session.syncUsers([whale, secondUser]);

        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);
        let { returnCode: returnCodeDeposit, accountNonce: accountNonceSecondUser, accountTokenId: accountTokenIdSecondUser } = await interactor.enter_market(secondUser);
        assert.isTrue(returnCodeDeposit.isSuccess());

        session.saveBreadcrumb({name: "accountNonceSecondUser", value: accountNonceSecondUser})
        session.saveBreadcrumb({name: "accountTokenIdSecondUser", value: accountTokenIdSecondUser})

    });

    it("addCollateral token USD", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        
        await session.syncUsers([whale, firstUser]);

        let tokenUSD = await session.loadToken("tokenUSD");
        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);
        let accountNonceFirstUser = await session.loadBreadcrumb("accountNonceFirstUser");
        let accountTokenIdFirstUser = await session.loadBreadcrumb("accountTokenIdFirstUser");

        let paymentAccountNFT = TokenPayment.nonFungible(accountTokenIdFirstUser, accountNonceFirstUser);
        let paymentUSD = TokenPayment.fungibleFromAmount(tokenUSD.identifier, "20", tokenUSD.decimals);

        let returnCode = await interactor.addCollateral(firstUser, paymentAccountNFT, paymentUSD);
        assert.isTrue(returnCode.isSuccess());
    });


    it("addCollateral token EGLD", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        await session.syncUsers([whale, secondUser]);
        
        let tokenEGLD = await session.loadToken("tokenEGLD");
        let address = await session.loadAddress("lendingAddr");
        let interactor = await createLendingInteractor(session, address);
        let accountNonceSecondUser = await session.loadBreadcrumb("accountNonceSecondUser");
        let accountTokenIdSecondUser = await session.loadBreadcrumb("accountTokenIdSecondUser");

        let paymentAccountNFT = TokenPayment.nonFungible(accountTokenIdSecondUser, accountNonceSecondUser);
        let paymentUSD = TokenPayment.fungibleFromAmount(tokenEGLD.identifier, "10", tokenEGLD.decimals);

        let returnCode = await interactor.addCollateral(secondUser, paymentAccountNFT, paymentUSD);
        assert.isTrue(returnCode.isSuccess());
    });

    it("withdraw token EGLD", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        
        await session.syncUsers([secondUser]);

        let tokenEGLD = await session.loadToken("tokenEGLD");
        let lendingAddress = await session.loadAddress("lendingAddr");
        let accountNonceSecondUser = await session.loadBreadcrumb("accountNonceSecondUser");
        let accountTokenIdSecondUser = await session.loadBreadcrumb("accountTokenIdSecondUser");
        let paymentAccountNFT = TokenPayment.nonFungible(accountTokenIdSecondUser, accountNonceSecondUser);

        let lendingInteractor = await createLendingInteractor(session, lendingAddress);
        let returnCode = await lendingInteractor.removeCollateral(secondUser, tokenEGLD.identifier, 5, paymentAccountNFT);
        assert.isTrue(returnCode.isSuccess());
    });

    it("borrow USDC token - First user", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        await session.syncUsers([firstUser]);

        let tokenUSD = await session.loadToken("tokenUSD");
        let lendingAddress = await session.loadAddress("lendingAddr");
        let lendingInteractor = await createLendingInteractor(session, lendingAddress);

        let accountNonceFirstUser = await session.loadBreadcrumb("accountNonceFirstUser");
        let accountTokenIdFirstUser = await session.loadBreadcrumb("accountTokenIdFirstUser");
        let paymentAccountNFT = TokenPayment.nonFungible(accountTokenIdFirstUser, accountNonceFirstUser);

        let returnBorrowCode = await lendingInteractor.borrow(firstUser, tokenUSD.identifier, 7000000000000000000, paymentAccountNFT);
        assert.isTrue(returnBorrowCode.isSuccess());
    });


    it("repay USDC token - First user", async function () {
        this.timeout(FiveMinutesInMilliseconds);
        await session.syncUsers([firstUser]);

        let tokenUSD = await session.loadToken("tokenUSD");
        let lendingAddress = await session.loadAddress("lendingAddr");
        let lendingInteractor = await createLendingInteractor(session, lendingAddress);

        let accountNonceFirstUser = await session.loadBreadcrumb("accountNonceFirstUser");
        let accountTokenIdFirstUser = await session.loadBreadcrumb("accountTokenIdFirstUser");
        let paymentAccountNFT = TokenPayment.nonFungible(accountTokenIdFirstUser, accountNonceFirstUser);
        let paymentUSD = TokenPayment.fungibleFromAmount(tokenUSD.identifier, "7", tokenUSD.decimals);

        let returnCode = await lendingInteractor.repay(firstUser, paymentAccountNFT, paymentUSD);
        assert.isTrue(returnCode.isSuccess());
    });

    it("generate report", async function () {
        await session.generateReport();
    });

    it("destroy session", async function () {
        await session.destroy();
    });
});

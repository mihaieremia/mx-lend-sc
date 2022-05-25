# elrond-lend-rs

## Introduction

As the next stage in the evolution of DeFi, decentralized lending brings more opportunities.
In contrast to decentralized exchanges, lending involves two parties: the *borrower* and the *provider*.
Suppliers deposit assets into a pool to earn interest.
Borrowers can obtain an asset by placing other assets as collateral.
For the borrowed asset, the borrower pays interest *over time* to the suppliers.

Lending Protocol enhanches the opportunities for DeFi participants to further grow the ecosystem:

1. Instead of staking tokens into liquidity mining pools, users can now lend their tokens.
Therefore, users can now earn yield *without* the limitation of *impermanent loss* and not having to hold two tokens at a fixed ratio (as for liquidity pool).

2. Users can deposit their tokens as collateral to borrow other class of tokens.

These benefits are not risk-free.
Borrowers have the risk of *liquidation*, when the borrowing balance exceeds the total collateral value.
Supplieres have the risk of *short fall*, which happens when the liquidation does not recover sufficient obligation. 

The main idea behind the lending/borrowing collateral in a p2p way is that of utilisation ratios.
This means that contracts don’t need to hold state about rates, or enforce rates statically. 
It achieves this by computing the current borrow/deposit rate in a dynamic way, taking the relation between the particular pool’s reserves and the utilisation rate at that moment (amount of collateral borrowed).

![Supply-Borrow Lending Pool (1)](https://user-images.githubusercontent.com/3630188/170192980-f68ca7d9-88bb-4118-a83a-a1a67269f7b4.png)


## Protocol Architecture

We propose a lending strategy based on algorithmic pool-based liquidity.
The Lending Protocol supports multiple Liquidity Pools, which have their own state and a Lending Pool which acts as Router and Proxy for routing and executing transactions based on the transaction input.
The addition of a new asset requires the deployment of another liquidity pool contract.
The Lending Pool deploys and manages the Liquidity Pools as an owner.



### The LendingPool Contract

#### Deposit

The deposit action is the simplest one and does not have any state check.

![Deposit Scenario (5)](https://user-images.githubusercontent.com/3630188/170304702-787c000f-c606-4255-b289-a800e28d625b.png)




The flow is as follows:
1. The user calls `deposit` endpoint from the *Lending Pool SC*, which calls the `deposit_asset` endpoint from the *Liquidity Pool*;
2. The *Liquidity Pool SC* mints metaESDT tokens with an unique nonce. These have the same ticker as the token with an *L* appended in front (ABC -> LABC);
3. The *Liquidity Pool SC* creates a `DepositPosition` which contains the timestamp and the amount of tokens;
4. The *Liquidity Pool SC* updates the reservers (*Reserves Storage Mapper*).
5. The *Liquidity Pool SC* sends the MetaESDT tokens to the user directly.
6. The *Liquidity Pool SC* returns an EsdtTokenPayment with the MetaESDT freshly minted, its nonce and the amount which is the same as the deposited amount (1 TOKEN = 1 LTOKEN). The *Lending Pool* returns the same result to the user.

TL;DR: User sends *TOKEN_A* and receives *LTOKEN_A*.

#### Widthraw

This is the opposite of Deposit scenario.


![Withdraw Scenario (3)](https://user-images.githubusercontent.com/3630188/170304749-7a2adbe7-2cf3-4a22-a7b0-013c69980f81.png)



The flow is as follows:
1. The user calls `withdraw` endpoint from the *Lending Pool SC*, which calls the `withdraw` endpoint from the *Liquidity Pool*;
2. The *Liquidity Pool SC* computes the witdrawal amount (capital deposited + interest accrueled);
3. The *Liquidity Pool SC* updates the reservers and the DepositPosition (the user may have withdraw only a part of the amount deposited)
4. The *Liquidity Pool SC* burns the MetaEsdt tokens sent by the user;
5. The *Liquidity Pool SC* send the original tokens directly to the user;

TL;DR: User sends *LTOKEN_A* and receives *TOKEN_A*.

#### Borrow



![Borrow Scenario (2)](https://user-images.githubusercontent.com/3630188/170364142-a9e5b0f4-80ae-4930-887e-174de375994e.png)



The flow is as follows:
1. The user calls `borrow` endpoint from the *Lending Pool SC*;
2. The *Lending Pool SC* gets the *Liquidity Pool Address* based on the *asset_to_borrow* parameters passed by the user;
3. The *Lending Pool SC* gets the *LTV* (Loan To Value), a value specific (different) for each token;
4. The *Lending Pool SC* calls the `borrow` endpoint from the *Liquidity Pool SC*;
5. The *Liquidity Pool SC* mints metaESDT tokens with an unique nonce. These have the same ticker as the borrwed token with an *B* appended in front (ABC -> BABC);
6. The *Liquidity Pool SC* computes the borrowable amount (*collateral value *LTV*) and creates a BorrowPosition with this particular borrow information.
7. The *Liquidity Pool SC* updates the reservers;
8. The *Liquidity Pool SC* performs 2 transactions: First transaction with the MetaEsdt tokens freshly minted, derived from the collateral (if the borrowed token is TOKEN_B, the minted token is BTOKEN_B). These tokens will be used to repay the debt.
9. The second transaction with the requested tokens (TOKEN_B).
10. The *Liquidity Pool SC* returns an EsdtTokenPayment with the MetaESDT freshly minted, its nonce and the amount which is the same as the borrowed amount (1 TOKEN = 1 BTOKEN). The *Lending Pool* returns the same result to the user.

TL;DR: User sends *LTOKEN_A* as collateral and receives *BTOKEN_B* (used for repay) and *TOKEN_B* (the token wanted to borrow).



#### Repay

Borrowers can repay tokens to the market up to the **borrow balance**.
If partial repayment is made, the **borrow balance** may be non-zero, and continue to accrue debt.
Repayment is a transfer of tokens from the borrower back to the token market.


![Repay Scenario (1)](https://user-images.githubusercontent.com/3630188/170364482-cd85653d-d8fa-4b00-a97d-bdd90877dc17.png)


The flow is as follows:
1. The user calls `borrow` endpoint from the *Lending Pool SC*. (This scenario requires *multi token transfer*);
2. The *Lending Pool SC* gets the *Liquidity Pool Address* based on the *asset_to_repay* parameters passed by the user;
3. The *Lending Pool SC* calls the `repay` endpoint from the *Liquidity Pool SC*;
4. The *Lending Pool SC* computes the accumulated debt. `total_owed = borrowed_token_amount + accumulated_debt`;
5. The *Lending Pool SC* sends back to user the extra tokens (`extra_tokens = asset_payed - total_owed`); 
6. The *Lending Pool SC* computes the amount of tokens to send to user;
7. The *Lending Pool SC* update reserves (borrowed amount and LP asset amount);
8. The *Lending Pool SC* burns the borrowed tokens (minted at the borrow scenario);
9. The *Lending Pool SC* sends directly to user the collateral used for borrow.

TL;DR: User sends *BTOKEN_B* and *TOKEN_B* (initial borrowed amount + interest) and receives *BTOKEN_A*.


#### Liquidate

If a borrower’s **borrowing balance exceeds** their total collateral value, it means that the protocol is at risk of suffering a loss if the borrower defaults on repayment.
We call this a shortfall event.
In order to reduce this risk, the protocol provides a public liquidate function which can be called by any user


## Interest Rate Model

We use an interest rate model which is based on the pool’s capital utilisation, which we can call *U*.
Liquidity risk materialises when utilisation is high, its becomes more problematic as *U* gets closer to 100%.
To tailor the model to this constraint, the interest rate curve is split in two parts around an optimal utilisation rate *Uoptimal*.
Before *Uoptimal* the slope is small, after it starts rising sharply. 

For *U >= Uoptimal*, the interest rates rise, because the capital is scarce.
For *U < Uoptimal*, the interest rates decrease, because the capital is plentiful.

The interest rate *Rt* formulas depending on the capital utilisation of the pool, where:

![image](https://user-images.githubusercontent.com/3630188/160089036-63f00d49-4a4c-4de0-8a5a-d4be220d9004.png)

*R0*, *Rslope1* and *Rslope2* are predefined values

### Simulations

![image](https://user-images.githubusercontent.com/3630188/160086654-8cfb9201-abb6-4b56-a57d-5bb72a0273e9.png)


In this case we have a pool where the asset is a Stablecoin, which is not volatile, so the borrow rate increases slightly with utilisation.
An optimal Utilisation Rate of 80% has been chosen, from which the rate increases sharply, as it provides enough safety in case of market downside.

![image](https://user-images.githubusercontent.com/3630188/160086719-ed30925f-f9f4-43d3-a07a-a5b7f2436477.png)


For a more volatile asset the borrow rate follows the same pattern as in the case of BUSD, but a much lower optimal utilisation rate has been chosen, with a steeper slope, as this assets are more likely to become undercollateralized and put the protocol solvency at risk.
Using a steeper slope once the Borrowed Amounts exceed the Optimal Rate ensures that loans are being repaid in time or, if not, liquidated before it becomes unhealthy for the pool’s reserves.

The simulations were run in a python environment using multiple timeframes and multiple borrows.
**In a real scenario, the curves are not going to be perfectly smooth as pictured above, as a pool’s reserves won’t follow a linear action.**


### Risk Parameters

**Loan to Value (LTV)** ratio defines the maximum amount of currency that can be borrowed with a specific collateral.
It’s expressed in percentage: at LTV=75%, for every 1 EGLD worth of collateral, borrowers will be able to borrow 0.75 EGLD worth of the corresponding (different) currency.
Once a borrow is taken, the LTV evolves with market conditions.

**Liquidation Threshold** is the percentage at which a position is defined as undercollateralised.
For example, a Liquidation threshold of 80% means that if the value rises above 80% of the collateral, the position is undercollateralised and could be liquidated.

The delta between the Loan-To-Value and the Liquidation Threshold is a safety cushion for borrowers.



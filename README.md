# jex-sc-pair

Liquidity pool smart contract with `x*y=k` AMM


## List of endpoints

* `addInitialLiquidity`
* `addLiquidity`
* `addLiquiditySingle`
* `configureLiqProvidersFees`
* `configurePlatformFees`
* `enableMintBurn`
* `estimateAddLiquiditySingle`
* `estimateAmountIn`
* `estimateAmountOut`
* `estimateRemoveLiquidity`
* `estimateRemoveLiquiditySingle`
* `getAnalyticsForLastEpochs`
* `getFirstToken`
* `getLiqProvidersFees`
* `getPlatformFees`
* `getPlatformFeesReceiver`
* `getSecondToken`
* `getStatus`
* `init`
* `isPaused`
* `issueLpToken`
* `pause`
* `removeLiquidity`
* `removeLiquiditySingle`
* `swapTokensFixedInput`
* `swapTokensFixedOutput`
* `unpause`


## How-to deploy a pool

**Deploy the smart contract**

Prepare snippets shell file in [interaction](./interaction/) folder.

And run:

```shell
xxx.snippets.sh deploy
```

Check results in the explorer.

**Issue LP token**

```shell
xxx.snippets.sh issueLpToken
```

Check results in the explorer.

**Enable LP token mint/burn**

The smart contract needs to be able to mint and burn LP tokens.

To do so, run:

```shell
xxx.snippets.sh enableMintBurn
```

Check results in the explorer.

**Configure fees**

Configure fees for liquidity providers and platform fees.

```shell
xxx.snippets.sh configureLiqProvidersFees
xxx.snippets.sh configurePlatformFees
```

Note: 100 = 1%, 20 = 0.2%

**Add initial liquidity**

Adding initial liquidity will determine the initial exchange rate of tokens.

For example, adding 10,000 JEX and 1 wEGLD:

* initial exchange rate will be 1 wEGLD = 10,000 JEX (1 JEX = 0.0001 wEGLD)


```shell
xxx.snippets.sh addInitialLiquidity
```

Check results in the explorer.

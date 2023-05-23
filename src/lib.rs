#![no_std]

use core::ops::Deref;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod analytics;
mod fees;
mod liquidity;
mod pausable;
mod swap;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct PairStatus<M: ManagedTypeApi> {
    paused: bool,
    first_token_identifier: TokenIdentifier<M>,
    first_token_reserve: BigUint<M>,
    second_token_identifier: TokenIdentifier<M>,
    second_token_reserve: BigUint<M>,
    lp_token_identifier: TokenIdentifier<M>,
    lp_token_supply: BigUint<M>,
    owner: ManagedAddress<M>,
    lp_fees: u32,
    platform_fees: u32,
    platform_fees_receiver: Option<ManagedAddress<M>>,
    volume_prev_epoch: [BigUint<M>; 2],
    fees_prev_epoch: [BigUint<M>; 2],
    fees_last_7_epochs: [BigUint<M>; 2],
}

#[multiversx_sc::contract]
pub trait JexScPairContract:
    analytics::AnalyticsModule
    + fees::FeesModule
    + liquidity::LiquidityModule
    + pausable::PausableModule
    + swap::SwapModule
{
    #[init]
    fn init(&self, first_token: TokenIdentifier, second_token: TokenIdentifier) {
        self.first_token().set_if_empty(&first_token);
        self.second_token().set_if_empty(&second_token);

        self.do_pause();
    }

    // owner endpoints

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(&self, lp_token_display_name: ManagedBuffer, lp_token_ticker: ManagedBuffer) {
        require!(self.lp_token().is_empty(), "LP token already issued");

        let egld_value = self.call_value().egld_value().deref().clone();
        let caller = self.blockchain().get_caller();

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                egld_value,
                &lp_token_display_name,
                &lp_token_ticker,
                &BigUint::from(1_000u32),
                FungibleTokenProperties {
                    num_decimals: 18,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().lp_token_issue_callback(&caller))
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(enableMintBurn)]
    fn enable_mint_burn(&self) {
        let lp_token = self.lp_token().get();
        require!(lp_token.is_valid_esdt_identifier(), "LP token not issued");

        let roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];

        let sc_address = self.blockchain().get_sc_address();

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&sc_address, &lp_token, roles.iter().cloned())
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(addInitialLiquidity)]
    fn add_initial_liquidity(&self) {
        require!(!self.lp_token().is_empty(), "LP token not issued");

        let [first_payment, second_payment] = self.call_value().multi_esdt();

        require!(
            first_payment.token_identifier == self.first_token().get() && first_payment.amount > 0,
            "Invalid payment for first token"
        );

        require!(
            second_payment.token_identifier == self.second_token().get()
                && second_payment.amount > 0,
            "Invalid payment for second token"
        );

        let (lp_amount, lp_token) =
            self.lp_add_initial_liquidity(&first_payment.amount, &second_payment.amount);

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &lp_token, 0, &lp_amount);
    }

    /// Configure liquidity providers swap fees
    /// 100 = 1%
    #[only_owner]
    #[endpoint(configureLiqProvidersFees)]
    fn configure_liq_providers_fees(&self, fees: u32) {
        self.liq_providers_fees().set(fees);
    }

    /// Configure platform swap fees
    /// 100 = 1%
    #[only_owner]
    #[endpoint(configurePlatformFees)]
    fn configure_platform_fees(&self, fees: u32, receiver: ManagedAddress) {
        self.platform_fees().set(fees);
        self.platform_fees_receiver().set(&receiver);
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self) {
        self.do_pause();
    }

    #[only_owner]
    #[endpoint]
    fn unpause(&self) {
        self.do_unpause();
    }

    // public endpoints

    /// Add liquidity is both tokens
    /// Note: liquidity can be added if SC is paused
    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(&self, min_second_token_amount: BigUint) {
        let [first_payment, second_payment] = self.call_value().multi_esdt();

        require!(
            first_payment.token_identifier == self.first_token().get() && first_payment.amount > 0,
            "Invalid payment for first token"
        );

        require!(
            second_payment.token_identifier == self.second_token().get()
                && second_payment.amount > 0,
            "Invalid payment for second token"
        );

        let (lp_amount, lp_token, overpaid_second_token_amount) = self.lp_add_liquidity(
            &first_payment.amount,
            &min_second_token_amount,
            &second_payment.amount,
        );

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &lp_token, 0, &lp_amount);

        if overpaid_second_token_amount > 0 {
            self.send().direct_esdt(
                &caller,
                &second_payment.token_identifier,
                second_payment.token_nonce,
                &overpaid_second_token_amount,
            );
        }
    }

    /// Add liquidity by providing only 1 of the 2 tokens
    /// Provided liquidity is added to the reserves and corresponding LP tokens are sent to caller.
    /// payment = token to deposit
    /// Note: unlike classic liquidity addition, SC must not be paused
    #[payable("*")]
    #[endpoint(addLiquiditySingle)]
    fn add_liquidity_single(
        &self,
        min_first_token_amount: BigUint,
        min_second_token_amount: BigUint,
    ) {
        self.require_not_paused();

        let (token_identifier, payment_amount) = self.call_value().single_fungible_esdt();

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_in = token_identifier == first_token;
        let is_second_token_in = token_identifier == second_token;

        require!(
            is_first_token_in || is_second_token_in,
            "Invalid payment token"
        );

        let (lp_amount, lp_token) = self.lp_add_liquidity_single_side(
            &payment_amount,
            &min_first_token_amount,
            &min_second_token_amount,
            is_first_token_in,
        );

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &lp_token, 0, &lp_amount);
    }

    /// Remove liquidity in both tokens
    /// Note: liquidity can be removed if SC is paused
    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(&self, min_first_token_amount: BigUint, min_second_token_amount: BigUint) {
        let (lp_token, lp_amount) = self.call_value().single_fungible_esdt();

        let (exact_first_token_amount, exact_second_token_amount) =
            self.lp_remove_liquidity(lp_token, lp_amount);

        require!(
            exact_first_token_amount >= min_first_token_amount,
            "Max slippage exceeded for first token"
        );
        require!(
            exact_second_token_amount >= min_second_token_amount,
            "Max slippage exceeded for second token"
        );

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(
            &caller,
            &self.first_token().get(),
            0,
            &exact_first_token_amount,
        );
        self.send().direct_esdt(
            &caller,
            &self.second_token().get(),
            0,
            &exact_second_token_amount,
        );
    }

    /// Remove liquidity and swap one half to desired token in 1 transaction
    /// Note: unlike classic liquidity removal, SC must not be paused
    #[payable("*")]
    #[endpoint(removeLiquiditySingle)]
    fn remove_liquidity_single(
        &self,
        token_out: TokenIdentifier,
        min_first_token_amount: BigUint,
        min_second_token_amount: BigUint,
    ) {
        self.require_not_paused();

        let (lp_token, lp_amount) = self.call_value().single_fungible_esdt();

        let (first_tokens_removed, second_tokens_removed) =
            self.lp_remove_liquidity(lp_token, lp_amount);

        let is_first_token_out = &token_out == &self.first_token().get();
        let is_first_token_in = !is_first_token_out;

        let (token_in, swap_amount_in) = if is_first_token_in {
            (self.first_token().get(), &first_tokens_removed)
        } else {
            (self.second_token().get(), &second_tokens_removed)
        };

        let swap_payment = self.swap_tokens_fixed_input_inner(
            &token_in,
            swap_amount_in,
            &token_out,
            is_first_token_in,
        );

        let caller = self.blockchain().get_caller();
        if is_first_token_out {
            let amount_out = &first_tokens_removed + &swap_payment.amount;
            require!(
                amount_out >= min_first_token_amount,
                "Max slippage exceeded for first token"
            );

            self.send()
                .direct_esdt(&caller, &self.first_token().get(), 0, &amount_out);
        } else {
            let amount_out = &second_tokens_removed + &swap_payment.amount;
            require!(
                amount_out >= min_second_token_amount,
                "Max slippage exceeded for second token"
            );
            self.send()
                .direct_esdt(&caller, &self.second_token().get(), 0, &amount_out);
        }
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(&self, min_amount_out: BigUint) {
        self.require_not_paused();

        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_in = token_in == first_token;
        let is_second_token_in = token_in == second_token;

        require!(
            is_first_token_in || is_second_token_in,
            "Invalid payment token"
        );

        let token_out = if is_first_token_in {
            second_token
        } else {
            first_token
        };

        let payment_out = self.swap_tokens_fixed_input_inner(
            &token_in,
            &amount_in,
            &token_out,
            is_first_token_in,
        );

        require!(
            payment_out.amount >= min_amount_out,
            "Max slippage exceeded"
        );

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(
            &caller,
            &payment_out.token_identifier,
            payment_out.token_nonce,
            &payment_out.amount,
        );
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(&self, exact_amount_out: BigUint) {
        self.require_not_paused();

        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_in = token_in == first_token;
        let is_second_token_in = token_in == second_token;

        require!(
            is_first_token_in || is_second_token_in,
            "Invalid payment token"
        );

        let token_out = if is_first_token_in {
            second_token
        } else {
            first_token
        };

        let exact_amount_in = self.swap_tokens_fixed_output_inner(
            &token_in,
            &token_out,
            &exact_amount_out,
            is_first_token_in,
        );

        require!(exact_amount_in <= amount_in, "Max slippage exceeded");

        let caller = self.blockchain().get_caller();
        self.send()
            .direct_esdt(&caller, &token_out, 0, &exact_amount_out);

        if amount_in > exact_amount_in {
            self.send()
                .direct_esdt(&caller, &token_in, 0, &(amount_in - exact_amount_in));
        }
    }

    // storage & views

    #[view(estimateAmountIn)]
    fn estimate_amount_in(
        &self,
        token_out: TokenIdentifier,
        amount_out: BigUint,
    ) -> swap::EstimateAmountIn<Self::Api> {
        self.require_not_paused();

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_out = token_out == first_token;
        let is_second_token_out = token_out == second_token;

        require!(
            is_first_token_out || is_second_token_out,
            "Invalid payment token"
        );

        let is_first_token_in = !is_first_token_out;

        let estimation = self.estimate_amount_in_inner(&amount_out, is_first_token_in);

        estimation
    }

    #[view(estimateAmountOut)]
    fn estimate_amount_out(
        &self,
        token_in: TokenIdentifier,
        amount_in: BigUint,
    ) -> swap::EstimateAmountOut<Self::Api> {
        self.require_not_paused();

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_in = token_in == first_token;
        let is_second_token_in = token_in == second_token;

        require!(
            is_first_token_in || is_second_token_in,
            "Invalid payment token"
        );

        let estimation = self.estimate_amount_out_inner(&amount_in, is_first_token_in);

        estimation
    }

    #[view(estimateAddLiquiditySingle)]
    fn estimate_add_liquidity_single(
        &self,
        token_in: TokenIdentifier,
        amount_in: BigUint,
    ) -> liquidity::EstimateAddLiquidityOut<Self::Api> {
        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_in = token_in == first_token;
        let is_second_token_in = token_in == second_token;

        require!(
            is_first_token_in || is_second_token_in,
            "Invalid payment token"
        );

        let estimation = self.lp_estimate_add_liquidity_single(&amount_in, is_first_token_in);

        estimation
    }

    #[view(estimateRemoveLiquidity)]
    fn estimate_remove_liquidity(
        &self,
        lp_amount: BigUint,
    ) -> liquidity::EstimateRemoveLiquidityOut<Self::Api> {
        let estimation = self.lp_estimate_remove_liquidity(&lp_amount);

        estimation
    }

    /// Estimate liquidity removal to one token
    /// (liquidity removal + swap of one half to desired token)
    #[view(estimateRemoveLiquiditySingle)]
    fn estimate_remove_liquidity_single(
        &self,
        lp_amount: BigUint,
        token_out: TokenIdentifier,
    ) -> liquidity::EstimateRemoveLiquidityOut<Self::Api> {
        require!(
            &lp_amount * 2u32 <= self.lp_token_supply().get(),
            "Cannot remove that much liquidity"
        );

        let est_remove_lp = self.lp_estimate_remove_liquidity(&lp_amount);

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let is_first_token_out = token_out == first_token;
        let is_second_token_out = token_out == second_token;

        require!(
            is_first_token_out || is_second_token_out,
            "Invalid out token"
        );

        self.first_token_reserve()
            .update(|x| *x -= &est_remove_lp.eq_first_tokens);
        self.second_token_reserve()
            .update(|x| *x -= &est_remove_lp.eq_second_tokens);

        let half_swap_estimate = if is_first_token_out {
            self.estimate_amount_out_inner(&est_remove_lp.eq_second_tokens, false)
        } else {
            self.estimate_amount_out_inner(&est_remove_lp.eq_first_tokens, true)
        };

        let estimation = liquidity::EstimateRemoveLiquidityOut {
            eq_first_tokens: if is_first_token_out {
                &est_remove_lp.eq_first_tokens + &half_swap_estimate.net_amount_out
            } else {
                BigUint::zero()
            },
            eq_second_tokens: if is_second_token_out {
                &est_remove_lp.eq_second_tokens + &half_swap_estimate.net_amount_out
            } else {
                BigUint::zero()
            },
        };

        estimation
    }

    #[view(getFirstToken)]
    #[storage_mapper("first_token")]
    fn first_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getSecondToken)]
    #[storage_mapper("second_token")]
    fn second_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getStatus)]
    fn get_status(&self) -> PairStatus<Self::Api> {
        let prev_epoch = self.blockchain().get_block_epoch() - 1u64;

        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let opt_platform_fees_receiver = if self.platform_fees_receiver().is_empty() {
            Option::None
        } else {
            Option::Some(self.platform_fees_receiver().get())
        };

        let mut sum_lp_fees_first = BigUint::zero();
        let mut sum_lp_fees_second = BigUint::zero();
        for i in 0u64..=6u64 {
            sum_lp_fees_first += self.lp_fees(prev_epoch - i, &first_token).get();
            sum_lp_fees_second += self.lp_fees(prev_epoch - i, &second_token).get();
        }

        let status = PairStatus {
            paused: self.is_paused().get(),
            first_token_identifier: first_token.clone(),
            first_token_reserve: self.first_token_reserve().get(),
            second_token_identifier: second_token.clone(),
            second_token_reserve: self.second_token_reserve().get(),
            lp_token_identifier: self.lp_token().get(),
            lp_token_supply: self.lp_token_supply().get(),
            owner: self.blockchain().get_owner_address(),
            lp_fees: self.liq_providers_fees().get(),
            platform_fees: self.platform_fees().get(),
            platform_fees_receiver: opt_platform_fees_receiver,
            volume_prev_epoch: [
                self.trading_volume(prev_epoch, &first_token).get(),
                self.trading_volume(prev_epoch, &second_token).get(),
            ],
            fees_prev_epoch: [
                self.lp_fees(prev_epoch, &first_token).get(),
                self.lp_fees(prev_epoch, &second_token).get(),
            ],
            fees_last_7_epochs: [sum_lp_fees_first, sum_lp_fees_second],
        };

        status
    }

    #[view(getAnalyticsForLastEpochs)]
    fn get_analytics_for_last_epochs(
        &self,
        countback: u64,
    ) -> MultiValueEncoded<Self::Api, analytics::AnalyticsForEpoch<Self::Api>> {
        let first_token = self.first_token().get();
        let second_token = self.second_token().get();

        let res = self.get_analytics_for_last_epochs_inner(countback, &first_token, &second_token);

        res.into()
    }

    // callbacks

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<()>,
    ) {
        let (token_id, returned_tokens) = self.call_value().egld_or_single_fungible_esdt();
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                let esdt = token_id.unwrap_esdt();
                self.lp_token().set(&esdt);
                self.send().direct_esdt(caller, &esdt, 0, &returned_tokens);
            }
            ManagedAsyncCallResult::Err(_) => {
                if token_id.is_egld() && returned_tokens > 0u64 {
                    self.send().direct_egld(caller, &returned_tokens);
                }
            }
        }
    }
}

#![no_std]

multiversx_sc::imports!();

mod fees;
mod liquidity;
mod swap;
mod wrap_sc_proxy;

#[multiversx_sc::contract]
pub trait JexScPairContract:
    fees::FeesModule + liquidity::LiquidityModule + swap::SwapModule
{
    #[init]
    fn init(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
        lp_token: TokenIdentifier,
    ) {
        self.first_token().set_if_empty(&first_token);
        self.second_token().set_if_empty(&second_token);
        self.lp_token().set_if_empty(&lp_token);
    }

    // owner endpoints

    #[only_owner]
    #[payable("*")]
    #[endpoint(addInitialLiquidity)]
    fn add_initial_liquidity(&self) {
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

    // public endpoints

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
    #[payable("*")]
    #[endpoint(addLiquiditySingle)]
    fn add_liquidity_single(&self, min_other_token_amount: BigUint) {
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
            &min_other_token_amount,
            is_first_token_in,
        );

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &lp_token, 0, &lp_amount);
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(&self, min_first_token_amount: BigUint, min_second_token_amount: BigUint) {
        let (lp_token_identifier, lp_amount) = self.call_value().single_fungible_esdt();

        let (exact_first_token_amount, exact_second_token_amount) =
            self.lp_remove_liquidity(lp_token_identifier, lp_amount);

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

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(&self, min_amount_out: BigUint) {
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

        let payment_out =
            self.swap_tokens_fixed_input_inner(&amount_in, &token_out, is_first_token_in);

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

        let exact_amount_in =
            self.swap_tokens_fixed_output_inner(&exact_amount_out, &token_out, is_first_token_in);

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

    #[view(getFirstToken)]
    #[storage_mapper("first_token")]
    fn first_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getSecondToken)]
    #[storage_mapper("second_token")]
    fn second_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getWrapScAddress)]
    #[storage_mapper("wrap_sc_address")]
    fn wrap_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    // proxies

    #[proxy]
    fn wrap_sc_proxy(&self, sc_address: ManagedAddress) -> wrap_sc_proxy::Proxy<Self::Api>;
}

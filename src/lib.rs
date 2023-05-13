#![no_std]

multiversx_sc::imports!();

mod liquidity;
mod wrap_sc_proxy;

#[multiversx_sc::contract]
pub trait JexScPairContract: liquidity::LiquidityModule {
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

    // storage & views

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

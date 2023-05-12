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

        self.first_token_reserve().set(&first_payment.amount);
        self.second_token_reserve().set(&second_payment.amount);

        let (lp_amount, lp_token) =
            self.lp_add_initial_liquidity(&first_payment.amount, &second_payment.amount);

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &lp_token, 0, &lp_amount);
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

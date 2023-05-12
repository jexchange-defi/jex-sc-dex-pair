multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait LiquidityModule {
    // functions

    fn lp_add_initial_liquidity(
        &self,
        first_token_amount: &BigUint,
        second_token_amount: &BigUint,
    ) -> (BigUint, TokenIdentifier) {
        require!(self.lp_token_supply().get() == 0, "Liquidity already added");

        self.first_token_reserve().set(first_token_amount);
        self.second_token_reserve().set(second_token_amount);

        let lp_amount = first_token_amount.min(second_token_amount).clone();

        let lp_token = self.lp_mint(&lp_amount);

        (lp_amount, lp_token)
    }

    fn lp_mint(&self, amount: &BigUint) -> TokenIdentifier {
        self.lp_token_supply().set(amount);

        let lp_token = self.lp_token().get();
        self.send().esdt_local_mint(&lp_token, 0, amount);

        lp_token
    }

    // storage & views

    #[view(getFirstTokenReserve)]
    #[storage_mapper("first_token_reserve")]
    fn first_token_reserve(&self) -> SingleValueMapper<BigUint>;

    #[view(getSecondTokenReserve)]
    #[storage_mapper("second_token_reserve")]
    fn second_token_reserve(&self) -> SingleValueMapper<BigUint>;

    #[view(getLpToken)]
    #[storage_mapper("lp_token")]
    fn lp_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLpTokenSupply)]
    #[storage_mapper("lp_token_supply")]
    fn lp_token_supply(&self) -> SingleValueMapper<BigUint>;
}

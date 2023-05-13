multiversx_sc::imports!();

#[multiversx_sc::module]
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

    fn lp_add_liquidity(
        &self,
        exact_first_token_amount: &BigUint,
        min_second_token_amount: &BigUint,
        max_second_token_amount: &BigUint,
    ) -> (BigUint, TokenIdentifier, BigUint) {
        let first_token_reserve = self.first_token_reserve().get();

        let exact_second_token_amount =
            exact_first_token_amount * &self.second_token_reserve().get() / &first_token_reserve;

        require!(
            &exact_second_token_amount <= max_second_token_amount,
            "Not enough second tokens"
        );
        require!(
            &exact_second_token_amount >= min_second_token_amount,
            "Max slippage exceeded"
        );

        self.first_token_reserve()
            .update(|x| *x += exact_first_token_amount);
        self.second_token_reserve()
            .update(|x| *x += &exact_second_token_amount);

        let lp_amount =
            exact_first_token_amount * &self.lp_token_supply().get() / &first_token_reserve;

        let lp_token = self.lp_mint(&lp_amount);

        let overpaid_second_token_amount = max_second_token_amount - &exact_second_token_amount;

        (lp_amount, lp_token, overpaid_second_token_amount)
    }

    fn lp_burn(&self, amount: &BigUint) {
        self.lp_token_supply().update(|x| *x -= amount);

        let lp_token = self.lp_token().get();
        self.send().esdt_local_burn(&lp_token, 0, amount);
    }

    fn lp_mint(&self, amount: &BigUint) -> TokenIdentifier {
        self.lp_token_supply().update(|x| *x += amount);

        let lp_token = self.lp_token().get();
        self.send().esdt_local_mint(&lp_token, 0, amount);

        lp_token
    }

    fn lp_remove_liquidity(
        &self,
        lp_token_identifier: TokenIdentifier,
        lp_amount: BigUint,
    ) -> (BigUint, BigUint) {
        require!(
            self.lp_token().get() == lp_token_identifier,
            "Invalid LP token"
        );

        let first_token_reserve = self.first_token_reserve().get();
        let second_token_reserve = self.second_token_reserve().get();
        let lp_token_supply = self.lp_token_supply().get();

        let exact_first_token_amount = &lp_amount * &first_token_reserve / &lp_token_supply;
        let exact_second_token_amount = &lp_amount * &second_token_reserve / &lp_token_supply;

        self.first_token_reserve()
            .update(|x| *x -= &exact_first_token_amount);
        self.second_token_reserve()
            .update(|x| *x -= &exact_second_token_amount);

        self.lp_burn(&lp_amount);

        (exact_first_token_amount, exact_second_token_amount)
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

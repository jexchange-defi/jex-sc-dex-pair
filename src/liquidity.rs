multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const MIN_LIQUIDITY: u32 = 10000u32;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct EstimateAddLiquidityOut<M: ManagedTypeApi> {
    lp_amount: BigUint<M>,
    lp_supply: BigUint<M>,
    eq_first_tokens: BigUint<M>,
    eq_second_tokens: BigUint<M>,
}

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

        self.require_enough_liquidity();

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

    fn lp_add_liquidity_single_side(
        &self,
        amount_in: &BigUint,
        min_other_token_amount: &BigUint,
        is_first_token_in: bool,
    ) -> (BigUint, TokenIdentifier) {
        let estimation = self.lp_estimate_add_liquidity_single(amount_in, is_first_token_in);

        let other_token_amount = if is_first_token_in {
            estimation.eq_second_tokens
        } else {
            estimation.eq_first_tokens
        };

        require!(
            &other_token_amount >= min_other_token_amount,
            "Max slippage exceeded"
        );

        if is_first_token_in {
            self.first_token_reserve().update(|x| *x += amount_in);
        } else {
            self.second_token_reserve().update(|x| *x += amount_in);
        }

        let lp_token = self.lp_mint(&estimation.lp_amount);

        (estimation.lp_amount, lp_token)
    }

    fn lp_burn(&self, amount: &BigUint) {
        self.lp_token_supply().update(|x| *x -= amount);

        let lp_token = self.lp_token().get();
        self.send().esdt_local_burn(&lp_token, 0, amount);
    }

    fn lp_estimate_add_liquidity_single(
        &self,
        amount_in: &BigUint,
        is_first_token_in: bool,
    ) -> EstimateAddLiquidityOut<Self::Api> {
        let (in_reserve, other_reserve) = if is_first_token_in {
            (
                self.first_token_reserve().get(),
                self.second_token_reserve().get(),
            )
        } else {
            (
                self.second_token_reserve().get(),
                self.first_token_reserve().get(),
            )
        };

        let in_reserve_after = &in_reserve + amount_in;

        let lp_supply_before = self.lp_token_supply().get();

        let lp_amount = (amount_in * &lp_supply_before) / (&in_reserve * 2u32);

        let lp_supply_after = &lp_supply_before + &lp_amount;

        let eq_amount_in = (&lp_amount * &in_reserve_after) / &lp_supply_after;
        let eq_amount_other = (&lp_amount * &other_reserve) / &lp_supply_after;

        let estimation = EstimateAddLiquidityOut {
            lp_amount,
            lp_supply: lp_supply_after,
            eq_first_tokens: if is_first_token_in {
                eq_amount_in.clone()
            } else {
                eq_amount_other.clone()
            },
            eq_second_tokens: if is_first_token_in {
                eq_amount_other.clone()
            } else {
                eq_amount_in.clone()
            },
        };

        estimation
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

        // prevent removing all liquidity
        self.require_enough_liquidity();

        self.lp_burn(&lp_amount);

        (exact_first_token_amount, exact_second_token_amount)
    }

    fn lp_update_reserves(
        &self,
        amount_in: &BigUint,
        amount_out: &BigUint,
        is_first_token_in: bool,
    ) {
        let (in_reserve_mapper, out_reserve_mapper) = if is_first_token_in {
            (self.first_token_reserve(), self.second_token_reserve())
        } else {
            (self.second_token_reserve(), self.first_token_reserve())
        };

        in_reserve_mapper.update(|x| *x += amount_in);
        out_reserve_mapper.update(|x| *x -= amount_out);

        // prevent draining all of one reserve
        self.require_enough_liquidity();
    }

    fn require_enough_liquidity(&self) {
        require!(
            self.first_token_reserve().get() >= MIN_LIQUIDITY,
            "Not enough liquidity for first token"
        );
        require!(
            self.second_token_reserve().get() >= MIN_LIQUIDITY,
            "Not enough liquidity for second token"
        );
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

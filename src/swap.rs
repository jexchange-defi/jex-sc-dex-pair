multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SwapModule: crate::fees::FeesModule + crate::liquidity::LiquidityModule {
    fn swap_tokens_fixed_input_inner(
        &self,
        amount_in: &BigUint,
        token_out: &TokenIdentifier,
        is_first_token_in: bool,
    ) -> EsdtTokenPayment {
        let in_reserve_mapper = if is_first_token_in {
            self.first_token_reserve()
        } else {
            self.second_token_reserve()
        };

        let out_reserve_mapper = if is_first_token_in {
            self.second_token_reserve()
        } else {
            self.first_token_reserve()
        };

        let in_reserve_before = in_reserve_mapper.get();
        let out_reserve_before = out_reserve_mapper.get();

        let mut amount_out = amount_in * &out_reserve_before / &in_reserve_before;

        require!(amount_out < out_reserve_before, "Not enough liquidity");

        let liq_providers_fee = self.calculate_liq_providers_fee(&amount_out);
        let platform_fee = self.calculate_and_send_platform_fee(&token_out, &amount_out);

        amount_out -= liq_providers_fee;

        in_reserve_mapper.update(|x| *x += amount_in);
        out_reserve_mapper.update(|x| *x -= &amount_out);

        amount_out -= platform_fee;

        EsdtTokenPayment::new(token_out.clone(), 0, amount_out)
    }
}

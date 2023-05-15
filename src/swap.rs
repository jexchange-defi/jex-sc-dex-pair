multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct EstimeAmountOut<M: ManagedTypeApi> {
    net_amount_out: BigUint<M>,
    amount_out: BigUint<M>,
    liq_providers_fee: BigUint<M>,
    platform_fee: BigUint<M>,
}

#[multiversx_sc::module]
pub trait SwapModule: crate::fees::FeesModule + crate::liquidity::LiquidityModule {
    // functions

    fn swap_tokens_fixed_input_inner(
        &self,
        amount_in: &BigUint,
        token_out: &TokenIdentifier,
        is_first_token_in: bool,
    ) -> EsdtTokenPayment {
        let (in_reserve_mapper, out_reserve_mapper) = if is_first_token_in {
            (self.first_token_reserve(), self.second_token_reserve())
        } else {
            (self.second_token_reserve(), self.first_token_reserve())
        };

        let estimation = self.estimate_amount_out_inner(amount_in, is_first_token_in);

        let diff_out_reserve = estimation.amount_out - estimation.liq_providers_fee;

        in_reserve_mapper.update(|x| *x += amount_in);
        out_reserve_mapper.update(|x| *x -= &diff_out_reserve);

        self.send_platform_fee(token_out, &estimation.platform_fee);

        EsdtTokenPayment::new(token_out.clone(), 0, estimation.net_amount_out)
    }

    fn estimate_amount_out_inner(
        &self,
        amount_in: &BigUint,
        is_first_token_in: bool,
    ) -> EstimeAmountOut<Self::Api> {
        let (in_reserve_before, out_reserve_before) = if is_first_token_in {
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

        let amount_out = amount_in * &out_reserve_before / &in_reserve_before;

        require!(amount_out < out_reserve_before, "Not enough liquidity");

        let liq_providers_fee = self.calculate_liq_providers_fee(&amount_out);
        let platform_fee = self.calculate_platform_fee(&amount_out);

        let estimation = EstimeAmountOut {
            amount_out: amount_out.clone(),
            net_amount_out: &amount_out - &liq_providers_fee - &platform_fee,
            liq_providers_fee,
            platform_fee,
        };

        estimation
    }
}

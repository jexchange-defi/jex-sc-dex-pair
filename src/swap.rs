multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct EstimateAmountIn<M: ManagedTypeApi> {
    amount_in: BigUint<M>,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct EstimateAmountOut<M: ManagedTypeApi> {
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
        let estimation = self.estimate_amount_out_inner(amount_in, is_first_token_in);

        let diff_out_reserve = estimation.amount_out - estimation.liq_providers_fee;

        self.lp_update_reserves(amount_in, &diff_out_reserve, is_first_token_in);

        self.send_platform_fee(token_out, &estimation.platform_fee);

        EsdtTokenPayment::new(token_out.clone(), 0, estimation.net_amount_out)
    }

    fn swap_tokens_fixed_output_inner(
        &self,
        exact_amount_out: &BigUint,
        token_out: &TokenIdentifier,
        is_first_token_in: bool,
    ) -> BigUint {
        let estimation = self.estimate_amount_in_inner(exact_amount_out, is_first_token_in);

        let exact_amount_in = estimation.amount_in;
        self.swap_tokens_fixed_input_inner(&exact_amount_in, token_out, is_first_token_in);

        exact_amount_in
    }

    fn ceil_div(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if b == &0 {
            return BigUint::zero();
        }

        let res = if &a.div(b).mul(b) == a {
            a.div(b)
        } else {
            a.div(b) + 1u32
        };

        res
    }

    fn estimate_amount_in_inner(
        &self,
        net_amount_out: &BigUint,
        is_first_token_in: bool,
    ) -> EstimateAmountIn<Self::Api> {
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

        let amount_out = self.unapply_fees(&net_amount_out);

        let amount_in = self.ceil_div(&(&amount_out * &in_reserve_before), &out_reserve_before);

        require!(amount_in < in_reserve_before, "Not enough liquidity");

        let estimation = EstimateAmountIn { amount_in };

        estimation
    }

    fn estimate_amount_out_inner(
        &self,
        amount_in: &BigUint,
        is_first_token_in: bool,
    ) -> EstimateAmountOut<Self::Api> {
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

        let estimation = EstimateAmountOut {
            amount_out: amount_out.clone(),
            net_amount_out: &amount_out - &liq_providers_fee - &platform_fee,
            liq_providers_fee,
            platform_fee,
        };

        estimation
    }
}

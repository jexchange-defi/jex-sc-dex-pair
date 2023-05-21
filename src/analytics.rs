multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(ManagedVecItem, TopEncode, TopDecode, TypeAbi)]
pub struct AnalyticsForEpoch<M: ManagedTypeApi> {
    epoch: u64,
    volume_first_token: BigUint<M>,
    volume_second_token: BigUint<M>,
    lp_fees_first_token: BigUint<M>,
    lp_fees_second_token: BigUint<M>,
}

#[multiversx_sc::module]
pub trait AnalyticsModule {
    fn analytics_add_lp_fees(&self, token: &TokenIdentifier, vol: &BigUint) {
        let epoch = self.blockchain().get_block_epoch();

        self.lp_fees(epoch, token).update(|x| *x += vol);
    }

    fn analytics_add_volume(&self, token: &TokenIdentifier, vol: &BigUint) {
        let epoch = self.blockchain().get_block_epoch();

        self.trading_volume(epoch, token).update(|x| *x += vol);
    }

    #[view(getLpFees)]
    #[storage_mapper("an_lp_fees")]
    fn lp_fees(&self, epoch: u64, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getTradingVolume)]
    #[storage_mapper("an_t_vol")]
    fn trading_volume(&self, epoch: u64, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    fn get_analytics_for_last_epochs_inner(
        &self,
        countback: u64,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) -> ManagedVec<Self::Api, AnalyticsForEpoch<Self::Api>> {
        let mut res = ManagedVec::<Self::Api, AnalyticsForEpoch<Self::Api>>::new();

        let current_epoch = self.blockchain().get_block_epoch();
        for epoch in (current_epoch - countback)..current_epoch {
            let item = AnalyticsForEpoch::<Self::Api> {
                epoch,
                volume_first_token: self.trading_volume(epoch, first_token).get(),
                volume_second_token: self.trading_volume(epoch, second_token).get(),
                lp_fees_first_token: self.lp_fees(epoch, first_token).get(),
                lp_fees_second_token: self.lp_fees(epoch, second_token).get(),
            };
            res.push(item);
        }

        res
    }
}

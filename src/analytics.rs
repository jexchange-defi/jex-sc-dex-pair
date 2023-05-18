multiversx_sc::imports!();

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
}

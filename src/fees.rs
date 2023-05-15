multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FeesModule {
    // functions

    fn calculate_liq_providers_fee(&self, amount: &BigUint) -> BigUint {
        return amount * self.liq_providers_fees().get() / 10000u32;
    }

    fn calculate_platform_fee(&self, amount: &BigUint) -> BigUint {
        let fee: BigUint = amount * self.platform_fees().get() / 10000u32;

        fee
    }

    fn send_platform_fee(&self, token: &TokenIdentifier, fee: &BigUint) {
        if fee > &0 {
            self.send()
                .direct_esdt(&self.platform_fees_receiver().get(), token, 0, fee);
        }
    }

    // storage & views

    #[view(getLiqProvidersFees)]
    #[storage_mapper("liq_providers_fees")]
    fn liq_providers_fees(&self) -> SingleValueMapper<u32>;

    #[view(getPlatformFees)]
    #[storage_mapper("platform_fees")]
    fn platform_fees(&self) -> SingleValueMapper<u32>;

    #[view(getPlatformFeesReceiver)]
    #[storage_mapper("platform_fees_receiver")]
    fn platform_fees_receiver(&self) -> SingleValueMapper<ManagedAddress>;
}

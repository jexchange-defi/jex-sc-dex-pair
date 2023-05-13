multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait FeesModule {
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

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::proxy]
pub trait WrapScProxy {
    #[payable("*")]
    #[endpoint(unwrapEgld)]
    fn unwrap_egld(&self);
}

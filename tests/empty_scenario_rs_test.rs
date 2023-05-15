use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    blockchain.register_contract("file:output/jex-sc-pair.wasm", jex_sc_pair::ContractBuilder);
    blockchain
}

#[test]
fn empty_rs() {
    multiversx_sc_scenario::run_rs(
        "scenarios/remove_liquidity_not_enough_liquidity.scen.json",
        world(),
    );
}

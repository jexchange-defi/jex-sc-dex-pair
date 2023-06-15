#!/bin/bash

BYTECODE=../output-docker/jex-sc-dex-pair/jex-sc-dex-pair.wasm
PROXY=https://gateway.multiversx.com
SC_ADDRESS=$(mxpy data load --key=address-mainnet-jex-wegld)
CHAIN=1
SCRIPT_DIR=$(dirname $0)
FIRST_TOKEN_ID=JEX-9040ca
SECOND_TOKEN_ID=WEGLD-bd4d79

source "${SCRIPT_DIR}/_common.snippets.sh"

# Reproducible build using:
# mxpy contract reproducible-build --docker-image="multiversx/sdk-rust-contract-builder:v5.0.0"
deploy() {
    echo 'You are about to deploy SC on mainnet (Ctrl-C to abort)'
    read answer

    mxpy contract deploy --bytecode=${BYTECODE} \
         --keyfile=${1} --gas-limit=80000000 --outfile="deploy-mainnet.interaction.json" \
         --arguments "str:${FIRST_TOKEN_ID}" "str:${SECOND_TOKEN_ID}" \
         --proxy=${PROXY} --chain=${CHAIN} --recall-nonce --send || return

    SC_ADDRESS=$(mxpy data parse --file="deploy-mainnet.interaction.json" --expression="data['contractAddress']")

    mxpy data store --key=address-mainnet-jex-wegld --value=${SC_ADDRESS}

    echo ""
    echo "Smart contract address: ${SC_ADDRESS}"
}

upgrade() {
    echo 'You are about to upgrade current SC on mainnet (Ctrl-C to abort)'
    read answer

    mxpy contract upgrade --bytecode ${PROJECT}/output-docker/jex-sc-dex-pair/jex-sc-dex-pair.wasm \
        --keyfile=${KEYFILE} --gas-limit=80000000 --outfile="deploy-mainnet.interaction.json" \
        --arguments "0x" "0x" \
        --proxy=${PROXY} --chain=${CHAIN} --recall-nonce --send ${SC_ADDRESS} || return

    echo ""
    echo "Smart contract upgraded: ${SC_ADDRESS}"
}

CMD=$1
shift

$CMD $*

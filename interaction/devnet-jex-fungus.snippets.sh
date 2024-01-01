#!/bin/bash

BYTECODE=../output/jex-sc-dex-pair.wasm
PROXY=https://devnet-gateway.multiversx.com
SC_ADDRESS=$(mxpy data load --key=address-devnet-jex-fungus)
CHAIN=D
SCRIPT_DIR=$(dirname $0)
FIRST_TOKEN_ID=JEX-89ce64
SECOND_TOKEN_ID=FUNGUS-73c9cd

source "${SCRIPT_DIR}/_common.snippets.sh"

deploy() {
    echo 'You are about to deploy SC on devnet (Ctrl-C to abort)'
    read answer

    mxpy contract deploy --bytecode=${BYTECODE} \
        --keyfile=${1} --gas-limit=80000000 --outfile="deploy-devnet.interaction.json" \
        --arguments "str:${FIRST_TOKEN_ID}" "str:${SECOND_TOKEN_ID}" \
        --proxy=${PROXY} --chain=${CHAIN} --recall-nonce --send || return

    SC_ADDRESS=$(mxpy data parse --file="deploy-devnet.interaction.json" --expression="data['contractAddress']")

    mxpy data store --key=address-devnet-jex-fungus --value=${SC_ADDRESS}

    echo ""
    echo "Smart contract address: ${SC_ADDRESS}"
}

upgrade() {
    echo 'You are about to upgrade current SC on devnet (Ctrl-C to abort)'
    read answer

    mxpy contract upgrade --project=${PROJECT} \
        --keyfile=${1} --gas-limit=80000000 --outfile="deploy-devnet.interaction.json" \
        --arguments "0x" "0x" \
        --proxy=${PROXY} --chain=${CHAIN} --recall-nonce --send ${SC_ADDRESS} || return

    echo ""
    echo "Smart contract upgraded: ${SC_ADDRESS}"
}

CMD=$1
shift

$CMD $*

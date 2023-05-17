##
# Info
##

echo "Proxy: ${PROXY}"
echo "SC address: ${SC_ADDRESS:-Not deployed}"

##
# Owner endpoints
##

addInitialLiquidity() {
    read -p "Amount (${FIRST_TOKEN_ID}): " AMOUNT_FIRST
    read -p "Amount (${SECOND_TOKEN_ID}): " SECOND_FIRST

    USER_ADDRESS=$(mxpy wallet pem-address ${1})

    mxpy contract call ${USER_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="MultiESDTNFTTransfer" \
        --arguments "${SC_ADDRESS}" "2" \
            "str:${FIRST_TOKEN_ID}" "0" "${AMOUNT_FIRST}" \
            "str:${SECOND_TOKEN_ID}" "0" "${SECOND_FIRST}" \
            "str:addInitialLiquidity" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

enableMintBurn() {
    mxpy contract call ${SC_ADDRESS} --recall-nonce --pem=${1} --gas-limit=75000000 \
        --function="enableMintBurn" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

issueLpToken() {
    read -p 'Display name: ' DISPLAY_NAME
    read -p 'Ticker: ' TICKER

    mxpy contract call ${SC_ADDRESS} --recall-nonce --pem=${1} --gas-limit=75000000 \
        --function="issueLpToken" \
        --arguments "str:${DISPLAY_NAME}" "str:${TICKER}" \
        --value 50000000000000000 \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

##
# Views
##

getStatus() {
    mxpy contract query ${SC_ADDRESS} --function "getStatus" --proxy=${PROXY} | jq '.[].hex'
}

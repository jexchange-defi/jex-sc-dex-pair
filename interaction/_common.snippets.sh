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
    read -p "Amount (${SECOND_TOKEN_ID}): " AMOUNT_SECOND

    USER_ADDRESS=$(mxpy wallet pem-address ${1})

    mxpy contract call ${USER_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="MultiESDTNFTTransfer" \
        --arguments "${SC_ADDRESS}" "2" \
            "str:${FIRST_TOKEN_ID}" "0" "${AMOUNT_FIRST}" \
            "str:${SECOND_TOKEN_ID}" "0" "${AMOUNT_SECOND}" \
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
# Public endpoints
##

addLiquidity() {
    read -p "Amount (${FIRST_TOKEN_ID}): " AMOUNT_FIRST
    read -p "Amount (${SECOND_TOKEN_ID}): " AMOUNT_SECOND
    read -p "Min Amount (${SECOND_TOKEN_ID}): " MIN_AMOUNT_SECOND

    USER_ADDRESS=$(mxpy wallet pem-address ${1})

    mxpy contract call ${USER_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="MultiESDTNFTTransfer" \
        --arguments "${SC_ADDRESS}" "2" \
            "str:${FIRST_TOKEN_ID}" "0" "${AMOUNT_FIRST}" \
            "str:${SECOND_TOKEN_ID}" "0" "${AMOUNT_SECOND}" \
            "str:addLiquidity" \
            "${MIN_AMOUNT_SECOND}" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

addLiquiditySingle() {
    read -p 'Token: ' TOKEN_IN
    read -p 'Amount: ' AMOUNT_IN
    
    mxpy contract call ${SC_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="ESDTTransfer" \
        --arguments "str:${TOKEN_IN}" "${AMOUNT_IN}" \
                    "str:addLiquiditySingle" "1" "1" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

removeLiquidity() {
    read -p 'LP Token: ' TOKEN_IN
    read -p 'Amount: ' AMOUNT_IN

    mxpy contract call ${SC_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="ESDTTransfer" \
        --arguments "str:${TOKEN_IN}" "${AMOUNT_IN}" \
                    "str:removeLiquidity" "1" "1" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

removeLiquiditySingle() {
    read -p 'LP Token: ' TOKEN_IN
    read -p 'Amount: ' AMOUNT_IN
    read -p 'Token out: ' TOKEN_OUT

    mxpy contract call ${SC_ADDRESS} --recall-nonce --pem=${1} --gas-limit=10000000 \
        --function="ESDTTransfer" \
        --arguments "str:${TOKEN_IN}" "${AMOUNT_IN}" \
                    "str:removeLiquiditySingle" "str:${TOKEN_OUT}" "1" "1" \
        --proxy=${PROXY} --chain=${CHAIN} --send || return
}

##
# Views
##

estimateAmountOut() {
    TOKEN_IN=$1
    AMOUNT_IN=$2

    mxpy contract query ${SC_ADDRESS} \
        --function "estimateAmountOut" \
        --arguments "str:${TOKEN_IN}" "${AMOUNT_IN}"  \
        --proxy=${PROXY} | jq '.[].hex'
}

estimateRemoveLiquidity() {
    read -p 'Amount: ' LP_AMOUNT

    mxpy contract query ${SC_ADDRESS} \
        --function "estimateRemoveLiquidity" \
        --arguments "${LP_AMOUNT}"  \
        --proxy=${PROXY} | jq '.[].hex'
}

estimateRemoveLiquiditySingle () {
    read -p 'LP amount: ' LP_AMOUNT
    read -p 'Token out: ' TOKEN_OUT

    mxpy contract query ${SC_ADDRESS} \
        --function "estimateRemoveLiquiditySingle" \
        --arguments "${LP_AMOUNT}" "str:${TOKEN_OUT}"  \
        --proxy=${PROXY} | jq '.[].hex'
}

getStatus() {
    mxpy contract query ${SC_ADDRESS} --function "getStatus" --proxy=${PROXY} | jq '.[].hex'
}

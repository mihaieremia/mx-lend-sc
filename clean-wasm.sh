#!/bin/sh

# cleans all wasm targets

set -e
SMART_CONTRACT_JSONS=$(find . -name "multiversx.json")
for smart_contract_json in $SMART_CONTRACT_JSONS
do
    smart_contract_folder=$(dirname $smart_contract_json)
    echo ""
    (set -x; mxpy --verbose contract clean $smart_contract_folder)
done


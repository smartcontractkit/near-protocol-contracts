#!/bin/bash
near call v0.oracle.testnet request '{"payment": "1", "spec_id": [97, 108, 111, 104, 97, 32, 104, 111, 110, 117, 97], "callback_address": "v0.ryu.oracle-client.testnet", "callback_method": "token_price_callback", "nonce": "3", "data_version": "1", "data": [66, 65, 84]}' --accountId v0.ryu.oracle-client.testnet --gas 10000000000000000

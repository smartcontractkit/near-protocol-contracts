#!/bin/bash
../../near-shell/bin/near call v0.oracle.testnet fulfill_request '{"request_id": "v0.ryu.oracle-client.testnet:3", "payment": "1", "callback_address": "v0.ryu.oracle-client.testnet", "callback_method": "token_price_callback", "expiration": "1906293427246306700", "data": [66, 65, 84]}' --accountId oracle-trusted-1.test --gas 10000000000000000

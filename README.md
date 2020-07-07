# near-protocol-contracts

This is using flow adapted for https://github.com/nearprotocol/near-bindgen/blob/master/examples/fun-token/src/lib.rs#L63 as token.

1. Consumer calls set_allowance on token contract to allow oracle contract to charge given account.
2. Consumer calls oracleRequest in oracle contract.
3. oracleRequest calls lock in token contract to lock tokens to be used for payment.
4. Oracle contract gets called by provider with request result (`fulfillOracleRequest`).
5. Oracle contract calls transfer_from on token contract to transfer previously locked tokens from consumer account to provider account.

Alternatively:
instead of doing lock before fulfillment and then transfer_from after fulfillment it's possible to just charge immediately. The only catch here is around what happens in case of error / request not being fulfilled.

Notes:

- 128-bit numbers confirmed to be enough for payment, nonce and dataVersion
- specId is the same as a Job ID. The specs themselves must be defined by the node, and the requester initiates a run of that spec by providing its Job ID (or Spec ID,, these terms can be used interchangeably). The specs do not have to be from a pre-defined set, it's up to the node operator to create them. It is not possible, and not advised, for a requester to be able to pass in the full JSON of a job. That opens up the node to attack from malicious job specs that they haven't vetted.
- data should be straight JSON instead of JSON-like CBOR

## Set up ability to run on testnet

Set an environment variable to use in these examples. For instance, if your test account is `oracle.testnet` set it like so:

```bash
export NEAR_ACCT=oracle.testnet
```

Create a NEAR testnet account with [Wallet](https://wallet.testnet.near.org).
Create a subaccounts in this fashion:

```bash
near create_account oracle.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create_account client.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create_account oracle-node.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create_account near-link.$NEAR_ACCT --masterAccount $NEAR_ACCT
```

**Oracle client** will call the **oracle contract** to make a request for external data.
**Oracle client** has given the **oracle contract** allowance to take NEAR LINK from it. Before officially adding the request, it will `transfer_from` to capture the payment, keeping track of this amount in the `withdrawable_token` state variable.
The **oracle node** will be polling the state of its **oracle contract** using the paginated `get_requests` function.

Build the oracle, client, and NEAR LINK contracts with:

```bash
./build_all.sh
```

Then deploy and instantiate like so…

NEAR LINK

```bash
near deploy --accountId near-link.$NEAR_ACCT --wasmFile near-link-token/res/near_link_token.wasm
near call near-link.$NEAR_ACCT new '{"owner_id": "near-link.'$NEAR_ACCT'", "total_supply": "1000000"}' --accountId near-link.$NEAR_ACCT
```

Oracle contract

```bash
near deploy --accountId oracle.$NEAR_ACCT --wasmFile oracle/res/oracle.wasm
near call oracle.$NEAR_ACCT new '{"link_id": "near-link.'$NEAR_ACCT'", "owner_id": "oracle.'$NEAR_ACCT'"}' --accountId oracle.$NEAR_ACCT
```

Oracle client

This contract is very bare-bones and does not need an initializing call with `new`

```bash
near deploy --accountId client.$NEAR_ACCT --wasmFile client/res/client.wasm
```

## Give fungible tokens and set allowances

Give 50 NEAR LINK to client:

```bash
near call near-link.$NEAR_ACCT transfer '{"new_owner_id": "client.'$NEAR_ACCT'", "amount": "50"}' --accountId near-link.$NEAR_ACCT --amount .0365
```

**Note**: above, we use the `amount` flag in order to pay for the state required.

(Optional) Check balance to confirm:

```bash
near view near-link.$NEAR_ACCT get_balance '{"owner_id": "client.'$NEAR_ACCT'"}'
```

**Oracle client** gives **oracle contract** allowance to spend 20 NEAR LINK on their behalf:

```bash
near call near-link.$NEAR_ACCT inc_allowance '{"escrow_account_id": "oracle.'$NEAR_ACCT'", "amount": "20"}' --accountId client.$NEAR_ACCT --amount .0696
```

(Optional) Check allowance to confirm:

```bash
near view near-link.$NEAR_ACCT get_allowance '{"owner_id": "client.'$NEAR_ACCT'", "escrow_account_id": "oracle.'$NEAR_ACCT'"}'
```

**Oracle client** makes a request to **oracle contract** with payment of 10 NEAR LINK:

```bash
near call oracle.$NEAR_ACCT request '{"payment": "10", "spec_id": "dW5pcXVlIHNwZWMgaWQ=", "callback_address": "client.'$NEAR_ACCT'", "callback_method": "token_price_callback", "nonce": "1", "data_version": "1", "data": "QkFU"}' --accountId client.$NEAR_ACCT --gas 10000000000000000
```

Before the **oracle node** can fulfill the request, they must be authorized.

```bash
near call oracle.$NEAR_ACCT add_authorization '{"node": "oracle-node.'$NEAR_ACCT'"}' --accountId oracle.$NEAR_ACCT
```

(Optional) Check authorization to confirm:

```bash
near view oracle.$NEAR_ACCT is_authorized '{"node": "oracle-node.'$NEAR_ACCT'"}'
```

Oracle node is polling the state of **oracle contract** to see paginated request _summary_, which shows which accounts have requests pending and how many total are pending:

    near view oracle.$NEAR_ACCT get_requests_summary '{"max_num_accounts": "10"}'

**Note**: aside from `get_requests_summary` there is also `get_requests_summary_from`. Since the `TreeMap` data structure is ordered, the former will list the first N (`max_num_accounts`). Usage of `get_requests_summary_from` is for paging, providing a window of results to return. Please see function details for parameters and usage.

The previous command is useful if there has been significant scaling from many client accounts/contracts. To see the individual requests for a particular user, use the following command:

```bash
near view oracle.$NEAR_ACCT get_requests '{"account": "client.'$NEAR_ACCT'", "max_requests": "10"}'
```

Or if you have [jq installed](https://stedolan.github.io/jq/) you may use:

```bash
near view oracle.$NEAR_ACCT get_requests '{"account": "client.'$NEAR_ACCT'", "max_requests": "10"}' | tail -n 1 | sed "s/.\[32m'//g; s/'.\[39m//g" | jq
```

It sees the `data` is `QkFU` which is the Base64-encoded string for `BAT`, the token to look up. The **oracle node** presumably makes a call to an exchange to gather the price of Basic Attention Token (BAT) and finds it is at \$0.19 per token.
The data `0.19` as a Vec<u8> is `MTkuMQ==`

**Oracle node** uses its NEAR account keys to fulfill the request:

```bash
near call oracle.$NEAR_ACCT fulfill_request '{"account": "client.'$NEAR_ACCT'", "nonce": "1", "payment": "10", "callback_address": "client.'$NEAR_ACCT'", "callback_method": "token_price_callback", "expiration": "1906293427246306700", "data": "MTkuMQ=="}' --accountId oracle-node.$NEAR_ACCT --gas 10000000000000000
```

(Optional) Check the balance of **oracle client**:

```bash
near view near-link.$NEAR_ACCT get_balance '{"owner_id": "client.'$NEAR_ACCT'"}'
```

Expect `40`

(Optional) Check the allowance of **oracle contract**:

```bash
near view near-link.$NEAR_ACCT get_allowance '{"owner_id": "client.'$NEAR_ACCT'", "escrow_account_id": "oracle.'$NEAR_ACCT'"}'
```

Expect `10`

The oracle node and oracle contract are assumed to be owned by the same person/entity. The oracle contract has "withdrawable tokens" that can be taken when it's most convenient. Some oracles may choose to transfer these tokens immediately after fulfillment. Here we are using the withdrawable pattern, where gas is conserved by not transferring after each request fulfillment.

(Optional) Check the withdrawable tokens on the oracle contract with this command:

```bash
near view oracle.$NEAR_ACCT get_withdrawable_tokens
```

(Optional) Check the fungible token balance of the client and the base account we'll be extracting to it. (This is the original account we set the `NEAR_ACCT` environment variable to, for demonstration purposes)

```bash
near view near-link.$NEAR_ACCT get_balance '{"owner_id": "oracle.'$NEAR_ACCT'"}'
near view near-link.$NEAR_ACCT get_balance '{"owner_id": "'$NEAR_ACCT'"}'
```

Finally, withdraw the fungible tokens from the oracle contract into another account, the base account, who presumably owns both the oracle node and oracle contract.

```bash
near call oracle.$NEAR_ACCT withdraw '{"recipient": "'$NEAR_ACCT'", "amount": "20"}' --accountId oracle.$NEAR_ACCT --gas 10000000000000000
```

You may use the previous two `get_balance` view methods to confirm that the fungible tokens have indeed been withdrawn.

## Notes

The client is responsible for making sure there is enough allowance for fungible token transfers. It may be advised to add a cushion in addition to expected fungible token transfers as duplicate requests will also decrease allowance.

**Scenario**: a client accidentally sends the same request or a request with the same nonce. The fungible token transfer occurs, decrementing the allowance on the fungible token contract. Then it is found that it's a duplicate, and the fungible tokens are returned. In this case, the allowance will not be increased as this can only be done by the client itself.

One way to handle this is for the client to have logic to increase the allowance if it receives the response indicating a duplicate request has been sent.

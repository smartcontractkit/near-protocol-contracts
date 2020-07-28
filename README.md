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
near create-account oracle.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account client.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account oracle-node.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account near-link.$NEAR_ACCT --masterAccount $NEAR_ACCT
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
near deploy --accountId near-link.$NEAR_ACCT --wasmFile near-link-token/res/near_link_token.wasm --initFunction new --initArgs '{"owner_id": "near-link.'$NEAR_ACCT'", "total_supply": "1000000"}'
```

Oracle contract

```bash
near deploy --accountId oracle.$NEAR_ACCT --wasmFile oracle/res/oracle.wasm --initFunction new --initArgs '{"link_id": "near-link.'$NEAR_ACCT'", "owner_id": "oracle.'$NEAR_ACCT'"}'
```

Oracle client

```bash
near deploy --accountId client.$NEAR_ACCT --wasmFile client/res/client.wasm --initFunction new --initArgs '{"oracle_account": "oracle.'$NEAR_ACCT'"}'
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

We'll show two ways to have the client contract send the oracle contract a request. First, we'll directly call the oracle contract using the key pair from the client contract.

1. **Oracle client** makes a request to **oracle contract** with payment of 10 NEAR LINK:

```bash
near call oracle.$NEAR_ACCT request '{"payment": "10", "spec_id": "dW5pcXVlIHNwZWMgaWQ=", "callback_address": "client.'$NEAR_ACCT'", "callback_method": "token_price_callback", "nonce": "1", "data_version": "1", "data": "QkFU"}' --accountId client.$NEAR_ACCT --gas 300000000000000
```

2. (For demo purposes) **Any NEAR account** calls the **oracle client** contract, providing a symbol. Upon receiving this, the **oracle client** sends a cross-contract call to the **oracle contract** to store the request. (Payment and other values are hardcoded here, the nonce is automatically incremented. This assumes that the **oracle client** contract only wants to use one oracle contract.)

```bash
near call client.$NEAR_ACCT demo_token_price '{"symbol": "QkFU"}' --accountId client.$NEAR_ACCT --gas 300000000000000
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

```bash
near view oracle.$NEAR_ACCT get_requests_summary '{"max_num_accounts": "10"}'
```

**Note**: aside from `get_requests_summary` there is also `get_requests_summary_from`. Since the `TreeMap` data structure is ordered, the former will list the first N (`max_num_accounts`). Usage of `get_requests_summary_from` is for paging, providing a window of results to return. Please see function details for parameters and usage.

For folks who prefer to see a more low-level approach to hitting the RPC, here's the curl command:

```bash
curl -d '{"jsonrpc": "2.0", "method": "query", "id": "chainlink", "params": {"request_type": "call_function", "finality": "final", "account_id": "oracle.'$NEAR_ACCT'", "method_name": "get_requests_summary", "args_base64": "eyJtYXhfbnVtX2FjY291bnRzIjogIjEwIn0="}}' -H 'Content-Type: application/json' https://rpc.testnet.near.org
```

The above will return something like:
```json
{"jsonrpc":"2.0","result":{"result":[91,123,34,97,99,99,111,117,110,116,34,58,34,99,108,105,101,110,116,46,100,101,109,111,46,116,101,115,116,110,101,116,34,44,34,116,111,116,97,108,95,114,101,113,117,101,115,116,115,34,58,49,125,93],"logs":[],"block_height":10551293,"block_hash":"Ljh67tYk5bGXPu9TamJNG4vHp18cEBDxebKHpEUeZEo"},"id":"chainlink"}
```

We'll outline a quick way to see results if the machine has Python installed. Copy the value of the innermost `result` key, which is an array of unsigned 8-bit integers.

Open the Python REPL with the command `python` and see the prompt. (It should take input after `>>>`)

Enter the below replacing BYTE_ARRAY with the the result value (including the square brackets):

```python
res = BYTE_ARRAY
```

then

```python
''.join(chr(x) for x in res)
```

and python will print something like:

```text
'[{"account":"client.demo.testnet","total_requests":1}]'
```

The previous command (calling the method `get_requests_summary`) is useful if there has been significant scaling from many client accounts/contracts. To see the individual requests for a particular user, use the following command:

```bash
near view oracle.$NEAR_ACCT get_requests '{"account": "client.'$NEAR_ACCT'", "max_requests": "10"}'
```

It sees the `data` is `QkFU` which is the Base64-encoded string for `BAT`, the token to look up. The **oracle node** presumably makes a call to an exchange to gather the price of Basic Attention Token (BAT) and finds it is at \$0.19 per token.
The data `0.19` as a Vec<u8> is `MTkuMQ==`

There's a third method to get all the requests, ordered by account name and nonce, where a specified maximum number of results is provided.

```bash
near view oracle.$NEAR_ACCT get_all_requests '{"max_num_accounts": "100", "max_requests": "100"}'
```

**Oracle node** uses its NEAR account keys to fulfill the request:

```bash
near call oracle.$NEAR_ACCT fulfill_request '{"account": "client.'$NEAR_ACCT'", "nonce": "1", "data": "MTkuMQ=="}' --accountId oracle-node.$NEAR_ACCT --gas 300000000000000
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

Finally, withdraw the fungible tokens from the oracle contract into the oracle node, the base account, who presumably owns both the oracle node and oracle contract.

```bash
near call oracle.$NEAR_ACCT withdraw '{"recipient": "oracle-node.'$NEAR_ACCT'", "amount": "10"}' --accountId oracle.$NEAR_ACCT --gas 300000000000000
```

You may use the previous two `get_balance` view methods to confirm that the fungible tokens have indeed been withdrawn.

## Notes

The client is responsible for making sure there is enough allowance for fungible token transfers. It may be advised to add a cushion in addition to expected fungible token transfers as duplicate requests will also decrease allowance.

**Scenario**: a client accidentally sends the same request or a request with the same nonce. The fungible token transfer occurs, decrementing the allowance on the fungible token contract. Then it is found that it's a duplicate, and the fungible tokens are returned. In this case, the allowance will not be increased as this can only be done by the client itself.

One way to handle this is for the client to have logic to increase the allowance if it receives the response indicating a duplicate request has been sent.

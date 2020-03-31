# near-protocol-contracts

This is using flow adapted for https://github.com/nearprotocol/near-bindgen/blob/master/examples/fun-token/src/lib.rs#L63 as token.

1) Consumer calls set_allowance on token contract to allow oracle contract to charge given account.
2) Consumer calls oracleRequest in oracle contract.
3) oracleRequest calls lock in token contract to lock tokens to be used for payment.
4) Oracle contract gets called by provider with request result (`fulfillOracleRequest`).
5) Oracle contract calls transfer_from on token contract to transfer previously locked tokens from consumer account to provider account.

Alternatively:
instead of doing lock before fulfillment and then transfer_from after fulfillment it's possible to just charge immediately. The only catch here is around what happens in case of error / request not being fulfilled.

Notes:
- 128-bit numbers confirmed to be enough for payment, nonce and dataVersion
- specId  is the same as a Job ID. The specs themselves must be defined by the node, and the requester initiates a run of that spec by providing its Job ID (or Spec ID,, these terms can be used interchangeably). The specs do not have to be from a pre-defined set, it's up to the node operator to create them. It is not possible, and not advised, for a requester to be able to pass in the full JSON of a job. That opens up the node to attack from malicious job specs that they haven't vetted.
- data should be straight JSON instead of JSON-like CBOR

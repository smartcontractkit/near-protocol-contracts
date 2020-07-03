#!/bin/bash
near call near-link.$NEAR_ACCT new '{"owner_id": "near-link.'$NEAR_ACCT'", "total_supply": "1000000"}' --accountId near-link.$NEAR_ACCT
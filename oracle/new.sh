#!/bin/bash
near call oracle.$NEAR_ACCT new '{"link_id": "near-link.'$NEAR_ACCT'", "owner_id": "oracle.'$NEAR_ACCT'"}' --accountId oracle.$NEAR_ACCT

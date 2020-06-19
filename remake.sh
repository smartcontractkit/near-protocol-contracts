#!/bin/bash
near delete oracle.$NEAR_ACCT $NEAR_ACCT
near create_account oracle.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete oracle-client.$NEAR_ACCT $NEAR_ACCT
near create_account oracle-client.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete oracle-node.$NEAR_ACCT $NEAR_ACCT
near create_account oracle-node.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete near-link.$NEAR_ACCT $NEAR_ACCT
near create_account near-link.$NEAR_ACCT --masterAccount $NEAR_ACCT

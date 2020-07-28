#!/bin/bash
near delete oracle.$NEAR_ACCT $NEAR_ACCT
near create-account oracle.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete client.$NEAR_ACCT $NEAR_ACCT
near create-account client.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete oracle-node.$NEAR_ACCT $NEAR_ACCT
near create-account oracle-node.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete near-link.$NEAR_ACCT $NEAR_ACCT
near create-account near-link.$NEAR_ACCT --masterAccount $NEAR_ACCT

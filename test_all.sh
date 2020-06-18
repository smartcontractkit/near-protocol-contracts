#!/bin/bash

cd near-link-token && ./test.sh && cd ..
cd oracle && ./test.sh && cd ..
cd oracle-client && ./test.sh && cd ..
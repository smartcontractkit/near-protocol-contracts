#!/bin/bash
set -e

cd near-link-token && ./build.sh && cd ..
cd oracle && ./build.sh && cd ..
cd oracle-client && ./build.sh && cd ..


#!/bin/bash

cargo build --target wasm32-unknown-unknown --release
mkdir -p ./res
cp target/wasm32-unknown-unknown/release/near_link_token.wasm ./res

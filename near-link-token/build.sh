#!/bin/bash
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/near_link_token.wasm ./res/
#wasm-opt -Oz --output ./res/fun_token.wasm ./res/fun_token.wasm

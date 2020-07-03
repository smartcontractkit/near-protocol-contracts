#!/bin/bash
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/client.wasm ./res/
#wasm-opt -Oz --output ./res/fun_token.wasm ./res/fun_token.wasm

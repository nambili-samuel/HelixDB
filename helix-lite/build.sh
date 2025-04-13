#!/bin/bash

mkdir -p bindings/swift
mkdir -p bindings/kotlin
mkdir -p bindings/js

cargo build --release

cargo build

npm install
npm run build

cp target/release/libhelix_lite.* bindings/swift/
cp target/release/libhelix_lite.* bindings/kotlin/
cp target/release/helix_lite.*.node bindings/js/
#!/bin/bash

cargo build --release --bin server
cargo build --release --bin client
cargo build --target x86_64-pc-windows-gnu --release --bin server
cargo build --target x86_64-pc-windows-gnu --release --bin client
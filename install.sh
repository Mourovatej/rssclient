#!/bin/bash
cargo build --release
sudo cp target/release/rssclient /usr/local/bin/rssclient

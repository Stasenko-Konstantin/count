#!/bin/sh

cargo build --release
sudo cp target/release/count /bin/count
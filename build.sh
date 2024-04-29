#!/bin/sh

cargo build --release --bin client

cp ./target/release/client ~/bin/lurk-knight

ls -alh ~/bin/
#!/bin/sh

cargo build --release --bin client

cp ./target/release/client ~/bin/lurk-knight
cp ~/spring24/cs435/lurk_server/.env ~/bin/.env

ls -alh ~/bin/
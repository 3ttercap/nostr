#!/bin/bash

set -euo pipefail

args=(
    "-p nostr"
    "-p nostr-database"
    "-p nostr-sqlite"
    "-p nostr-rocksdb"
    "-p nostr-indexeddb"
    "-p nostr-relay-pool"
    "-p nostr-signer"
    "-p nostr-zapper"
    "-p nostr-webln"
    "-p nwc"
    "-p nostr-sdk"
)

for arg in "${args[@]}"; 
do
    echo "Publishing '$arg'"
    cargo publish $arg
    echo
done
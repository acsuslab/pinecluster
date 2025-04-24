#!/bin/bash

echo "Reading nodelist..."

# Read all nodes into an array to avoid stdin interference
mapfile -t nodes < nodelist

for node in "${nodes[@]}"; do
  node=$(echo "$node" | tr -d '\r')  # Clean carriage return if any
  echo "=== Creating private key on $node ==="

  ssh -i rpi.key node@"${node}.local" bash -c "'
    mkdir -p ~/Ebeacon &&
    openssl genpkey -algorithm RSA -out ~/Ebeacon/private.pem -pkeyopt rsa_keygen_bits:2048 &&
    chmod 600 ~/Ebeacon/private.pem &&
    echo Private key created on ${node}
  '"
done

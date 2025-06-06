#!/bin/bash
# generate_public_keys.sh
# This script reads your nodelist, connects to each node,
# and generates the public key from the existing private key
# (located at ~/Ebeacon/private.pem) and writes it to ~/Ebeacon/public.pem.
# It then prints out the public key from each node so you can verify uniqueness.

echo "Reading nodelist..."
mapfile -t nodes < nodelist

for node in "${nodes[@]}"; do
  # Clean carriage returns
  node=$(echo "$node" | tr -d '\r')
  echo "=== Processing node: $node ==="
  
  # Connect and generate the public key if a private key exists.
  ssh -i rpi.key -o StrictHostKeyChecking=no node@"${node}.local" bash -c "'
    mkdir -p ~/Ebeacon
    if [ -f ~/Ebeacon/private.pem ]; then
      # Generate public key from the private key
      openssl rsa -in ~/Ebeacon/private.pem -pubout -out ~/Ebeacon/public.pem
      chmod 644 ~/Ebeacon/public.pem
      echo \"Public key generated on ${node}\"
    else
      echo \"No private key found on ${node}\"
    fi
  '"
done

echo "=== Verifying public keys on all nodes ==="
# Loop again to print each node's public key for verification.
for node in "${nodes[@]}"; do
  node=$(echo "$node" | tr -d '\r')
  echo "Node: $node"
  ssh -i rpi.key -o StrictHostKeyChecking=no node@"${node}.local" "cat ~/Ebeacon/public.pem || echo 'No public key found'"
  echo "--------------------------"
done

echo "All nodes processed."

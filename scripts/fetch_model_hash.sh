#!/usr/bin/env bash
set -e

URL="https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4_k_m.gguf"
echo "Fetching SHA-256 for: $URL"

# We can query the HuggingFace API for the sha256 of the LFS file
HASH=$(curl -s "https://huggingface.co/api/models/microsoft/Phi-3-mini-4k-instruct-gguf/tree/main" | grep -o '"oid":"[a-f0-9]*"' | head -n 1 | cut -d'"' -f4)

if [ -z "$HASH" ]; then
    echo "Could not fetch SHA-256."
    exit 1
fi

echo "SHA-256: $HASH"
echo "Paste this value into crates/sprawl-inference/src/lib.rs DEFAULT_MODEL"

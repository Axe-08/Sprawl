#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="$HOME/.sprawl/models"
MODEL_FILE="Phi-3-mini-4k-instruct-q4.gguf"
MODEL_URL="https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/$MODEL_FILE"

mkdir -p "$MODEL_DIR"

if [ ! -f "$MODEL_DIR/$MODEL_FILE" ]; then
  echo "[sprawl] Downloading model (~2.4GB) — this is a one-time operation..."
  curl -L --progress-bar -o "$MODEL_DIR/$MODEL_FILE" "$MODEL_URL"
  echo "[sprawl] Download complete."
else
  echo "[sprawl] Model already present at $MODEL_DIR/$MODEL_FILE"
fi

TOKENIZER_FILE="tokenizer.json"
TOKENIZER_URL="https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/$TOKENIZER_FILE"

if [ ! -f "$MODEL_DIR/$TOKENIZER_FILE" ]; then
  echo "[sprawl] Downloading tokenizer.json..."
  curl -L --progress-bar -o "$MODEL_DIR/$TOKENIZER_FILE" "$TOKENIZER_URL"
else
  echo "[sprawl] Tokenizer already present at $MODEL_DIR/$TOKENIZER_FILE"
fi

echo "[sprawl] Running inference tests..."
cargo test -p sprawl-inference --features inference -- --include-ignored
cargo test -p sprawl-sentinel --features sprawl-inference/inference -- --include-ignored
echo "[sprawl] Done."

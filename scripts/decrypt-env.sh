#!/usr/bin/env bash
set -euo pipefail

if ! command -v sops >/dev/null 2>&1; then
  echo "sops is required (https://github.com/mozilla/sops)" >&2
  exit 1
fi

ENVIRONMENT="${1:-prod}"
FORMAT="${2:-dotenv}"
ENC_FILE="secrets/${ENVIRONMENT}.env.enc"
OUT_FILE="secrets/.${ENVIRONMENT}.env"

if [[ ! -f "${ENC_FILE}" ]]; then
  echo "Encrypted env file ${ENC_FILE} not found." >&2
  exit 1
fi

umask 177
sops --decrypt \
  --input-type "${FORMAT}" \
  --output-type "${FORMAT}" \
  "${ENC_FILE}" > "${OUT_FILE}"
echo "Decrypted environment written to ${OUT_FILE}"
echo "Remember to securely remove the file when finished."

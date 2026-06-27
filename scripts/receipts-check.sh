#!/usr/bin/env bash
set -euo pipefail
if [ ! -f "receipts/v26.6.28-dx.receipt.json" ]; then echo "Missing dx receipt"; exit 1; fi
if [ ! -f "receipts/v26.6.28-qol.receipt.json" ]; then echo "Missing qol receipt"; exit 1; fi
if [ ! -f "receipts/v26.6.28-doctor.receipt.json" ]; then echo "Missing doctor receipt"; exit 1; fi
echo "Receipts present."

#!/usr/bin/env bash
# Run this once on a fresh Ubuntu/Debian VPS as root (or with sudo).
# After it finishes, fill in /opt/solana-arb/.env.production and start the stack.

set -euo pipefail

REPO_DIR=/opt/solana-arb

echo "==> Installing Docker..."
apt-get update -qq
apt-get install -y --no-install-recommends \
    ca-certificates curl gnupg lsb-release git

curl -fsSL https://get.docker.com | sh
systemctl enable --now docker

# Allow current user to run docker without sudo (takes effect on next login)
if [[ -n "${SUDO_USER:-}" ]]; then
    usermod -aG docker "$SUDO_USER"
fi

echo "==> Docker $(docker --version)"
echo "==> Docker Compose $(docker compose version)"

echo ""
echo "==> Copying project files..."
echo "    (This script expects you to upload the repo yourself.)"
echo "    Recommended: rsync -avz --exclude node_modules --exclude target \\"
echo "        ./ root@YOUR_SERVER_IP:${REPO_DIR}/"
echo ""

# If the directory already exists, just confirm; otherwise remind user to upload.
if [[ -d "${REPO_DIR}" ]]; then
    echo "==> Found ${REPO_DIR}"
else
    echo "    ${REPO_DIR} not found — upload your project there first, then re-run."
    exit 1
fi

cd "${REPO_DIR}"

echo "==> Checking .env.production..."
if [[ ! -f .env.production ]]; then
    echo "    .env.production not found — you should have one from the repo."
    exit 1
fi

# Warn about unfilled placeholders
if grep -q "YOUR_HELIUS_KEY\|change-me\|YOUR_WALLET" .env.production; then
    echo ""
    echo "  !! .env.production still has placeholder values."
    echo "     Edit ${REPO_DIR}/.env.production and fill in your secrets,"
    echo "     then run:  cd ${REPO_DIR} && docker compose up -d --build"
    echo ""
    exit 0
fi

echo ""
echo "==> Building and starting services (first build takes ~20 min for Rust)..."
docker compose up -d --build

echo ""
echo "==> Stack is up. Service status:"
docker compose ps

echo ""
echo "==> Useful commands:"
echo "    docker compose logs -f          # stream all logs"
echo "    docker compose logs -f control-api"
echo "    docker compose ps               # check health"
echo "    docker compose down             # stop everything"
echo "    docker compose pull && docker compose up -d --build  # redeploy"

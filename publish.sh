#!/usr/bin/env bash
# Creates the sphatikOS repository on your GitHub account, commits everything and pushes.
# Run from inside this folder:  bash publish.sh
set -euo pipefail
cd "$(dirname "$0")"

REPO_NAME="sphatikOS"
VISIBILITY="--private"   # change to --public when you are ready to share

if ! git config user.name >/dev/null || ! git config user.email >/dev/null; then
  echo "Set your git identity first:"
  echo '  git config --global user.name  "Your Name"'
  echo '  git config --global user.email "you@example.com"'
  exit 1
fi

if [ ! -d .git ]; then
  git init -b main
fi
git add .
git commit -m "docs: initial Sphatik OS docs, ADRs, API specs, guides and shell prototype" || echo "Nothing new to commit."

if command -v gh >/dev/null 2>&1; then
  gh auth status >/dev/null 2>&1 || gh auth login
  gh repo create "$REPO_NAME" $VISIBILITY --source . --remote origin --push \
    --description "Sphatik OS: a liquid-glass phone operating system built in Rust"
  echo "Done: $(gh repo view --json url -q .url)"
else
  echo
  echo "GitHub CLI (gh) not found. Either install it (https://cli.github.com) and re-run,"
  echo "or create an EMPTY repository named $REPO_NAME on github.com, then run:"
  echo "  git remote add origin https://github.com/<your-username>/$REPO_NAME.git"
  echo "  git push -u origin main"
fi

#!/usr/bin/env bash
set -euo pipefail

echo "Current working directory: $(pwd)"

if [ ! -f "scripts/update-reference-repos.sh" ]; then
  echo "Error: Please run this script from the root of the project."
  exit 1
fi

mkdir -p reference-repos

clone_if_missing() {
  local name="$1" url="$2"
  local dest="reference-repos/$name"
  if [ ! -d "$dest" ]; then
    echo "Cloning $name..."
    git clone --depth 1 --single-branch --branch main "$url" "$dest"
  else
    echo "$name already exists, skipping clone."
  fi
}

clone_if_missing litellm "https://github.com/BerriAI/litellm.git"
clone_if_missing cline   "https://github.com/cline/cline.git"

for dir in reference-repos/*/; do
  if [ -d "$dir/.git" ]; then
    echo "Updating $dir..."
    (cd "$dir" && git pull origin main)
  else
    echo "Skipping $dir — not a git repository."
  fi
done

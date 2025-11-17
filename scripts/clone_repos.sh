#!/usr/bin/env bash
set -euo pipefail

while [[ $# -gt 0 ]]; do
  case $1 in
    --whitelist)
      WHITELIST="$2"
      shift 2
      ;;
    --dest)
      DEST="$2"
      shift 2
      ;;
  esac
done

branch=$(yq '.branch' "$WHITELIST")
repos=$(yq '.repos[]' "$WHITELIST")

mkdir -p "$DEST"

echo "[clone] Destination: $DEST"

for repo in $repos; do
  name=$(basename "$repo" .git)
  target="$DEST/$name"

  if [[ -d "$target/.git" ]]; then
    echo "[clone] Updating: $name"
    git -C "$target" fetch
    git -C "$target" checkout "$branch"
    git -C "$target" pull --ff-only
  else
    echo "[clone] Cloning: $repo"
    git clone --branch "$branch" "$repo" "$target"
  fi
done

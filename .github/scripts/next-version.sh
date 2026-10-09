#!/usr/bin/env bash
# Следующая версия по Conventional Commits с последнего тега vX.Y.Z:
# `type!:` или BREAKING CHANGE → major, feat → minor, остальное → patch.
# Тегов нет — версия из app/package.json как есть.
set -euo pipefail

last="$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*' 2>/dev/null || true)"
if [ -z "$last" ]; then
  node -p "require('./app/package.json').version"
  exit 0
fi

# Тела тоже: в merge-коммите PR заголовок PR лежит в теле.
log="$(git log --format='%s%n%b' "$last"..HEAD)"
IFS=. read -r major minor patch <<<"${last#v}"

if grep -qE '^[a-z]+(\([^)]*\))?!:|^BREAKING[ -]CHANGE:' <<<"$log"; then
  echo "$((major + 1)).0.0"
elif grep -qE '^feat(\([^)]*\))?:' <<<"$log"; then
  echo "$major.$((minor + 1)).0"
else
  echo "$major.$minor.$((patch + 1))"
fi

#!/bin/sh
# Patch the Cargo.toml version using nsv

sed -i '' "s/^version = \"$NSV_PREV_TAG\"/version = \"$NSV_NEXT_TAG\"/" \
  "$NSV_WORKING_DIRECTORY/Cargo.toml"

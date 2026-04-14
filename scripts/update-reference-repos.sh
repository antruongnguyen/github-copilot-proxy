#!/bin/bash

# print current working directory for debugging purposes
echo "Current working directory: $(pwd)"

# Check if the current directory is the root of the project
if [ ! -f "scripts/update-reference-repos.sh" ]; then
  echo "Error: Please run this script from the root of the project."
  exit 1
fi

cd reference-repos

# Update the reference repositories
# for each subdirectory, pull the latest changes from the remote repository
for dir in */; do
  if [ -d "$dir/.git" ]; then
    echo "Updating repository: $dir"
    cd "$dir"
    git pull origin main
    cd ..
  else
    echo "Skipping $dir, not a git repository."
  fi
done

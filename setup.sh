#!/bin/bash

source template/funcs.sh
# Directory of the script
ROOT_DIR="$(dirname "${BASH_SOURCE[0]}")"
TEMPLATE_DIR="$ROOT_DIR/template"

cd ${ROOT_DIR}

# Set variables to be replaced in the template files
export APP=$(git remote get-url origin | sed -E 's/^(https:\/\/|git@github\.com:).+\/([^\/]+)\.git$/\2/; t; s/.*/app/')
export GIT_USERNAME=$(git remote get-url origin | sed -E 's/^(https:\/\/|git@github\.com:)([^\/]+).*$/\2/; t; s/.*/username/')
export YEAR=$(date +%Y)
check_var APP GIT_USERNAME YEAR

# Copy files recursively and replace variables
while IFS= read -r -d '' file; do
    # Determine relative path and destination path
    REL_PATH="${file#$TEMPLATE_DIR/}"
    DEST_PATH="$ROOT_DIR/$REL_PATH"

    # Create destination directory if it doesn't exist
    mkdir -p "$(dirname "$DEST_PATH")"

    # Replace variables using envsubst and copy the file
    envsubst '${APP} ${GIT_USERNAME} ${YEAR}' < "$file" > "$DEST_PATH"
    echo "Processed: $file -> $DEST_PATH"
done < <(find "$TEMPLATE_DIR" -type f -print0)

echo "Template render complete!"
rm -rf template setup.sh
git stage .

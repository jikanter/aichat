#!/bin/bash
for f in docs/demos/*.md; do
  if [[ "$f" == "docs/demos/demo-mcp-client.md" ]]; then
    echo "Skipping WIP demo: $f"
    continue
  fi
  echo "Checking $f..."
  # Run verify. If it fails (non-zero exit code), it means output has changed OR a command failed.
  # Showboat verify --output will try to regenerate the file.
  if ! showboat verify "$f" > /dev/null 2>&1; then
    echo "Changes or errors detected in $f, attempting update..."
    # We ignore the exit code because verify --output returns 1 if any command in the demo fails,
    # which is common for demos showing error cases.
    showboat verify "$f" --output "$f.tmp" > /dev/null 2>&1
    if [ -f "$f.tmp" ] && [ -s "$f.tmp" ]; then
      mv "$f.tmp" "$f"
      echo "Successfully updated $f"
    else
      echo "CRITICAL FAILURE: Could not update $f. No output file generated."
      rm -f "$f.tmp"
    fi
  else
    echo "$f is already up to date."
  fi
done

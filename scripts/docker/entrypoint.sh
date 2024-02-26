#!/bin/sh

# This script serves as the entrypoint for the Docker container.
# It checks if a user-defined configuration file exists, and if so,
# it uses that file; otherwise, it falls back to the default configuration.
export MODEL_PATH=/model
# Check if the user-defined configuration file exists
if [ -f "$QUERENT_NODE_CONFIG" ]; then
  echo "Using user-defined configuration file: $QUERENT_NODE_CONFIG"
  # Print the content of the configuration file
  # cat "$QUERENT_NODE_CONFIG"
  # Execute quester serve with the user-defined configuration
  exec querent serve --config "$QUERENT_NODE_CONFIG"
else
  echo "Using default configuration file: $QUERENT_CONFIG"
  # Print the content of the default configuration file
  cat "$QUERENT_CONFIG"
  # Execute quester serve with the default configuration
  exec querent serve --config "$QUERENT_CONFIG"
fi

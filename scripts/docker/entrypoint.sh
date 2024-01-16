#!/bin/sh

# This script serves as the entrypoint for the Docker container.
# It checks if a user-defined configuration file exists, and if so,
# it uses that file; otherwise, it falls back to the default configuration.

# Check if the user-defined configuration file exists
if [ -f "$QUESTER_NODE_CONFIG" ]; then
  echo "Using user-defined configuration file: $QUESTER_NODE_CONFIG"
  # Execute quester serve with the user-defined configuration
  exec ./querent serve --config "$QUESTER_NODE_CONFIG"
else
  echo "Using default configuration file: $QUESTER_CONFIG"
  # Execute quester serve with the default configuration
  exec ./querent serve --config "$QUESTER_CONFIG"
fi

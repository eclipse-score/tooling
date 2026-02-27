#!/usr/bin/env bash
# Wrapper to use system-installed npm/cdxgen
# This relies on npm/cdxgen being available in the system PATH

# Add common Node.js installation paths to PATH
export PATH="/home/lj/.nvm/versions/node/v24.13.0/bin:$PATH"
export PATH="$HOME/.nvm/versions/node/v24.13.0/bin:$PATH"
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"

# If called with "exec -- @cyclonedx/cdxgen", just run cdxgen directly
if [[ "$1" == "exec" && "$2" == "--" && "$3" == "@cyclonedx/cdxgen" ]]; then
    shift 3  # Remove "exec -- @cyclonedx/cdxgen"
    exec cdxgen "$@"
else
    # Otherwise, run npm normally
    exec npm "$@"
fi

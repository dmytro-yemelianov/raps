"""Entry point for running raps as a module.

This module allows running RAPS via `python -m raps` in addition to
the direct `raps` command.
"""

import os
import sys
import subprocess


def main():
    """Execute the raps binary with all provided arguments."""
    # Find the binary in the same directory as this module
    binary_name = "raps"
    if sys.platform == "win32":
        binary_name += ".exe"

    binary_path = os.path.join(os.path.dirname(__file__), binary_name)

    if not os.path.exists(binary_path):
        print(f"Error: RAPS binary not found at {binary_path}", file=sys.stderr)
        print("This may indicate a corrupted installation.", file=sys.stderr)
        print("Try reinstalling with: pip install --force-reinstall raps", file=sys.stderr)
        sys.exit(1)

    try:
        result = subprocess.run([binary_path] + sys.argv[1:])
        sys.exit(result.returncode)
    except Exception as e:
        print(f"Error executing RAPS: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

"""RAPS - Rust CLI for Autodesk Platform Services.

This package provides the RAPS command-line interface for interacting with
Autodesk Platform Services (APS) APIs.

Usage:
    raps --help
    raps --version
    raps bucket list
    raps auth test

For more information, visit: https://rapscli.xyz
"""

__version__ = "0.0.0"  # Set by maturin at build time

from ..__main__ import main

__all__ = ["main"]

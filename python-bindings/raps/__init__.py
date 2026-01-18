# RAPS - Python bindings for Autodesk Platform Services
#
# This module provides native Python bindings for RAPS, enabling
# programmatic access to Autodesk Platform Services.
#
# Example:
#     from raps import Client
#
#     client = Client.from_env()
#     buckets = client.buckets.list()
#     for bucket in buckets:
#         print(f"{bucket.key}: {bucket.policy}")

from .raps import (
    # Main client
    Client,
    # Data classes
    Bucket,
    Object,
    TranslationJob,
    Hub,
    Project,
    # Managers
    BucketsManager,
    ObjectsManager,
    HubsManager,
    # Exceptions
    RapsError,
    AuthenticationError,
    NotFoundError,
    RateLimitError,
    ValidationError,
    # Version
    __version__,
)

__all__ = [
    # Main client
    "Client",
    # Data classes
    "Bucket",
    "Object",
    "TranslationJob",
    "Hub",
    "Project",
    # Managers
    "BucketsManager",
    "ObjectsManager",
    "HubsManager",
    # Exceptions
    "RapsError",
    "AuthenticationError",
    "NotFoundError",
    "RateLimitError",
    "ValidationError",
    # Version
    "__version__",
]

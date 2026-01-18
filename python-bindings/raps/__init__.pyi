"""Type stubs for RAPS Python bindings."""

from typing import List, Optional

__version__: str

# Exceptions
class RapsError(Exception):
    """Base exception for all RAPS errors."""
    ...

class AuthenticationError(RapsError):
    """Raised when authentication fails."""
    ...

class NotFoundError(RapsError):
    """Raised when a resource is not found."""
    ...

class RateLimitError(RapsError):
    """Raised when API rate limit is exceeded."""
    ...

class ValidationError(RapsError):
    """Raised for invalid parameters."""
    ...

# Data classes
class Bucket:
    """Represents an OSS bucket."""

    key: str
    """Unique bucket identifier."""

    owner: str
    """Bucket owner ID."""

    created_date: int
    """Creation timestamp in milliseconds."""

    policy: str
    """Retention policy (transient, temporary, persistent)."""

    region: Optional[str]
    """Storage region (US or EMEA)."""

class Object:
    """Represents an object in an OSS bucket."""

    bucket_key: str
    """Bucket containing this object."""

    object_key: str
    """Object key (filename)."""

    object_id: str
    """Full object URN."""

    size: int
    """Object size in bytes."""

    sha1: Optional[str]
    """SHA-1 hash of the object."""

    @property
    def urn(self) -> str:
        """Base64-encoded URN for Model Derivative API."""
        ...

class TranslationJob:
    """Represents a Model Derivative translation job."""

    urn: str
    """Source file URN."""

    status: str
    """Current status (pending, inprogress, success, failed, timeout)."""

    progress: str
    """Progress percentage or status message."""

    def wait(
        self,
        timeout: Optional[int] = None,
        poll_interval: Optional[int] = None,
    ) -> "TranslationJob":
        """
        Wait for the translation to complete.

        Args:
            timeout: Maximum seconds to wait (default: 600).
            poll_interval: Seconds between status checks (default: 5).

        Returns:
            Updated TranslationJob with final status.

        Raises:
            RuntimeError: If timeout is exceeded.
        """
        ...

class Hub:
    """Represents a Data Management hub."""

    id: str
    """Hub ID."""

    name: str
    """Hub display name."""

    hub_type: str
    """Hub type (hubs:autodesk.core:Hub, hubs:autodesk.a360:PersonalHub, etc.)."""

    region: Optional[str]
    """Hub region."""

class Project:
    """Represents a project in a hub."""

    id: str
    """Project ID."""

    name: str
    """Project display name."""

    project_type: str
    """Project type."""

# Managers
class BucketsManager:
    """Manager for bucket operations."""

    def list(
        self,
        region: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Bucket]:
        """
        List all buckets.

        Args:
            region: Optional region filter ("US" or "EMEA").
            limit: Maximum number of buckets to return.

        Returns:
            List of Bucket objects.
        """
        ...

    def create(
        self,
        key: str,
        policy: Optional[str] = None,
        region: Optional[str] = None,
    ) -> Bucket:
        """
        Create a new bucket.

        Args:
            key: Bucket key (unique identifier).
            policy: Retention policy ("transient", "temporary", "persistent").
            region: Storage region ("US" or "EMEA").

        Returns:
            Created Bucket object.
        """
        ...

    def get(self, key: str) -> Bucket:
        """
        Get bucket details.

        Args:
            key: Bucket key.

        Returns:
            Bucket object.

        Raises:
            NotFoundError: If bucket doesn't exist.
        """
        ...

    def delete(self, key: str) -> None:
        """
        Delete a bucket.

        Args:
            key: Bucket key to delete.

        Raises:
            NotFoundError: If bucket doesn't exist.
        """
        ...

class ObjectsManager:
    """Manager for object operations within a bucket."""

    def list(self, limit: Optional[int] = None) -> List[Object]:
        """
        List objects in the bucket.

        Args:
            limit: Maximum number of objects to return.

        Returns:
            List of Object objects.
        """
        ...

    def upload(self, path: str, object_key: Optional[str] = None) -> Object:
        """
        Upload a file to the bucket.

        Args:
            path: Local file path to upload.
            object_key: Optional object key (defaults to filename).

        Returns:
            Uploaded Object.
        """
        ...

    def download(self, object_key: str, path: str) -> str:
        """
        Download an object from the bucket.

        Args:
            object_key: Key of the object to download.
            path: Local path to save the file.

        Returns:
            Path to downloaded file.
        """
        ...

    def delete(self, object_key: str) -> None:
        """
        Delete an object from the bucket.

        Args:
            object_key: Key of the object to delete.

        Raises:
            NotFoundError: If object doesn't exist.
        """
        ...

    def signed_url(self, object_key: str, minutes: Optional[int] = None) -> str:
        """
        Get a signed download URL for an object.

        Args:
            object_key: Key of the object.
            minutes: URL expiration in minutes (2-60, default: 2).

        Returns:
            Signed URL string.
        """
        ...

class HubsManager:
    """Manager for hub operations (requires 3-legged auth)."""

    def list(self) -> List[Hub]:
        """
        List all hubs (requires 3-legged authentication).

        Returns:
            List of Hub objects.

        Raises:
            AuthenticationError: If not logged in with 3-legged auth.
        """
        ...

# Main client
class Client:
    """
    Main RAPS client for interacting with Autodesk Platform Services.

    Create a client with explicit credentials:
        client = Client(client_id="xxx", client_secret="yyy")

    Or load from environment:
        client = Client.from_env()

    Example:
        client = Client.from_env()
        buckets = client.buckets.list()
        for bucket in buckets:
            print(f"{bucket.key}: {bucket.policy}")
    """

    def __init__(
        self,
        client_id: str,
        client_secret: str,
        base_url: Optional[str] = None,
    ) -> None:
        """
        Create a new RAPS client with 2-legged authentication.

        Args:
            client_id: APS application client ID.
            client_secret: APS application client secret.
            base_url: Optional API base URL (for testing).
        """
        ...

    @staticmethod
    def from_env() -> "Client":
        """
        Create a client from environment variables.

        Reads APS_CLIENT_ID and APS_CLIENT_SECRET from environment.

        Raises:
            ValidationError: If environment variables not set.
        """
        ...

    def test_auth(self) -> bool:
        """
        Test 2-legged authentication.

        Returns:
            True if authentication succeeds.

        Raises:
            AuthenticationError: If authentication fails.
        """
        ...

    @property
    def buckets(self) -> BucketsManager:
        """Get bucket operations manager."""
        ...

    @property
    def hubs(self) -> HubsManager:
        """Get hub operations manager (requires login for 3-legged auth)."""
        ...

    def objects(self, bucket_key: str) -> ObjectsManager:
        """
        Get object operations manager for a specific bucket.

        Args:
            bucket_key: The bucket to operate on.

        Returns:
            ObjectsManager for the bucket.
        """
        ...

    def translate(
        self,
        urn: str,
        output_format: Optional[str] = None,
        force: Optional[bool] = None,
    ) -> TranslationJob:
        """
        Start a translation job.

        Args:
            urn: Base64-encoded URN of the source file.
            output_format: Target format (default: "svf2").
            force: Force re-translation even if exists.

        Returns:
            TranslationJob object.
        """
        ...

    def get_translation_status(self, urn: str) -> TranslationJob:
        """
        Get translation status.

        Args:
            urn: Base64-encoded URN of the source file.

        Returns:
            TranslationJob with current status.
        """
        ...

    def get_urn(self, bucket_key: str, object_key: str) -> str:
        """
        Generate a URN for a bucket/object combination.

        Args:
            bucket_key: Bucket key.
            object_key: Object key.

        Returns:
            Base64-encoded URN string.
        """
        ...

    def __enter__(self) -> "Client":
        """Context manager entry."""
        ...

    def __exit__(
        self,
        exc_type: Optional[type],
        exc_val: Optional[BaseException],
        exc_tb: Optional[object],
    ) -> bool:
        """Context manager exit."""
        ...

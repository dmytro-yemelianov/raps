# Exit Codes

RAPS CLI uses standardized exit codes to help you write robust automation scripts.

| Code | Name | Description |
|------|------|-------------|
| `0` | **Success** | The command completed successfully. |
| `2` | **Invalid Arguments** | Command line usage error (e.g., missing flag, invalid value) or validation failure. |
| `3` | **Authentication Failure** | The command requires authentication, but the session is invalid, expired, or missing permissions. Run `raps auth login`. |
| `4` | **Not Found** | The requested resource (bucket, object, hub, etc.) could not be found. |
| `5` | **Remote/API Error** | A network error or 5xx server error occurred. These may be transient; retry strategies are recommended. |
| `6` | **Internal Error** | An unexpected internal error occurred within the CLI. |

## Examples

### Check for Auth Failure

```bash
raps bucket list
if [ $? -eq 3 ]; then
    echo "Session expired. Logging in..."
    raps auth login
fi
```

### Retry on Remote Error

```bash
MAX_RETRIES=3
COUNT=0

while [ $COUNT -lt $MAX_RETRIES ]; do
    raps translate status $URN --wait
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -eq 0 ]; then
        break
    elif [ $EXIT_CODE -eq 5 ]; then
        echo "Remote error, retrying..."
        COUNT=$((COUNT+1))
        sleep 5
    else
        echo "Permanent error: $EXIT_CODE"
        exit $EXIT_CODE
    fi
done
```
# RAPS CLI Exit Codes

The RAPS CLI uses standard exit codes to indicate the result of a command. This allows for robust error handling in automation scripts and CI/CD pipelines. 

By checking the exit code of a `raps` command, scripts can determine whether to retry an operation (e.g., on a transient remote error) or fail fast (e.g., on invalid arguments or missing resources).

## Exit Code Reference

| Exit Code | Name | Description | Suggested Action |
| :--- | :--- | :--- | :--- |
| **0** | **Success** | The command completed successfully. | Continue execution. |
| **2** | **Invalid Arguments** | The command was invoked with invalid flags, missing required arguments, or failed local validation. | Do not retry. Fix the script or command invocation. |
| **3** | **Auth Failure** | Authentication or authorization failed. This includes missing credentials, expired tokens, or insufficient permissions (HTTP 401/403). | Do not retry automatically. Prompt for login or check permissions. |
| **4** | **Not Found** | The targeted resource (bucket, project, user, etc.) could not be found (HTTP 404). | Do not retry. Verify the resource ID. |
| **5** | **Remote Error** | A remote or API error occurred, such as a server error (HTTP 5xx), a network timeout, or connection reset. | Retry the operation with exponential backoff. |
| **6** | **Internal Error** | An internal application error occurred within the CLI itself, such as an IO error or an unexpected panic. | Report the issue to the maintainers. |

## Scripting Example (Bash)

Here is an example of how to handle RAPS exit codes in a Bash script:

```bash
#!/bin/bash

raps admin user add-to-all-projects test@example.com --role "Project Admin"
EXIT_CODE=$?

case $EXIT_CODE in
  0)
    echo "Successfully added user to projects."
    ;;
  2)
    echo "Error: Invalid arguments provided to the CLI."
    exit 2
    ;;
  3)
    echo "Error: Authentication failed. Please run 'raps auth login' and try again."
    exit 3
    ;;
  4)
    echo "Error: The specified account or project was not found."
    exit 4
    ;;
  5)
    echo "Error: Remote API error. You might want to retry this operation."
    # Add retry logic here
    exit 5
    ;;
  6)
    echo "Error: Internal CLI error."
    exit 6
    ;;
  *)
    echo "Unknown error occurred (Exit code: $EXIT_CODE)."
    exit $EXIT_CODE
    ;;
esac
```

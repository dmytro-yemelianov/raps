## Summary

The CLI supports multiple output formats (JSON, YAML, CSV) but doesn't provide formal schema definitions for the output structures. This makes it harder for automation consumers to rely on the output format.

## Current Behavior

- Output structures are implicitly defined by the code
- No formal schema documentation exists
- Breaking changes to output format may go unnoticed
- Automation built on the CLI may break unexpectedly

## Proposed Solution

1. **Generate JSON Schema for output types**:
   - Use `schemars` (already in dependencies for MCP) to generate schemas
   - Publish schemas alongside releases

2. **Add output format versioning**:
   - Include version field in JSON/YAML output
   - Document backward compatibility policy for output

3. **Create output documentation**:
   - Document expected output structure for each command
   - Provide examples in documentation

## Expected Benefits

- Reliable automation/scripting
- Clear contract for output format
- Easier integration with other tools
- Confidence in upgrade compatibility

## References

- Related: `schemars` crate already used for MCP tool definitions
- Related: Stability policy in `docs/STABILITY.md`

## Summary

The plugin system, while functional, has some areas that could be improved for clarity and ease of use. The distinction between plugins and aliases isn't always clear to users.

## Current State

- Plugin discovery works for `raps-<name>` executables in PATH
- Aliases allow command shortcuts
- The relationship between plugins and aliases isn't obvious
- Plugin help/documentation is limited

## Proposed Solution

1. **Improve documentation**:
   - Add `raps plugin --help` with detailed examples
   - Explain plugin vs alias distinction clearly
   - Provide sample plugin templates

2. **Enhance plugin discovery**:
   - Add `raps plugin info <name>` for detailed plugin info
   - Show plugin version/description if available
   - Consider caching discovered plugins for performance

3. **Improve alias usability**:
   - Add `raps alias` as shortcut for `raps plugin alias`
   - Support alias parameters/templating
   - Allow aliases to be defined in config file

4. **Plugin development support**:
   - Add `raps plugin create <name>` scaffold command
   - Provide example plugins in documentation

## Expected Benefits

- Easier plugin adoption
- Clearer mental model for users
- Better developer experience for plugin authors

## References

- File: `src/plugins.rs`
- File: `src/commands/plugin.rs`
- Docs: `docs/plugins.md`

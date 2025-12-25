# License Migration: MIT → Apache 2.0 (v2.0.0)

## Overview

Starting with version 2.0.0, RAPS has transitioned from the MIT License to **Apache License 2.0**. This change was made to ensure better attribution preservation and provide explicit patent protection for both users and contributors.

## Why Apache 2.0?

1.  **Explicit Patent Protection**: Unlike MIT, Apache 2.0 includes an explicit grant of patent rights from contributors to users.
2.  **Attribution Preservation**: The requirement to preserve the `NOTICE` file in derivative works ensures that the original project and its contributors receive proper credit when the code is redistributed.
3.  **Enterprise Readiness**: Many enterprise legal departments prefer Apache 2.0 for its clearer definitions of "Contribution" and "Contributor".
4.  **Ecosystem Alignment**: Apache 2.0 is a standard in the Rust ecosystem, often used alongside or instead of MIT.

## What This Means for Users

*   ✅ **Same Permissions**: Commercial use, modification, and distribution are still fully permitted.
*   ✅ **No Copyleft**: You are not required to open-source your own code if you use RAPS.
*   ✅ **Compatibility**: Apache 2.0 is compatible with MIT and GPLv3.

### Requirements for Redistribution

If you redistribute RAPS or include it in your own project, you must:
1.  Include a copy of the `LICENSE` file.
2.  Preserve the `NOTICE` file in the root of your distribution.
3.  Include appropriate attribution headers in modified source files.

## Historical Migration Steps (for Reference)

The migration was performed as follows:

1.  **LICENSE**: Replaced MIT text with Apache 2.0 text.
2.  **NOTICE**: Created a new `NOTICE` file with project attribution.
3.  **Cargo.toml**: Updated the `license` field to `Apache-2.0`.
4.  **Source Headers**: Applied SPDX-style headers to all `.rs` files.
5.  **Documentation**: Updated `README.md`, `CHANGELOG.md`, and the documentation site.

## FAQ

### Can I still use RAPS commercially?
Yes. Apache 2.0 explicitly permits commercial use.

### Do I need to update my existing workflows?
No. The license change does not affect the functionality of the CLI.

### Is it compatible with my MIT project?
Yes. You can include Apache 2.0 licensed software in an MIT-licensed project.


## Dual Licensing Alternative

If you want maximum compatibility, you can also offer dual licensing:

```toml
# Cargo.toml
license = "MIT OR Apache-2.0"
```

This lets users choose which license suits their project. Keep both LICENSE-MIT and LICENSE-APACHE files in this case.

However, for your visibility goals, **Apache-2.0 alone is better** because it ensures the NOTICE file (with your attribution) is always preserved.

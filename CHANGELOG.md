# Changelog

## v0.0.24 (2026-04-19)

Changes since v0.0.22:

- refactor(ci): optimize workflows - simplify CI, fix release trigger, add caching
- fix(ci): flatten release workflow - merge create-release into build matrix
- fix(ci): remove secrets from job-level if, use continue-on-error instead
- fix(ci): include Cargo.lock in release commit and allow-dirty for publish
- fix(ci): allow same version in npm version step
- fix(ci): pin rust-toolchain to 1.88.0 in release workflow for target compatibility
- fix(ci): add tag push trigger back, prepare only for workflow_dispatch
- fix(ci): add rustfmt and clippy to rust-toolchain.toml

## v0.0.22 (2026-04-18)

Changes since v0.0.21:

- fix: remove dead code for non-Windows stubs and fix clippy single_match
- fix(lock): use compile-time cfg instead of runtime cfg! for platform-specific functions
- style: apply cargo fmt formatting
- fix: resolve cross-platform compilation errors for non-Windows
- chore: translate package description to English
- chore(ci): use rust-toolchain.toml for version pinning
- feat(ci): rewrite release workflow with workflow_dispatch and checksums
- chore: pin Rust toolchain to 1.88.0
- chore(release): add optimized release profile
- refactor: remove optimization 7 items + translate all Chinese to English
- fix(fs): preserve original IO error chain for retry detection
- feat(fs): add deletion retry with exponential backoff for locked files
- fix(lock): remove wmic call from PowerShell fallback
- feat(lock): use RestartManager API for file lock detection
- chore(deps): add windows-sys RestartManager feature
- refactor: code optimization - remove duplication, refactor modules, simplify functions
- feat(fs): integrate file lock check into remove command
- feat(fs): add file/directory lock detection

## v0.0.1 (2025-12-03)

- Cross-platform port lookup (Windows, Linux, macOS)
- Process termination with interactive selection
- Port listing with process details
- Colored terminal output
- Cargo and npm installation support

[unreleased]: https://github.com/Protagonistss/ziro/compare/v0.0.24...HEAD
[0.0.24]: https://github.com/Protagonistss/ziro/releases/tag/v0.0.24
[0.0.22]: https://github.com/Protagonistss/ziro/releases/tag/v0.0.22
[0.0.1]: https://github.com/Protagonistss/ziro/releases/tag/v0.0.1

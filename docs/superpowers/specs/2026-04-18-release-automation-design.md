# Release Automation Design

## Context

Current release process is fully manual: edit version in `Cargo.toml` + `package.json`, commit, create git tag, push tag to trigger CI. This is error-prone (version desync, stale CHANGELOG, silent publish failures). The goal is to automate the entire flow via a single GitHub Actions `workflow_dispatch` trigger.

## Design

### Trigger

Replace the current `v*` tag trigger with `workflow_dispatch` in `release.yml`:

```yaml
on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release (e.g. 0.0.22)'
        required: true
        type: string
```

Also keep `push tags: v*` as a secondary trigger for backwards compatibility.

### Jobs

**Job 1: `prepare`** (runs on `ubuntu-latest`)
1. Validate input version format (semver regex)
2. Check the version doesn't already exist as a git tag
3. Update `Cargo.toml` version via `sed`
4. Update `package.json` version via `npm version --no-git-tag-version`
5. Generate CHANGELOG from git log since last tag
6. Commit changes, create tag `v{version}`, push to repo
7. Output the tag for downstream jobs

**Job 2: `build`** (matrix, same 5 targets as current)
1. Build release binary with optimized `[profile.release]` (see below)
2. Package into zip
3. Generate SHA256 checksum file
4. Upload artifacts

**Job 3: `release`** (depends on `prepare` + `build`)
1. Download all artifacts
2. Create GitHub Release with auto-generated notes + CHANGELOG content
3. Upload all zip files + checksum files as release assets

**Job 4: `publish-crates`** (depends on `release`)
1. Run `cargo publish` — **remove `continue-on-error`**
2. Fail the workflow if publish fails

**Job 5: `publish-npm`** (depends on `release`)
1. Run `npm publish` — **remove `continue-on-error`**
2. Fail the workflow if publish fails

### Release Profile Optimization

Add to `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-Time Optimization
strip = true        # Strip debug symbols
panic = "abort"     # Smaller panic handler
codegen-units = 1   # Better optimization at cost of compile time
```

### rust-toolchain.toml

Create `rust-toolchain.toml` at project root to pin Rust version locally (matching CI's `1.88.0`):

```toml
[toolchain]
channel = "1.88.0"
```

### CHANGELOG Generation

Simple approach: extract commits since last tag using `git log --oneline v0.0.21..HEAD`, group by conventional commit prefix (`feat`, `fix`, `chore`, etc.), and append to `CHANGELOG.md`.

### Files to Modify

| File | Change |
|------|--------|
| `.github/workflows/release.yml` | Rewrite: add `workflow_dispatch`, `prepare` job, checksums, remove `continue-on-error` |
| `Cargo.toml` | Add `[profile.release]` section |
| `rust-toolchain.toml` | New file: pin Rust 1.88.0 |

### Verification

1. Push to dev branch, verify CI still passes (lint + test + version check)
2. Manually trigger `workflow_dispatch` with a test version
3. Verify: version synced in both files, tag created, Release published with checksums, crates.io/npm publish attempted

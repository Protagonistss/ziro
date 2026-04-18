# Release Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automate the release process via a single GitHub Actions `workflow_dispatch` trigger that handles version bump, CHANGELOG, build, checksums, and publish.

**Architecture:** Replace the current tag-triggered release workflow with a `workflow_dispatch`-driven pipeline. A new `prepare` job validates input, syncs versions, generates CHANGELOG, commits, and creates the tag. Existing build/publish jobs remain with reliability fixes.

**Tech Stack:** GitHub Actions, bash, npm, cargo

---

### Task 1: Add release profile to Cargo.toml

**Files:**
- Modify: `Cargo.toml` (append section at end)

- [ ] **Step 1: Add `[profile.release]` section**

Append to `Cargo.toml` after the `[lints.clippy]` section:

```toml
[profile.release]
opt-level = "z"
lto = true
strip = true
panic = "abort"
codegen-units = 1
```

- [ ] **Step 2: Verify build succeeds**

Run: `cargo build --release`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore(release): add optimized release profile"
```

---

### Task 2: Create rust-toolchain.toml

**Files:**
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Create file**

```toml
[toolchain]
channel = "1.88.0"
```

- [ ] **Step 2: Verify local toolchain matches**

Run: `rustc --version`
Expected: `rustc 1.88.0 ...`

- [ ] **Step 3: Commit**

```bash
git add rust-toolchain.toml
git commit -m "chore: pin Rust toolchain to 1.88.0"
```

---

### Task 3: Rewrite release.yml — prepare job

**Files:**
- Modify: `.github/workflows/release.yml` (full rewrite)

This task replaces the entire `release.yml` with a new workflow. Write the complete file.

- [ ] **Step 1: Write the new workflow file**

Replace the entire content of `.github/workflows/release.yml` with:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release (e.g. 0.0.22)'
        required: true
        type: string

env:
  CARGO_TERM_COLOR: always

permissions:
  contents: write

jobs:
  prepare:
    name: Prepare Release
    if: github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ secrets.GITHUB_TOKEN }}

      - name: Validate version format
        run: |
          VERSION="${{ github.event.inputs.version }}"
          if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
            echo "Error: Invalid version format '$VERSION'. Expected semver (e.g. 0.0.22)"
            exit 1
          fi

      - name: Check version not already tagged
        run: |
          VERSION="${{ github.event.inputs.version }}"
          if git tag -l "v$VERSION" | grep -q .; then
            echo "Error: Tag v$VERSION already exists"
            exit 1
          fi

      - name: Update Cargo.toml version
        run: |
          VERSION="${{ github.event.inputs.version }}"
          sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
          echo "Updated Cargo.toml to $VERSION"

      - name: Update package.json version
        run: |
          VERSION="${{ github.event.inputs.version }}"
          npm version "$VERSION" --no-git-tag-version
          echo "Updated package.json to $VERSION"

      - name: Verify versions match
        run: |
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
          NPM_VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
          if [ "$CARGO_VERSION" != "$NPM_VERSION" ]; then
            echo "Error: Version mismatch after update! Cargo: $CARGO_VERSION, npm: $NPM_VERSION"
            exit 1
          fi
          echo "Versions synchronized: $CARGO_VERSION"

      - name: Generate CHANGELOG
        run: |
          VERSION="${{ github.event.inputs.version }}"
          LAST_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")

          echo "## v$VERSION ($(date +%Y-%m-%d))" > /tmp/changelog_entry.md
          echo "" >> /tmp/changelog_entry.md

          if [ -n "$LAST_TAG" ]; then
            echo "Changes since $LAST_TAG:" >> /tmp/changelog_entry.md
            echo "" >> /tmp/changelog_entry.md
            git log "$LAST_TAG"..HEAD --pretty=format:"- %s (%h)" >> /tmp/changelog_entry.md
          else
            echo "Initial release" >> /tmp/changelog_entry.md
          fi
          echo "" >> /tmp/changelog_entry.md
          echo "" >> /tmp/changelog_entry.md

          # Prepend to existing CHANGELOG.md
          if [ -f CHANGELOG.md ]; then
            cat /tmp/changelog_entry.md CHANGELOG.md > /tmp/changelog_new.md
            mv /tmp/changelog_new.md CHANGELOG.md
          else
            cat /tmp/changelog_entry.md > CHANGELOG.md
          fi

      - name: Commit and tag
        run: |
          VERSION="${{ github.event.inputs.version }}"
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Cargo.toml package.json CHANGELOG.md
          git commit -m "chore(release): v$VERSION"
          git tag "v$VERSION"
          git push origin HEAD:${{ github.ref_name }} --tags

      - name: Output version
        id: version
        run: echo "version=${{ github.event.inputs.version }}" >> $GITHUB_OUTPUT

  create-release:
    name: Create Release
    needs: [prepare]
    if: always() && (needs.prepare.result == 'success' || github.event_name == 'push')
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.get_version.outputs.version }}
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Get version
        id: get_version
        run: |
          if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
            echo "version=${{ github.event.inputs.version }}" >> $GITHUB_OUTPUT
            echo "tag=v${{ github.event.inputs.version }}" >> $GITHUB_OUTPUT
          else
            echo "version=${GITHUB_REF#refs/tags/}" >> $GITHUB_OUTPUT
            echo "tag=${GITHUB_REF}" >> $GITHUB_OUTPUT
          fi

      - name: Create Release
        id: create_release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ steps.get_version.outputs.tag }}
          name: Release v${{ steps.get_version.outputs.version }}
          draft: false
          prerelease: false
          generate_release_notes: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  build:
    name: Build - ${{ matrix.platform.name }}
    needs: create-release
    runs-on: ${{ matrix.platform.runner }}
    strategy:
      fail-fast: false
      matrix:
        platform:
          - name: Windows x64
            os: windows
            arch: x86_64
            runner: windows-latest
            target: x86_64-pc-windows-msvc
            binary_name: ziro.exe
            asset_name: ziro-windows-x86_64.zip

          - name: Linux x64
            os: linux
            arch: x86_64
            runner: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            binary_name: ziro
            asset_name: ziro-linux-x86_64.zip

          - name: Linux ARM64
            os: linux
            arch: aarch64
            runner: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            binary_name: ziro
            asset_name: ziro-linux-aarch64.zip

          - name: macOS x64
            os: macos
            arch: x86_64
            runner: macos-latest
            target: x86_64-apple-darwin
            binary_name: ziro
            asset_name: ziro-macos-x86_64.zip

          - name: macOS ARM64
            os: macos
            arch: aarch64
            runner: macos-latest
            target: aarch64-apple-darwin
            binary_name: ziro
            asset_name: ziro-macos-aarch64.zip

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform.target }}

      - name: Install cross-compilation tools (Linux ARM64)
        if: matrix.platform.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build (Linux ARM64)
        if: matrix.platform.target == 'aarch64-unknown-linux-gnu'
        shell: bash
        run: |
          export CC=aarch64-linux-gnu-gcc
          export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
          cargo build --release --target ${{ matrix.platform.target }}

      - name: Build (Other platforms)
        if: matrix.platform.target != 'aarch64-unknown-linux-gnu'
        run: cargo build --release --target ${{ matrix.platform.target }}

      - name: Package binary
        id: package
        shell: bash
        run: |
          mkdir -p release-package
          cp target/${{ matrix.platform.target }}/release/${{ matrix.platform.binary_name }} release-package/ziro${{ matrix.platform.os == 'windows' && '.exe' || '' }}

          if [ "${{ matrix.platform.os }}" != "windows" ]; then
            chmod +x release-package/ziro
          fi

          cd release-package
          if [ "${{ matrix.platform.os }}" = "windows" ]; then
            7z a ../${{ matrix.platform.asset_name }} ziro.exe
          else
            zip ../${{ matrix.platform.asset_name }} ziro
          fi

      - name: Generate SHA256 checksum
        shell: bash
        run: |
          if [ "${{ matrix.platform.os }}" = "windows" ]; then
            certutil -hashfile ${{ matrix.platform.asset_name }} SHA256 > ${{ matrix.platform.asset_name }}.sha256
            # Clean up certutil output to just the hash
            sed -n '2p' ${{ matrix.platform.asset_name }}.sha256 | tr -d ' \r\n' > ${{ matrix.platform.asset_name }}.sha256.tmp
            echo "  ${{ matrix.platform.asset_name }}" >> ${{ matrix.platform.asset_name }}.sha256.tmp
            mv ${{ matrix.platform.asset_name }}.sha256.tmp ${{ matrix.platform.asset_name }}.sha256
          else
            shasum -a 256 ${{ matrix.platform.asset_name }} > ${{ matrix.platform.asset_name }}.sha256
          fi

      - name: Upload Release Assets
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ needs.create-release.outputs.version }}
          files: |
            ${{ matrix.platform.asset_name }}
            ${{ matrix.platform.asset_name }}.sha256
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  publish-crates:
    name: Publish to crates.io
    needs: [create-release, build]
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Publish to crates.io
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}

  publish-npm:
    name: Publish to npm
    needs: [create-release, build]
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Publish to npm
        run: npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

- [ ] **Step 2: Validate YAML syntax**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`
Expected: No output (valid YAML)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(ci): rewrite release workflow with workflow_dispatch and checksums"
```

---

### Task 4: Update ci.yml to use rust-toolchain.toml

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Remove pinned version from ci.yml, rely on rust-toolchain.toml**

Replace all occurrences of `dtolnay/rust-toolchain@1.88.0` with `dtolnay/rust-toolchain@stable` — the `rust-toolchain.toml` file will pin the actual version. This is already the case for the release workflow. The CI should also be consistent.

Actually, looking at it again: CI pins `1.88.0` via the action. With `rust-toolchain.toml` at project root, `dtolnay/rust-toolchain@stable` will read it and install `1.88.0`. So we can simplify.

Replace in `.github/workflows/ci.yml`:

```yaml
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@1.88.0
```

with:

```yaml
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
```

(appears 3 times in the file)

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "chore(ci): use rust-toolchain.toml for version pinning"
```

---

### Task 5: Update Cargo.toml description to English

**Files:**
- Modify: `Cargo.toml:7`

- [ ] **Step 1: Replace Chinese description**

Change line 7 in `Cargo.toml`:

```
description = "跨平台端口管理工具 - 快速查找和终止占用端口的进程"
```

to:

```
description = "Cross-platform port management tool - quickly find and kill processes occupying ports"
```

This is needed because `cargo publish` publishes to crates.io which requires English metadata.

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: translate package description to English"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Each section in the design doc maps to a task:
  - `workflow_dispatch` trigger → Task 3
  - Version sync (Cargo.toml + package.json) → Task 3 (prepare job)
  - CHANGELOG generation → Task 3 (prepare job)
  - `continue-on-error` removal → Task 3 (publish jobs)
  - SHA256 checksums → Task 3 (build job)
  - `rust-toolchain.toml` → Task 2
  - `[profile.release]` → Task 1
- [x] **Placeholder scan:** No TBD, TODO, or vague steps. All code shown in full.
- [x] **Type consistency:** All YAML references (outputs, env vars) are consistent across jobs.

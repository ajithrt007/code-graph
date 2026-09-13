# CodeGraph auto-updates (Tauri updater + GitHub Releases)

The app checks `latest.json` on GitHub Releases, downloads signed updater
artifacts, verifies signatures with the embedded public key, installs, and
restarts. No custom update server. Currently shipped for
**windows-x64** and **macos-x64** only (see `release.yml` matrix).

## One-time setup (owner only)

### 1. Generate the signing key pair

On any trusted machine with the Tauri CLI available:

```sh
npm --prefix apps/desktop install
npx --prefix apps/desktop tauri signer generate -w ~/.tauri-codegraph.key
```

This prints the **public key** and writes the **private key** to the given
file. Never commit either output; the private key file must never leave
trusted machines except into the GitHub secret below.

### 2. Configure GitHub secrets

Repository → Settings → Secrets and variables → Actions → New secret:

| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | contents of the private key file (required — release builds fail without it) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | password given to `signer generate`, if any (optional) |

CI never prints these values; the workflow only tests that the private key
is non-empty before building.

### 3. Embed the public key

Put the printed public key into
`apps/desktop/src-tauri/tauri.conf.json` → `plugins.updater.pubkey`,
replacing `REPLACE_WITH_UPDATER_PUBLIC_KEY`. Without the real public key,
 installs reject every update at signature verification (safe, but useless).

## Release procedure

The app version is the single source of truth and MUST equal the release
tag (minus the `v`): `tauri.conf.json` `version`, `src-tauri/Cargo.toml`
`version`, and `apps/desktop/package.json` `version`. CI **and**
`generate-latest-json.py` both fail the release on mismatch — a mismatch
would make every install update-loop forever.

```sh
# 1. Bump the version in all three files, e.g. 3.0.3 -> 3.0.4, and commit.
# 2. Tag and push the tag explicitly (full refspec avoids any
#    branch/tag name ambiguity):
git tag v3.0.4
git push origin tag v3.0.4
```

GitHub Actions then: publishes RoslynBridge → stages resources → builds
Tauri (signed updater artifacts: `.nsis.zip`, `.app.tar.gz`, each with a
`.sig`) → creates the GitHub Release with `--generate-notes` → generates
`latest.json` (version + notes + per-platform signed URLs) → uploads it to
the release. The client discovers updates at:

```text
https://github.com/ajithrt007/code-graph/releases/latest/download/latest.json
```

Note: there is no `v3.0.1` branch (branches are `v1`/`v2`/`v3`), so tag
pushes are currently unambiguous — but always use `git push origin tag
<v>` anyway; if a same-named branch is ever created, plain `git push
origin vX` becomes ambiguous and no cleanup of existing branches is
needed or performed by this flow.

## How the client discovers updates

- `UpdateSection` (`apps/desktop/src/components/UpdateSection.tsx`) mounts
  with the projects screen at startup and runs one silent `check()`
  (20 s timeout, never installs). Availability renders unobtrusively;
  "Check for updates" re-checks manually.
- `check()` returns `null` when up to date, else `{ version, body }`
  (release notes come from the GitHub release body via `latest.json`).
- "Update now" → `downloadAndInstall()` with progress events → "Restart
  now" → `relaunch()`. On Windows the installer exits the app itself
  after a successful install; the Restart button covers macOS (and any
  fallback path).
- Required capabilities: `updater:default` (check/download/install) and
  `process:default` (restart) in `src-tauri/capabilities/default.json`.

## Testing

### Updater disabled / no key yet

With the placeholder public key, checks fail into the error state with a
readable message. No install can proceed — safe by default.

### Local end-to-end (two versions)

1. Set the real public key in `tauri.conf.json`.
2. Build + install version A (e.g. `3.0.4`): `npm --prefix apps/desktop run tauri:build`.
3. Bump to version B (`3.0.5`), tag `v3.0.5`, push the tag; wait for CI.
4. Open version A → projects screen should offer `3.0.5` → Update now →
   progress → Restart now → app relaunches as `3.0.5` (check the version
   line in the Update section).

### Faster iteration without a release

Point a dev build's `plugins.updater.endpoints` at a local `latest.json`
served over HTTP with a `file://`-adjacent `url`... in practice: draft a
real GitHub **pre-release** (`gh release create v3.0.5-x --prerelease`)
— `releases/latest` ignores pre-releases, so production clients are
unaffected while you test the full signed flow from CI artifacts.

## Limitations / platform notes

- Only `windows-x64` + `macos-x64` ship updater artifacts today; re-enable
  matrix entries in `release.yml` (and `ci.yml`) to extend coverage —
  `generate-latest-json.py` already maps all six platform keys.
- Windows: `install()` exits the app itself; macOS: explicit relaunch.
- `latest.json` is attached to the newest *published* release; keep
  releases published (not draft) or clients won't see them.
- First updater-capable release is the first tag built from this branch
  onward; older releases (≤ v3.0.3) carry no updater artifacts, so
  0.1.0-era installs must be replaced manually once.

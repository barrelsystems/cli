# barrel

A standalone CLI that registers a release with [Barrel](https://barrel.codes)
for a desktop-app artifact your own pipeline has already built, signed, and
hosted.

## Problem

Once you've built, signed, and hosted a release artifact through your own
existing pipeline (electron-builder, Tauri, Sparkle, or anything else),
Barrel has no way to know a release happened unless you tell it — there's no
wired path from "I just shipped a release" to Barrel's release-creation API.

## What it does

One real command: `barrel release create`. It computes the artifact's
checksum and calls Barrel's release-creation API with that checksum plus the
release's metadata — platform, architecture, format, version, channel,
artifact URL — nothing else.

It never signs anything, never writes an update manifest, and never uploads
or hosts anything. Your own pipeline stays the sole owner of building,
signing, and hosting; `barrel` only ever makes one HTTP call to tell Barrel
that a release exists.

## Why this doesn't change your trust model

Your signing-key custody line stays exactly where it already is. Barrel's
servers never hold a signing key and never host a binary or a manifest —
`barrel` only sends metadata and a checksum of a file that stays wherever
you already put it.

## Not in scope (yet)

Signing orchestration and update-manifest generation were both considered
for this CLI and deliberately left out for now — not because they're not
useful, but because building either without Barrel also owning artifact
storage would mean a locally-generated file you'd still have to transport
yourself, which would need reworking the moment that changes. Your
pipeline's own framework tooling (electron-builder, Tauri, Sparkle) remains
the right place for both today.

## Install

No tagged binary release yet — build from source:

```
git clone https://github.com/barrelsystems/cli
cd cli
cargo install --path .
```

This installs the `barrel` binary via Cargo.

## Quickstart

```
export BARREL_CREDENTIAL=brl_app_...

barrel release create \
  --artifact ./dist/MyApp-1.2.3.dmg \
  --platform sparkle \
  --arch arm64 \
  --format dmg \
  --version 1.2.3 \
  --channel stable \
  --project-id proj_abc123 \
  --url https://cdn.example.com/MyApp-1.2.3.dmg
```

Add `--dry-run` to validate everything (config, credential, artifact,
checksum, payload shape) without calling the API. Add `--output json` to
get machine-readable output for scripting.

### `barrel.jsonc`

Commit a `barrel.jsonc` next to where you run `barrel` to avoid repeating
flags that don't change release to release:

```jsonc
{
  // Settings here are flag-overridable; the credential is never read
  // from this file -- it's env-var-only, via BARREL_CREDENTIAL.
  "projectId": "proj_abc123",
  "defaultChannel": "stable"
}
```

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Generic/unexpected error |
| `2` | Config or validation error |
| `3` | API/network error |

## License

[Apache-2.0](LICENSE). Third-party dependency licenses are listed in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

See [CONTRIBUTING.md](CONTRIBUTING.md) to contribute, and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards.

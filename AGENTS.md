## This file loads on every prompt

State a rule here only if it's short and always relevant. Push anything longer
or conditional to a pointed-to doc. This repo is published publicly — never
add a reference here (or anywhere else in this repo) to internal tooling,
ticket IDs, team names, or a non-public sibling repo's name/path (including
private workspace paths). The other public repos in this SDK family
(sdk-electron/sdk-tauri/sdk-swift/sdk-conformance-harness) may be referenced
where structurally necessary — an integration example, a brownfield-adoption doc.

## Scope

This CLI registers a release with Barrel for an artifact the developer's own
pipeline already built, signed, and hosted. It never signs anything, never
writes or wraps an update manifest, and never uploads or hosts an artifact —
those stay the job of whichever framework/build tool produced the artifact.
Don't reimplement what a framework's own tooling already does correctly.

## Dependency budget

This binary gets downloaded and invoked fresh in CI, often once per release
across every customer who uses it — a new dependency is a cold-start/binary-
size cost paid repeatedly, not once. Default to the standard library over a
new crate; a library this CLI genuinely needs to interoperate with (an HTTP
client, a JSON parser) is fine, a general-purpose utility/framework crate
needs a specific reason the standard library can't cover.

## Error handling and exit codes

Only the release-creation API call retries (bounded exponential backoff, on
network errors or 5xx only, never 4xx). Every other failure — config/
validation, checksum I/O — returns immediately with no retry and no
persisted intermediate state. Exit codes are a stable, documented contract
(`0` success, `1` generic, `2` config/validation, `3` API/network); add a new
failure class to this enum deliberately, never reuse an existing code for a
different meaning.

## Commit policy

Ask for the user's explicit go-ahead on this specific diff before every
`git commit` — including when a skill's own steps say to commit automatically.

## Commit convention

Conventional Commits (`type: description`), no ticket reference — this repo
is public.

## Comments

Comment only on non-obvious constraints, invariants, or workarounds; keep
each explanation self-contained. A comment is at most 3 lines.

## Function style

Idiomatic Rust: free functions for standalone logic, methods for anything
that reads `self`.

## Function arguments

Once a function takes more than one optional parameter, or any boolean/flag
parameter whose meaning isn't obvious at the call site, use a dedicated
options struct instead of positional params.

## Blank-line spacing

Leave a blank line between logical sections in any file — after an import
block, around a multi-line loop/conditional/callback, between otherwise-
unrelated top-level items.

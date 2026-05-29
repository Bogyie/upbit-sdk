# crates.io Publishing

This repository includes a GitHub Actions workflow for crates.io publish
readiness and controlled manual publishing of the `upbit-sdk` crate:

- workflow: `.github/workflows/publish-crates.yml`
- package path: `crates/upbit-sdk`
- requested publish action: `katyo/publish-crates@v2`
- crates.io token secret: `CARGO_REGISTRY_TOKEN`
- protected GitHub Environment for real publish: `crates-io`

Do not run `cargo publish`, create a release tag, create a GitHub Release, or
merge release branches unless the release is explicitly authorized.

## Safe Dry Runs

The workflow runs in dry-run mode for pull requests, pushes to
`integration/BOG-223-upbit-rust-sdk`, pushes to `main`, and manual
`workflow_dispatch` runs where `mode` is `dry-run`.

Dry-run jobs:

- run `cargo package -p upbit-sdk --allow-dirty --list`;
- call `katyo/publish-crates@v2` with `dry-run: true`;
- set `check-repo: false` so pull request or detached checkout contexts do not
  fail before the publish dry run;
- must not upload a package to crates.io.

The workflow prints an explicit dry-run confirmation before the action step.

## Real Publish Path

Real crates.io publishing is intentionally narrow. It can only happen when all
of these conditions are true:

- the workflow is started manually with `workflow_dispatch`;
- the `mode` input is `publish`;
- the selected workflow ref is a release tag;
- the repository has a `CARGO_REGISTRY_TOKEN` secret with crates.io publish
  permissions for `upbit-sdk`;
- the `crates-io` GitHub Environment approvals and protections, if configured,
  allow the job to continue.

The publish job validates the tag ref and token presence before invoking
`katyo/publish-crates@v2` with `dry-run: false` and `check-repo: true`.

Recommended release sequence:

1. Confirm the crate version, changelog, README, examples, and package include
   list are approved.
2. Run the workflow manually with `mode=dry-run` on the release candidate ref.
3. Create the release tag only after release approval.
4. Run the workflow manually from that tag with `mode=publish`.
5. Verify the crates.io package page and published metadata after the job
   completes.

## Secret Handling

Store the crates.io token only as a GitHub Actions secret named
`CARGO_REGISTRY_TOKEN`. Do not put the token in repository files, issue
comments, workflow logs, screenshots, or local command output.

Dry-run jobs pass the same input shape to the action, but the action documents
that the registry token is not used when dry-run mode is enabled.

## Failure And Rollback Notes

crates.io publishes are not mutable. A bad publish usually cannot be replaced
with the same version. If a publish fails or a bad package is released:

- inspect the GitHub Actions logs without exposing secrets;
- fix the repository state and rerun dry-run before any publish retry;
- publish a new semver version when a published artifact must be corrected;
- yank a version only when the release owner decides the published version
  should no longer be selected by new dependency resolution.

Tag deletion, release deletion, base-branch merges, and crates.io yanks all
require separate explicit authorization.

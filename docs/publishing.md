# crates.io Publishing

This repository includes a GitHub Actions workflow for crates.io publish
readiness and controlled publishing of the `upbit-sdk` crate after a GitHub
Release is published from `main`:

- workflow: `.github/workflows/publish-crates.yml`
- package path: `crates/upbit-sdk`
- requested publish action: reviewed pinned equivalent of
  `katyo/publish-crates@v2`
- reviewed action revision: `02cc2f1ad653fb25c7d1ff9eb590a8a50d06186b`
- crates.io token secret: `CARGO_REGISTRY_TOKEN`
- protected GitHub Environment for real publish: `crates-io`

Do not run `cargo publish`, create a release tag, create a GitHub Release, or
merge release branches unless the release is explicitly authorized.

## Safe Dry Runs

The workflow runs in dry-run mode for pull requests, pushes to
`integration/BOG-223-upbit-rust-sdk`, pushes to `main`, and manual
`workflow_dispatch` runs where `mode` is `dry-run`. GitHub Release events do
not run the dry-run job because they are reserved for the real publish path.
Pull request and push dry-runs only trigger when the workflow or package paths
listed in the workflow change.

Dry-run jobs:

- run `cargo package -p upbit-sdk --allow-dirty --list`;
- call the reviewed pinned `katyo/publish-crates@v2` equivalent with
  `dry-run: true`;
- set `check-repo: false` so pull request or detached checkout contexts do not
  fail before the publish dry run;
- do not pass `registry-token` to the third-party action;
- must not upload a package to crates.io.

The workflow prints an explicit dry-run confirmation before the action step.

## Real Publish Path

Real crates.io publishing is intentionally narrow. The expected path is:
merge the release commit to `main`, create a release tag from `main`, then
publish a GitHub Release for that tag. The publish job can only proceed when
all of these conditions are true:

- the workflow is triggered by a GitHub Release `published` event, or a
  separately authorized manual `workflow_dispatch` run with `mode=publish`;
- the selected workflow ref is a release tag;
- the release tag commit is already reachable from `origin/main`;
- for GitHub Release events, the event action is `published` and
  `release.target_commitish` resolves to `main`, `refs/heads/main`, the
  verified `origin/main` commit, or the verified tag commit;
- the repository has a `CARGO_REGISTRY_TOKEN` secret with crates.io publish
  permissions for `upbit-sdk`;
- the `crates-io` GitHub Environment approvals and protections, if configured,
  allow the job to continue.

The publish job validates the tag ref, main-branch ancestry, release event
shape, and token presence before invoking the reviewed pinned
`katyo/publish-crates@v2` equivalent with `dry-run: false` and
`check-repo: true`.

Recommended release sequence:

1. Confirm the crate version, changelog, README, examples, and package include
   list are approved.
2. Run the workflow manually with `mode=dry-run` on the release candidate ref.
3. Merge the approved release commit to `main`.
4. Create the release tag from `main`.
5. Publish the GitHub Release for that tag. The release `published` event starts
   the real publish job.
6. Verify the crates.io package page and published metadata after the job
   completes.

## Secret Handling

Store the crates.io token only as a GitHub Actions secret named
`CARGO_REGISTRY_TOKEN`. Do not put the token in repository files, issue
comments, workflow logs, screenshots, or local command output.

Dry-run jobs do not pass the registry token to the third-party action. The
token is only provided to the protected real publish job after its tag,
main-branch, release-event, and secret preconditions pass.

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

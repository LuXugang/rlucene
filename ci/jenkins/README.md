# Jenkins CI deployment

The `rlucene-ci` Pipeline defaults to `Rustify-All/rlucene:main`. Its definition
is [Jenkinsfile](Jenkinsfile) in this directory. The controller image, pinned
plugins, Compose service, and automatic job creation are versioned under
[deployment](deployment/README.md). Follow that guide to start a new instance
from a clone of this repository or configure it to build your fork.

The job checks out the same repository and branch that supplied its Jenkinsfile.
Each job has its own build cache and last-successful-commit state. The original
controller's disabled `legency` job and historical builds are instance data;
new installations create only `rlucene-ci`.

## Pull request test-light job

The `Jenkins PR test-light` GitHub Actions workflow handles pull request events.
It asks GitHub for the PR author's effective repository permission and starts
Jenkins only for `write` or `admin`. An external contributor's workflow exits
successfully without contacting Jenkins.

Authorized requests start the separate `rlucene-pr` Pipeline. The Pipeline
validates every parameter, fetches `refs/pull/<number>/head`, verifies that the
checked-out commit is the exact SHA reported by GitHub, and runs:

```sh
cargo test-light
```

The GitHub Actions run waits for Jenkins. Jenkins `SUCCESS` becomes a green
GitHub check; any other terminal result or timeout becomes a failed check. The
Actions summary links to the Jenkins build. A superseding update to the same PR
cancels the older Actions run and asks Jenkins to stop its obsolete build.
Every Jenkins PR build is protected from automatic deletion, including its log
and archived test output. The normal 200-build history limit applies only to a
run that fails before the Pipeline can mark it for retention.

PR code runs only on the exclusive `rlucene-pr` inbound agent. That container
does not mount Jenkins home or the Docker socket. The existing scheduled
`rlucene-ci` job remains on the built-in node and is unchanged.

## Jenkins prerequisites

- Jenkins 2.568.2 with Java 21, as pinned in the controller image.
- The plugin versions in `deployment/plugins.txt`, installed by the image.
- No Git credential is needed for the scheduled job's default public HTTPS
  checkout. The member-only PR job is intentionally fixed to
  `git@github.com:Rustify-All/rlucene.git` and requires the existing Jenkins SSH
  credential ID `github-ssh`; Jenkins uses it for the trusted definition checkout
  and the exact PR-ref checkout on the agent.
- Rust 1.98.0 with `rustfmt`, `clippy`, and `cargo-nextest` 0.9.143 available
  through `/opt/cargo/bin`.
- The version-controlled controller image and Compose configuration under
  `ci/jenkins/deployment`. They install `eu-stack` and `gdb` and grant the
  minimum ptrace capability needed by the dedicated `eu-stack` executable. The
  image configures Cargo to use the rsproxy sparse registry. It also installs
  the Simple Theme plugin and applies the version-controlled classic Pipeline
  console theme from `ci/jenkins/deployment/init.groovy.d`.
- Outbound access to GitHub and the configured Rust package mirrors.

Never put credential values in a Jenkinsfile, build parameter, email,
repository file, or console log.

## Scheduling

Run the new job once using **Build Now**. That run installs the Pipeline's
schedule and options: every minute (`* * * * *`), no concurrent builds,
500 retained builds, and a 30-minute overall timeout. The schedule is defined
directly in `Jenkinsfile`. After a schedule change is pushed, the next build
loads the updated file and applies it without restarting the controller.
The former `RLUCENE_CRON` environment setting is no longer used.

## Checkout and caching

The generated job uses a single-branch refspec, no tags, a depth-one clone,
and clean checkout. Jenkins keeps the workspace Git metadata between builds, so the first build clones the
repository and later builds normally fetch only changes to the configured branch.

The most recent successfully tested SHA is stored at
`$JENKINS_HOME/ci-state/$JOB_NAME/last-successful-sha` (the default job retains
the original `/var/jenkins_home/ci-state/rlucene-ci/last-successful-sha` path).
If the current SHA is unchanged, Jenkins skips dependency and infrastructure preflight, but still
runs nextest and doctests.

The persistent Cargo target is
`$JENKINS_HOME/cargo-target/$JOB_NAME`. Do not run `cargo clean` on every
scheduled build. Before and after every build, Jenkins logs free space for
Jenkins home and `/tmp`, plus the target directory size.

Rust test temporary files are isolated per build under
`$WORKSPACE_TMP/rlucene-ci-build-tmp/$BUILD_NUMBER` by setting `TMPDIR`. The
Pipeline removes stale data under its dedicated temporary root before a build
and deletes the current build directory from `post { always { ... } }`, so the
cleanup also runs after test failures and timeouts. This is safe because the
Pipeline disables concurrent builds. Do not clean the controller's global
`/tmp` from a scheduled build.

## Tests, timeouts, and diagnostics

The main release test command is run with debug assertions enabled:

```sh
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true \
  cargo nextest run --release --profile ci --workspace
```

Jenkins sets `CARGO_TARGET_DIR` to the persistent
`/var/jenkins_home/cargo-target/rlucene-ci` directory. Builds must not run
`cargo clean` or otherwise remove this directory: Cargo keeps release
artifacts there and reuses them when the commit is unchanged or when source
dependencies are unchanged.

Because nextest does not run Rust doctests, Jenkins also runs:

```sh
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true \
  cargo test --release --workspace --doc -q
```

The release-only `ci` profile in `.config/nextest.toml` marks an individual test
as slow after 30 seconds. A slow status is a warning and does not fail the build
if the test eventually passes. At 30 seconds, the Pipeline also asks nextest to
report all running test process IDs, elapsed times, and captured output. It
writes system load, the process tree, per-thread `/proc` state, kernel wait
stacks, and a best-effort userspace backtrace from `eu-stack`, `gdb`, or
`pstack` to `nextest-diagnostics.log`. If no debugger is installed or Linux
ptrace policy blocks attachment, the nextest status, process tree, resource
usage, and readable `/proc` diagnostics are still preserved.

The diagnostics helper recognizes a nextest test process by the `--exact`
argument used for a test-harness invocation. Cargo, Git, and other child
processes used while nextest is resolving dependencies are not treated as slow
tests and never cause the helper to send `SIGUSR1` to nextest.

The deployed container configuration and its verification procedure are
documented in `ci/jenkins/deployment/README.md`.

An individual Jenkins release test is terminated and reported as `TIMEOUT`
after 60 seconds. Nextest first sends `SIGTERM`, waits for a 30-second grace
period, and then sends `SIGKILL` if necessary. Non-release runs keep the default
60-second slow threshold and 360-second timeout. The Pipeline adds a 20-minute
outer timeout for the complete nextest run, a 4-minute timeout for doctests, and
a 30-minute timeout for the complete build.

Jenkins archives `nextest.log`, `nextest-junit.xml`,
`nextest-diagnostics.log`, and `doctest.log`. When `RLUCENE_FAILURE_EMAIL` is
configured and SMTP is set up in Jenkins,
failure emails include the commit SHA, failure classification, compressed
console log, and the available diagnostic artifacts. Email is disabled by
default; logs and diagnostics are always archived. Failures are reported for
human investigation; this
repository no longer starts an automatic repair job.

## Jenkins root URL

Set the Jenkins root URL in Jenkins administration to a stable address that
users and email recipients can reach. Do not commit private network addresses
to the repository.

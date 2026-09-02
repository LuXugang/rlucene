# Recreate the Jenkins CI environment

This directory builds the Jenkins controller and creates the `rlucene-ci`
Pipeline automatically on its first startup. The Pipeline is
[`../Jenkinsfile`](../Jenkinsfile). A new installation needs Docker Engine with
Compose v2, network access to the configured package sources and Git repository,
and a clone containing this version of the CI files.

## Start from a fresh clone

From the repository root:

```sh
cd ci/jenkins/deployment
cp .env.example .env
# Edit .env if you need different ports, a fork, or different resources.
docker compose config --quiet
docker compose build jenkins
docker compose up -d jenkins
docker compose ps
```

Open `http://localhost:8080` (or your Docker host and configured port). Complete
Jenkins' normal first-run setup, using the initial administrator password from:

```sh
docker compose exec jenkins cat /var/jenkins_home/secrets/initialAdminPassword
```

The image already contains the plugins listed in `plugins.txt`; do not select an
additional suggested-plugin installation if you want to retain that version
set. Create your own administrator account and set the Jenkins URL to the
address your users can reach. The setup wizard, authentication, and CSRF
protection are retained.

The `rlucene-ci` job is created from `init.groovy.d/rlucene-job.groovy.override`.
Run **Build Now** once to start testing and install the two-minute schedule.
The first build also compiles the Rust project; later builds reuse the cache.
The repository and branch configured in `.env` must contain
`ci/jenkins/Jenkinsfile` before this first build. For an unmerged change, point
both settings at a published fork/branch containing the change.

The default public HTTPS repository needs no SSH key. Private repository
credentials must be created in your own Jenkins instance. Put only the
credential ID in `.env`; never put passwords, private keys, API tokens, Jenkins
home, workspaces, or build history in Git or the Docker build context.

## What is reproduced

| Component | Source |
| --- | --- |
| Jenkins 2.568.2 / Java 21 | Version and image digest in `Dockerfile` and `.env.example` |
| Linux amd64 runtime | Compose `platform`, matching the original build host |
| Rust toolchain and components | Repository-root `rust-toolchain.toml`, copied during the image build |
| cargo-nextest 0.9.143 | Exact version in `Dockerfile` |
| 98 Jenkins plugins and dependencies | `plugins.txt`, installed with `--latest=false` |
| Pipeline job, SCM branch/refspec, shallow clean checkout | `init.groovy.d/rlucene-job.groovy.override` |
| Schedule, 500-build retention, no concurrent builds, test commands, timeouts and failure handling | `../Jenkinsfile` |
| Slow-test diagnostics and classic console theme | `../capture-slow-test-diagnostics.sh` and `init.groovy.d/rlucene-console-theme.groovy.override` |
| Container resources, ports, timezone and persistent storage | `docker-compose.yml` plus your local `.env` |

These versions were read from the running controller on 2026-09-01. The
previous Dockerfile only installed Simple Theme explicitly; the other plugins
and the job existed only in Jenkins home. That made a fresh volume incomplete.

This reproduces the CI configuration and named tool/plugin versions. It does
not promise a byte-identical image or identical test outcomes: Debian packages
can receive updates, Rust dependency resolution is
not frozen (`Cargo.lock` is intentionally untracked for this library), and
tests use randomness and concurrent scheduling. For an immutable binary
snapshot, separately retain a built image in a registry by digest. The base image is pinned by digest; update that digest explicitly when
upgrading Jenkins.

## Local configuration

`.env` is ignored by Git and excluded from the image build context.
`Dockerfile.dockerignore` permits only the deployment sources and toolchain
file, so Rust sources, local credentials, and build artifacts are not sent to
the image builder.

| Setting | Meaning |
| --- | --- |
| `COMPOSE_PROJECT_NAME` | Names the stack and its volume; default example is `rlucene-jenkins` |
| `JENKINS_HTTP_PORT` / `JENKINS_AGENT_PORT` | Host ports; defaults are 8080 / 50000 |
| `JAVA_OPTS` | Java controller heap only; reduce it for a smaller development machine |
| `RLUCENE_REPOSITORY_URL` / `RLUCENE_BRANCH` | SCM source for a newly created job; default is public upstream `main` |
| `RLUCENE_GIT_CREDENTIALS_ID` | Optional existing Jenkins Git credential ID |
| `RLUCENE_JOB_NAME` / `RLUCENE_JOB_DISABLED` | Name and initial disabled state of the generated job |
| `RLUCENE_CRON` | Schedule; default `H/2 * * * *`, empty disables periodic builds |
| `RLUCENE_FAILURE_EMAIL` | Optional failure recipient; empty disables email |

The job initializer creates missing jobs only. Restarting the container does
not replace existing jobs, history, credentials, or security settings. To
change an existing job's repository/branch, edit its Pipeline-from-SCM settings
in Jenkins. Changing only those bootstrap environment values does not rewrite
an existing job. To apply environment changes used during builds (schedule or
email), recreate the container and run the job once.

For SSH checkout, create your own SSH credential and configure host-key
verification in Jenkins before building. HTTPS avoids this setup for a public
repository. For email, configure **Manage Jenkins → System → Extended E-mail
Notification**, including your own SMTP authentication, then set
`RLUCENE_FAILURE_EMAIL`. Mail settings and secrets are specific to your own
installation; logs and diagnostics are archived even when email is disabled.

The default mirrors match the existing CI's Cargo registry (`rsproxy`). The
`.env.example` file also shows overrides for official Debian and Rust sources.
All mirrors are build arguments; rebuild the image after changing them.

## Resources and storage

The inspected host runs Docker Engine 29.1.3 and Compose 2.40.3. It is a
dedicated Linux VM with 20 vCPUs, about 30 GiB RAM and 8 GiB swap. Compose intentionally has no CPU or memory quota. `-Xms2g -Xmx8g`
limits only the Jenkins JVM, not Cargo or test processes. Smaller hosts need
appropriate heap, test parallelism and timeout choices. Docker Desktop on an
ARM Mac can emulate the pinned amd64 platform, with different performance.

The stack stores Jenkins data in `<COMPOSE_PROJECT_NAME>_jenkins_home`.
Recreating the container retains that named volume. Keep the same project
name when updating an existing installation. Removing the volume loses the
instance's accounts, credentials, jobs and history.

The default job retains the existing cache and state paths:

- `/var/jenkins_home/cargo-target/rlucene-ci`
- `/var/jenkins_home/ci-state/rlucene-ci/last-successful-sha`

Different job names get separate paths. Scheduled builds never run
`cargo clean`. See [Pipeline behavior](../README.md) for temporary-file cleanup,
release testing, timeouts and diagnostic archives.

## Migrate the existing controller

Moving the Jenkinsfile requires a coordinated SCM configuration change. This
repository change alone does not update an already running Jenkins job.

1. Publish/merge the branch containing `ci/jenkins/Jenkinsfile` before making
   the running job read that path. Pause scheduling while coordinating the
   update and wait for the current build to finish.
2. Back up the current deployment files and Jenkins home. Keep the existing
   volume and record the current image ID so it remains available for rollback.
3. Use a clone of the repository for deployment. The build context is now the
   repository root, because it includes `rust-toolchain.toml`; copying only
   this directory to an unrelated folder is no longer sufficient.
4. In your local `.env`, set `COMPOSE_PROJECT_NAME=jenkins` to reuse the original
   `jenkins_jenkins_home` volume. Set the desired Git credential ID and failure
   email locally. The new default image name avoids replacing the previous
   `local-jenkins-rust:lts` image.
5. Validate and build the image, then recreate the service during the maintenance
   window. The initializer leaves the existing `rlucene-ci` job in place.
6. In that job's **Pipeline → Pipeline script from SCM**, set **Script Path** to
   `ci/jenkins/Jenkinsfile`. Retain the existing repository/credentials and set
   the branch to `*/main`. Add the single-branch refspec
   `+refs/heads/main:refs/remotes/origin/main`, **Clean before checkout** (including
   nested repositories), and **Advanced clone behaviours** with shallow clone,
   depth 1, no tags, and honor refspec. The Pipeline now uses `checkout(scm)`,
   so these SCM settings also control the build checkout.
7. Set the Jenkins root URL to its current reachable address. The inspected
   instance still had an old address in its system configuration.
8. Run a build and verify checkout, tests, diagnostics, archive links and optional
   email. Resume scheduling only after validation. Keep the old deployment and
   image until this succeeds.

For rollback, restore both the previous Pipeline script path and a repository
revision that still contains the root Jenkinsfile, together with the previous
image/deployment configuration. Do not delete the persistent volume.

## Verify a new deployment

```sh
docker compose exec jenkins sh -c 'java -version; rustc --version; cargo nextest --version'
docker compose exec jenkins /usr/sbin/getcap /usr/bin/eu-stack
docker compose exec jenkins cat /opt/cargo/config.toml
```

The stack-capture capability must be `/usr/bin/eu-stack cap_sys_ptrace=eip`.
Compose also retains `SYS_PTRACE` in the capability bounding set so the
non-root Jenkins user can capture test subprocess stacks. The JVM does not
receive effective ptrace permission itself.

Check that `rlucene-ci` uses `ci/jenkins/Jenkinsfile`, runs once successfully,
and retains its configuration after a container restart. Classic Console
Output should show concise stage headings; downloaded `consoleText` and
archived logs still retain full Pipeline metadata.

The `.override` suffix on initialization sources tells the official Jenkins
image to copy the latest script into Jenkins home on each start. Job creation
is intentionally idempotent. This uses Jenkins' documented
[initialization hooks](https://www.jenkins.io/doc/book/managing/groovy-hook-scripts/)
and [Docker plugin installation](https://github.com/jenkinsci/docker#preinstalling-plugins).

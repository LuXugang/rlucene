# Recreate the Jenkins CI environment

This directory builds the Jenkins controller and the isolated pull request
agent. It creates both the scheduled `rlucene-ci` Pipeline and the member-only
`rlucene-pr` Pipeline automatically on first startup. Their definitions are
[`../Jenkinsfile`](../Jenkinsfile) and [`../pr/Jenkinsfile`](../pr/Jenkinsfile).
A new installation needs Docker Engine with Compose v2, network access to the
configured package sources and Git repository, and a clone containing this
version of the CI files.

## Start from a fresh clone

From the repository root:

```sh
cd ci/jenkins/deployment
cp .env.example .env
# Edit .env if you need different ports, a fork, or different resources.
docker compose config --quiet
docker compose build jenkins rlucene-pr-agent
docker compose up -d
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
Run **Build Now** once to start testing and install the one-minute schedule.
The first build also compiles the Rust project; later builds reuse the cache.
The repository and branch configured in `.env` must contain
`ci/jenkins/Jenkinsfile` before this first build. For an unmerged change, point
both settings at a published fork/branch containing the change.

The `rlucene-pr` job and its exclusive agent are created by the three
`rlucene-pr-*.groovy.override` hooks. After the setup wizard has configured
Jenkins' private user database and Project Matrix Authorization Strategy,
restart the controller. Jenkins then creates the dedicated `rlucene-github`
service account and one API token. The plaintext is written once to:

```text
/var/jenkins_home/secrets/rlucene-pr-trigger-token
```

Store the account name as the GitHub Actions secret `JENKINS_PR_USER`, store
the file content as `JENKINS_PR_API_TOKEN`, then delete the plaintext file from
Jenkins home. Jenkins retains only the token hash. Set
`RLUCENE_PR_JOB_DISABLED=false` before the job is first created, or enable the
job in Jenkins after adding the GitHub secrets. Never print either secret in a
build log.

From an authenticated repository checkout, the handoff can be done without
printing the token:

```sh
printf %s rlucene-github | gh secret set JENKINS_PR_USER --repo Rustify-All/rlucene
docker compose exec -T jenkins \
  cat /var/jenkins_home/secrets/rlucene-pr-trigger-token \
  | gh secret set JENKINS_PR_API_TOKEN --repo Rustify-All/rlucene
docker compose exec jenkins \
  rm /var/jenkins_home/secrets/rlucene-pr-trigger-token
```

The default public HTTPS repository needs no SSH key. Private repository
credentials must be created in your own Jenkins instance. Put only the
credential ID in `.env`; never put passwords, private keys, API tokens, Jenkins
home, workspaces, or build history in Git or the Docker build context.

The `rlucene-pr` job is specific to `Rustify-All/rlucene`: its trusted Pipeline
SCM and PR checkout both use the Jenkins SSH credential ID `github-ssh`. Create
that credential before enabling the PR job. The private key is supplied only to
Git checkout and is not stored in the agent image or Compose configuration.

## What is reproduced

| Component | Source |
| --- | --- |
| Jenkins 2.568.3 / Java 21 | Version and image digest in `Dockerfile` and `.env.example` |
| Linux amd64 runtime | Compose `platform`, matching the original build host |
| Rust toolchain and components | Repository-root `rust-toolchain.toml`, copied during the image build |
| cargo-nextest 0.9.143 | Exact version in `Dockerfile` |
| 98 Jenkins plugins and dependencies | `plugins.txt`, installed with `--latest=false` |
| Pipeline job, SCM branch/refspec, shallow clean checkout | `init.groovy.d/rlucene-job.groovy.override` |
| Trusted-PR job, service account and exclusive inbound agent | `init.groovy.d/rlucene-pr-*.groovy.override` and `Dockerfile.agent` |
| Optional public read-only authorization | `init.groovy.d/rlucene-public-read-only.groovy.override` |
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
| `JENKINS_CONTAINER_NAME` | Controller container name; defaults to `jenkins` for existing operational commands |
| `JAVA_OPTS` | Java controller heap only; reduce it for a smaller development machine |
| `PLUGINS_FORCE_UPGRADE` | Non-empty tells the official image to replace manually upgraded plugins when the image pins a newer version |
| `TRY_UPGRADE_IF_NO_MARKER` | Non-empty allows replacing older plugins from installations that have no image-version marker |
| `RLUCENE_REPOSITORY_URL` / `RLUCENE_BRANCH` | SCM source for a newly created job; default is public upstream `main` |
| `RLUCENE_GIT_CREDENTIALS_ID` | Optional existing Jenkins Git credential ID |
| `RLUCENE_JOB_NAME` / `RLUCENE_JOB_DISABLED` | Name and initial disabled state of the generated job |
| `RLUCENE_PUBLIC_READ_ONLY` | Opt in to anonymous read-only access for the configured job; disabled by default |
| `RLUCENE_ADMIN_USERS` | Comma-separated Jenkins user IDs that retain full administration when public read-only access is enabled |
| `RLUCENE_FAILURE_EMAIL` | Optional failure recipient; empty disables email |
| `RLUCENE_PR_AGENT_NAME` / `RLUCENE_PR_AGENT_LABEL` | Exclusive inbound agent identity and label |
| `RLUCENE_PR_JOB_NAME` / `RLUCENE_PR_JOB_DISABLED` | PR job name and initial disabled state |
| `RLUCENE_PR_TRIGGER_USER` | Dedicated API-only user stored in `JENKINS_PR_USER` on GitHub |

The job initializer creates missing jobs only. Restarting the container does
not replace existing jobs, history, credentials, or security settings. To
change an existing job's repository/branch, edit its Pipeline-from-SCM settings
in Jenkins. Changing only those bootstrap environment values does not rewrite
an existing job. To apply failure-email environment changes used during builds,
recreate the container and run the job once. The schedule is defined in
`../Jenkinsfile`; a pushed schedule change is applied by the next build without
recreating the container.

## Optional public read-only access

The deployment pins Jenkins 2.568.3 and the fixed plugin versions from the
[2026-09-02 Jenkins security advisory](https://www.jenkins.io/security/advisory/2026-09-02/).
Do not publish a controller still running Jenkins 2.568.2 or the older plugin
set, including one created from a previously built local image.

After completing the setup wizard and creating the administrator accounts, set
these values in the deployment's untracked `.env` file:

```dotenv
RLUCENE_PUBLIC_READ_ONLY=true
RLUCENE_ADMIN_USERS=luxugang,noreply
```

Then recreate or restart the Jenkins container. The initializer changes the
authorization strategy only from Jenkins' normal logged-in-user strategy or an
existing Project Matrix Authorization Strategy. When changing from the normal
strategy, it waits until every listed administrator ID exists, preventing a
first-start configuration from locking out the administrator. An unrelated
custom authorization strategy is left unchanged.

The resulting anonymous access is deliberately narrow:

- global `Overall/Read` and `View/Read`, so Jenkins pages and views can open;
- `Job/Read` on only the jobs named by `RLUCENE_JOB_NAME` and
  `RLUCENE_PR_JOB_NAME`, so their builds, console logs and artifacts can be
  viewed;
- no build, cancel, configure, workspace, credential or administration grants.

Existing matrix grants are retained. While the option remains enabled, a
container restart restores these three anonymous grants if they were removed
in the UI. Disabling the option stops managing them but does not revoke grants
already saved in Jenkins home; revoke those explicitly in **Manage Jenkins →
Security** and the job's **Configure → Enable project-based security** section.

This configuration is an initialization hook, not part of the Pipeline. A Git
checkout or a run of `ci/jenkins/Jenkinsfile` cannot replace it. A fresh Jenkins
home can reproduce it from this deployment configuration; the running
controller still stores the effective authorization state in its persistent
`jenkins_home` volume.

The official Jenkins image normally preserves plugin files already present in
an existing Jenkins home. Keep both `PLUGINS_FORCE_UPGRADE=true` and
`TRY_UPGRADE_IF_NO_MARKER=true` during this upgrade so the versions pinned in
`plugins.txt` replace older installed versions, including installations made
by an image that did not write version markers. These values are interpreted as
enabled whenever they are non-empty, so use empty values rather than `false` to
disable them after the migration.

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

The PR agent uses separate workspace and Cargo target volumes. It receives only
its inbound-agent connection secret; it cannot read `jenkins_home`. Do not add
the controller home or Docker socket to this service. Recreating the agent keeps
its compilation cache, while the Pipeline cleans the checked-out workspace and
per-build temporary directory after every result.

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

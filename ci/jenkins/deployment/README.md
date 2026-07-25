# Jenkins controller container

These files are the version-controlled backup of the Jenkins controller
deployment. The deployed copies live together in `/home/xugang/jenkins` on the
Jenkins VM. Jenkins configuration, credentials, job history, and workspaces
remain in the external `jenkins_home` named volume and are not stored here.
The image uses the rsproxy sparse registry for Cargo dependencies.

No credential value or private key may be added to these files.

## Deploy

Copy `Dockerfile` and `docker-compose.yml` to `/home/xugang/jenkins`, then run:

```sh
cd /home/xugang/jenkins
docker compose config --quiet
docker compose build jenkins
docker compose up -d --force-recreate jenkins
docker compose ps
```

Back up the currently deployed files before replacing them so configuration
rollback does not require rebuilding the previous revision from memory. The
named volume is retained when the container is recreated.

## Slow-test stack capture

The image installs `eu-stack` from `elfutils` and `gdb`. Jenkins runs as the
non-root `jenkins` user, so `/usr/bin/eu-stack` has the file capability
`cap_sys_ptrace=eip`; the Compose service adds `SYS_PTRACE` to the container
capability bounding set. This limits effective ptrace permission to the
dedicated stack-capture executable instead of granting it to the Jenkins Java
process.

Verify the deployed tools and capability with:

```sh
docker exec jenkins eu-stack --version
docker exec jenkins gdb --version
docker exec jenkins /usr/sbin/getcap /usr/bin/eu-stack
```

Expected capability output:

```text
/usr/bin/eu-stack cap_sys_ptrace=eip
```

Do not remove either the file capability or Compose `SYS_PTRACE`: both are
required for `ci/jenkins/capture-slow-test-diagnostics.sh` to attach to test
processes while Jenkins continues to run as a non-root user.

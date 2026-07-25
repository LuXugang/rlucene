FROM docker.m.daocloud.io/jenkins/jenkins:lts

USER root

RUN sed -i 's#http://deb.debian.org/debian#https://mirrors.tuna.tsinghua.edu.cn/debian#g' /etc/apt/sources.list.d/debian.sources \
    && sed -i 's#http://deb.debian.org/debian-security#https://mirrors.tuna.tsinghua.edu.cn/debian-security#g' /etc/apt/sources.list.d/debian.sources \
    && apt-get update \
    && apt-get install -y \
        curl \
        git \
        build-essential \
        pkg-config \
        libssl-dev \
        openssh-client \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_DIST_SERVER=https://mirrors.tuna.tsinghua.edu.cn/rustup
ENV RUSTUP_UPDATE_ROOT=https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH

RUN mkdir -p /opt/rustup /opt/cargo \
    && chown -R jenkins:jenkins /opt/rustup /opt/cargo

USER jenkins

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none

RUN mkdir -p /opt/cargo && cat > /opt/cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "ustc"

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
EOF

USER root
RUN /opt/cargo/bin/rustup toolchain install stable \
    && /opt/cargo/bin/cargo install cargo-nextest --locked \
    && chown -R jenkins:jenkins /opt/cargo /opt/rustup

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        elfutils \
        gdb \
        libcap2-bin \
    && setcap cap_sys_ptrace=eip /usr/bin/eu-stack \
    && rm -rf /var/lib/apt/lists/*

USER jenkins

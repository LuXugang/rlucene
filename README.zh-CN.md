# RLucene

<a href="README.md"><kbd>English</kbd></a>
<a href="README.zh-CN.md"><kbd>简体中文</kbd></a>

RLucene 是高性能、功能完善的搜索引擎库
[Apache Lucene](https://lucene.apache.org/) 的 Rust 移植版本。

项目的目标是尽可能贴近 Apache Lucene，在架构、行为、语义和日常使用方式上接近
100% 对齐。如果你以前使用过 Lucene，RLucene 用起来应该会很熟悉。文档、字段、
分析器、索引写入器、读取器、搜索器和查询等核心概念及工作流程都保持一致。

由于 Rust 和 Java 在类型系统、所有权和错误处理等方面存在差异，少量细微调整
不可避免。

RLucene 目前基于 Apache Lucene 10.1。待当前移植版本功能完善并达到稳定状态后，
项目将持续与 Apache Lucene 最新版本的前一个版本保持对齐。

## 项目状态

RLucene 还在积极开发中。最常用的功能已经实现，不过距离完整稳定还有很多移植、
打磨和测试工作要做。现阶段遇到一些不完善的地方也很正常，我们正在一点一点补齐。

有关 Lucene 基本概念和使用方式的介绍，请参考
[Apache Lucene 文档](https://lucene.apache.org/core/documentation.html)。

## 持续集成

RLucene 使用 Jenkins 进行持续集成。可以在
[jenkins.amazingkoala.com.cn](https://jenkins.amazingkoala.com.cn/) 查看构建状态和测试结果。

## 开发

项目使用的 Rust 工具链固定在 [`rust-toolchain.toml`](rust-toolchain.toml) 中。
如果使用 `rustup`，进入项目目录后会自动选择正确的工具链和组件。

| 命令 | 作用 |
| --- | --- |
| `cargo tidy` | 检查许可证头、应用 Cargo 自动修复、格式化代码，并对所有目标和 feature 运行 Clippy。你需要解决所有的警告和错误。 |
| `cargo test-light` | 运行与 `cargo test` 相同的库测试集合，但会启用轻量模式、复用共享只读测试数据并减少测试调试信息，因此执行速度更快；部分耗时测试在该模式下会使用较轻的工作量。 |
| `cargo commit` | 先运行 `cargo tidy`，任何未解决的警告或错误都会导致命令失败；随后确认工作区干净，并执行 `cargo test-light`。请在本地提交完成后、提交 PR 前使用；它不会创建 Git commit。 |
| `cargo nextest-run` | 使用第三方测试运行器 nextest 执行测试。 |

### cargo-nextest

使用 `cargo nextest-run` 前，需要先安装
[cargo-nextest](https://nexte.st/)。这项安装只需执行一次：

```shell
cargo install cargo-nextest --locked
```

安装完成后即可运行：

```shell
cargo nextest-run
```

其他项目自定义命令不依赖 cargo-nextest。

## 许可证

RLucene 使用 [Apache License 2.0](LICENSE) 许可证。

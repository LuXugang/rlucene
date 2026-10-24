# Vector query metric definitions

Reference: frozen luceneutil `f77ba1c41c181b9361a7ab5d48fa83b4a367f2e9`, especially
`src/main/knn/KnnGraphTester.java` and `src/python/knnPerfTest.py`. These independent
Rust/Java 10.1 drivers adapt the core metrics and add the checks below.

| Metric | Definition | Origin |
|---|---|---|
| Mean ms/query | Sum of complete-sweep elapsed time / completed queries | luceneutil latency concept |
| Serial QPS | Completed queries / sweep elapsed seconds, concurrency=1 | luceneutil log metric |
| CPU ms/query | Sum of sweep process CPU deltas / completed queries | Adapted netCPU |
| Average CPU cores | Process CPU time / elapsed time | Adapted avgCpuCount |
| Recall@100 | Mean returned-ID / groundtruth-Top100 intersection fraction | luceneutil recall concept |
| Visited | Independent leaf nearest-vector entry's totalHits value per query | luceneutil core metric |
| P50/P95/P99 | Linear interpolation at (N-1)*p over all per-request ns in a batch | Extension |
| Peak RSS MiB | macOS wait4.ru_maxrss / 1048576 for the complete batch process | Extension |
| Ordered IDs / f32 bits | Exact full output equality across engines and query/leaf paths | Extension |

Timing includes query vector read/decode, query construction, rewrite/KNN/search
and TopDocs assignment/retention. Index open, warmup, result checking, checksum,
groundtruth/visited processing and output are outside formal timing. Complete
sweep elapsed includes loop/timer bookkeeping; percentiles use per-request ns.

Rust process CPU uses the existing standalone getrusage(RUSAGE_SELF) measurement;
Java uses OperatingSystemMXBean.getProcessCpuTime. Both include user/kernel and
background process work, unlike the original luceneutil ThreadDetails mechanism.
The small FFI block is retained only in the measurement executable, not added to
the production library. Process CPU metrics do not gate runs or remove samples.

Peak RSS covers index open, warmup, formal sweeps, retained results, mappings and
runtime until exit. It is neither precise heap use nor a peak isolated to formal
query windows. It differs from vector RAM estimates and the Rust flush budget.

The headline compares paired complete-sweep elapsed ratios. Per-query changes use
the ratio of each engine's median over the 12 sweeps. Quality must be evaluated
alongside latency. Building/commit/merge, concurrent QPS, cold-cache latency,
filtered and quantized queries are outside this preset's measured scope.

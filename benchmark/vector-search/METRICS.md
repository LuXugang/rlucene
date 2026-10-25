# Vector metric definitions

## Full preset protocol v2

The preset runner in `reproduction/` follows fixed luceneutil
`c7eeab0ceb661dae8cf4171ceed7761689acc0e0` metric definitions. It writes the original
29 columns in their original order, preceded by engine, batch and thread-CPU
validity. It adapts the workload to Java10.1 and the serial published presets;
it is not the original parallel/quantized/rerank grid.

| Metric group | Definition |
|---|---|
| `index(s)` / `index_docs/s` | Time from directory/source/writer setup through the document loop and commit; truncate nanoseconds to milliseconds as in the frozen method; separately reported from the complete lifecycle |
| `merge(s)` | Explicit initial writer merge-scheduler wait; millisecond-truncated for this official column |
| `force_merge(s)` | Entire APPEND writer open, forceMerge request, scheduler wait and close lifecycle |
| Full construction total | Directory/reader/writer setup, input/decode/add, commit, wait, initial close, forceMerge lifecycle and final directory close; excludes post-build validation/index hashing |
| Input/decode/add | Combined ordinary document creation/add loop, including automatically triggered flush/merge work; these components are not separately instrumented |
| `latency(ms)` | Complete RUN elapsed, with each sweep truncated to milliseconds, divided by all completed requests |
| `netCPU` / `avgCpuCount` | All-live-thread snapshot CPU deltas, with frozen luceneutil conversion, per request / divided by wall time; N/A when snapshot thread sets change or counters are unavailable |
| Mean ms / QPS | Full RUN nanoseconds divided by request count / its reciprocal; reported at full precision separately from the official truncated field |
| P50 / P95 / P99 | Linear `(N-1)*p` interpolation over **separate LATENCY passes**, not a timer added to the throughput pass |
| Process CPU | All process user/kernel CPU per request and process CPU / wall, separately from thread snapshot CPU |
| `recall` | Mean intersection of returned stable IDs with graph-independent exact Top100 IDs |
| `visited` | Integer mean of all query visited counts, matched to an independent leaf nearest-vector request; full precision distribution is an extension |
| RSS | Process-exit macOS `wait4.ru_maxrss`, bytes converted to MiB; covers initialization, warmup, checking and timed requests |
| `vec_RAM(MB)` / `vec_disk(MB)` | `documents × dimensions × 4 / 1048576`, a logical payload formula, **not actual Rust heap layout or process RSS** |
| `index_size(MB)` | Actual index files summed /1048576, excluding `write.lock` |

Both engines query one identical fixed index. Each of two fresh batches includes
>=60s actual warmup per engine and12 balanced AB/BA rounds over the complete
query set. `fanout` adds to search K; the returned TopK remains100. No filter,
quantization or rerank is enabled. Precise preset deviations from upstream are
stored in each preset and the reproduction guide. Wiki uses all2,641 pinned text
queries, which differs from upstream's10,000-query configuration.

The structural compatibility group strictly checks every vector bit, stored-ID
permutation, doc-ord mapping, canonical HNSW traversal and integrity via both
readers. Cross-writer graph differences and cross-engine score bits, ordered IDs,
hit sets, per-query Recall and visited are distinct results. Wiki's `report`
policy records all query differences without relabelling them equal; SIFT retains
strict equality by default. Same-index graph digests must match between readers;
random metadata file SHA equality between newly built indices is not required.

Thread-CPU-invalid rounds, regressions and partial work remain in raw receipts.
Reporting never retries, filters or replaces samples. Missing construction data
in query-only runs is N/A. A stopped run and an incomplete batch are not marked
as a completed standard comparison. Each stage validates source, driver, binary,
JDK/JAR, input and index identities. Compilation is setup, never graph-build time.

## Original SIFT query protocol v1

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

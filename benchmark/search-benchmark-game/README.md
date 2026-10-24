# Wikipedia: Search Benchmark, the Game

Based on the [Tantivy/Quickwit benchmark method](https://github.com/quickwit-oss/search-benchmark-game/tree/a7c75473e91746280c5f01e69bf594ece5fca560),
adapted to public RLucene and Apache Lucene 10.1 APIs. The pinned 962-query set,
tags and original corpus transform are unchanged. Upstream files and hashes are
in `upstream.json`; their MIT license is in `UPSTREAM-LICENSE`.

## Prepare or reuse data

```sh
python3 -B benchmark/datasets/prepare.py wiki
# To reuse the exact source archive instead of downloading:
python3 -B benchmark/datasets/prepare.py wiki --archive /path/to/wiki-articles.json.bz2
python3 -B benchmark/run.py search-benchmark-game build
python3 -B benchmark/run.py search-benchmark-game prepare-index --corpus benchmark/data/wiki/corpus.json
```

The transformed JSONL has 5,032,104 documents / 8,327,453,114 bytes. Both raw
archive and transformed output are SHA-256 verified. Preparation requires space
for the compressed corpus, transformed corpus, index and build files.
`prepare-index` uses the Rust driver's existing index command: StandardAnalyzer,
stored `id`, `text` positions/frequencies/norms, numeric `sort_field`, 1024 MiB
RAM buffer, force merge to one segment. It refuses an existing output directory.

An exact byte copy of the published index can instead be provided by a separate
data artifact. `datasets/wiki/published-index.json` pins that artifact. Its
download URL is not yet published. Rebuilding creates a separately identified
index: the runner records `published_index_match=false`; use
`--require-published-index` when reproducing the exact physical input of the
published report. Both engines always read the same physical index.

## Run

```sh
python3 -B benchmark/run.py search-benchmark-game smoke --index /path/to/wiki-index
python3 -B benchmark/run.py search-benchmark-game run --index /path/to/wiki-index
# Optional subset; explicitly recorded as a nonstandard full run:
python3 -B benchmark/run.py search-benchmark-game run --index /path/to/wiki-index --modes COUNT
```

`smoke` checks six representative query IDs in all five modes, 30 requests per
engine. `run` verifies all 4,810 full outputs per engine before timing. Equality
includes ordered stored IDs, internal docIDs, f32 score bits, count and relation.
The Rust query adapter intentionally supports only the pinned query grammar;
unsupported Lucene syntax is rejected.

## Protocol and measurement

`configs/wiki-five-modes.json` defines the protocol: COUNT exact; TOP_10/100/1000
with total-hit threshold=k; TOP_100_COUNT with Top100 and exact total count.
BM25(1.2, 0.75), query cache disabled, StandardAnalyzer with empty stopwords,
serial search, one segment, no deletions. Java uses normal JIT, the vector module,
native access, a 4 GiB heap and ParallelGC.

Two fresh-process batches; per engine/mode/batch >=60 seconds of actual mixed
requests, followed by 12 full rounds. Each query has six Rust-first and six
Java-first observations per batch. Query permutation uses seed 2026101900+batch.
All samples and slower queries are retained. No CPU load gating or automatic retry.

Timing uses the original macOS select/read backend: parent request write/flush
through parsing, search, normal short-response formatting/flush and parent read.
It includes IPC; it is not pure search-kernel time. Timed TopN returns an
acknowledgement, while exact modes return count. Full-hit retrieval and output
serialization belong to the separate correctness phase.

The headline is the median of paired complete-round elapsed ratios minus one.
Per-query output also includes paired ratios, median times and median differences.
Negative changes mean Rust takes less time. Tags overlap and must not be summed.
The protocol is reused from the completed 2026-10-24 comparison at Rust
`8d69a4784b6c85a80a7826802488c1442dab8127`; running this checkout produces new
identified results rather than relabelling those historical numbers.

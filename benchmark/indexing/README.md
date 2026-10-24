# Indexing Benchmark — in preparation

This directory corresponds to the homepage's luceneutil Indexing Benchmark card.
It is reserved for a reproducible public-API indexing method, not yet runnable.

Before enabling it, pin the upstream method/version and corpus, align analyzers,
fields, threads, RAM/flush policy, merge scheduler, codec and index validation.
Report the document write loop, commit, merge/force-merge and close separately,
alongside total elapsed, throughput, process CPU/RSS, index size and segment count.

Existing synthetic indexing correctness/optimization probes are not equivalent
to this full method and are not presented as its benchmark results. The vector
suite's `prepare-index` is setup rather than a timed indexing comparison.

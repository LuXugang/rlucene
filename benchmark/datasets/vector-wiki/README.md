# Pinned Wiki vector inputs

This directory supplies the source manifest, all2,641 queries, and unmodified
luceneutil generator sources for GloVe100/300, MiniLM384 and MPNet768.
Large source archives, weights and generated vectors are not included in Git.

`manifest.json` fixes upstream commit, source URLs, model revisions, source byte
counts and recorded complete-file SHA256 values. Those local complete-file
identities are not described as checksums published by the upstream author.
The original Stanford GloVe archive is required; a similarly named mirror ZIP
that has different dictionary contents is not interchangeable.

The model references are sentence-transformers/all-MiniLM-L6-v2 at
`1110a243fdf4706b3f48f1d95db1a4f5529b4d41` and
sentence-transformers/all-mpnet-base-v2 at
`e8c3b32edf5434bc2275fc9bab85f82640a19130`.
The saved inference environment is `requirements-inference.txt`. It includes
transitive dependencies; generation checks the actual complete package list.

Vector generation must follow token inference plus `WikiVectors` aggregation,
not encode whole documents with SentenceTransformer. Document selection skips
the Wiki header; token dictionary inference preserves its original inclusion.
Duplicates, zero vectors, float32 values and original document/query order are
kept. Norms, zeros and final bytes are independently checked during exhaustive
truth generation. A different device may create a new vector identity, requiring
new truth and explicit acceptance rather than silently replacing published inputs.

See [the reproduction guide](../../vector-search/REPRODUCING.md) for executable
download, generation, groundtruth and benchmark commands. `--download` is an
explicit operation. An alternate mirror is usable only when the full file bytes
match the pinned checksum; the runner never swaps a model revision or archive.

Upstream Java/Python sources retain their original notices and are supplied under
the repository's Apache2.0 license. They come from luceneutil commit
`c7eeab0ceb661dae8cf4171ceed7761689acc0e0`.

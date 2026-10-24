# SIFT1M inputs

`manifest.json` records the original TexMex source, mirror, archive SHA-256,
individual fvecs/ivecs hashes and converted raw hashes. The payload conversion
retains original little-endian bytes, removing only dimension headers. All files
are verified after conversion; the implementation requires no NumPy dependency.

`published-index.json` identifies the one-million-vector, single-segment Lucene101
index used by the existing query method. The vector suite can reuse it or build
a new index with an explicit setup receipt. Raw vectors, downloaded archives and
indexes belong in ignored `benchmark/data/sift1m/` or an external data directory.

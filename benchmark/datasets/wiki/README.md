# Wikipedia inputs

The download URL, original compressed/transformed corpus SHA-256 and geometry
are in `manifest.json`. Queries and the original transform come from
`quickwit-oss/search-benchmark-game` at
`a7c75473e91746280c5f01e69bf594ece5fca560`; the source manifest and
[MIT license](../../search-benchmark-game/UPSTREAM-LICENSE) are included.

`corpus_transform.py` is the unmodified upstream file. `prepare.py wiki` invokes
it with UTF-8 input/output and verifies the entire output hash. The result has
5,032,104 documents. The original byte-identical index is separately distributed;
`published-index.json` pins its 17 files. `prepare-index` can create a new index
with a separately recorded identity when that artifact is unavailable.

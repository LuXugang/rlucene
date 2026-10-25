"""Pinned input generation. Invoked only by explicit prepare-data/truth actions."""
import ast
import importlib.util
import json
import lzma
from pathlib import Path
import re
import shutil
import sys
from types import SimpleNamespace
import zipfile

from common import runtime as rt
import build

WIKI = rt.BENCHMARK / "datasets/vector-wiki"


def needed_sources(config):
    manifest = rt.load(WIKI / "manifest.json")
    model = "all-MiniLM-L6-v2" if config["dimensions"] == 384 else "all-mpnet-base-v2"
    return [row for row in manifest["files"] if row["path"].endswith(".lzma") or
            config["dimensions"] in (100, 300) and row["path"] == "downloads/glove.6B.zip" or
            config["dimensions"] in (384, 768) and row["path"].startswith("models/" + model + "/")]


def verify_source(path, expected):
    rt.require(path.stat().st_size == expected["bytes"] and rt.sha(path) == expected["sha256"],
               "Source identity mismatch: " + str(path))


def check_upstream():
    manifest = rt.load(WIKI / "manifest.json")
    for name, digest in manifest["upstream_files"].items():
        rt.require(rt.sha(WIKI / name) == digest, "Pinned generator changed: " + name)
    rt.require(rt.sha(WIKI / "queries.txt") == manifest["queries"]["sha256"], "Query text changed")


def stage(args, config, prefix):
    """Use a separate retained receipt for each preparation attempt."""
    from runner import execute
    root = args.data / "preparation" / (prefix + "-" + __import__("time").strftime("%Y%m%d-%H%M%S"))
    (root / "raw").mkdir(parents=True)
    run = {"config": config, "status": "RUNNING", "commands": [], "performance_measurement": False}
    rt.save(root / "RUN.json", run)
    return root, run, execute


def prepare(args, config):
    from runner import inputs
    args.data.mkdir(parents=True, exist_ok=True)
    if (args.data / "DATASET.json").exists():
        observed = inputs(args.data, config, accept_new=args.accept_new_inputs)
        saved = rt.load(args.data / "DATASET.json")
        rt.require(saved["config"] == config and saved["vectors"] == observed, "Existing dataset receipt differs")
        print("DATASET_READY", args.data)
        return
    rt.require(not any((args.data / name).exists() for name in ("base.raw", "query.raw")),
               "Input outputs already exist without a completed receipt; retain and inspect them, or select a new --data")
    sources = args.sources or rt.BENCHMARK / "data/vector-sources"
    sources.mkdir(parents=True, exist_ok=True)
    if config["id"] == "sift1m":
        archive = sources / "sift.tar.gz"
        manifest = rt.load(rt.BENCHMARK / "datasets/sift1m/manifest.json")
        if not archive.exists():
            rt.require(args.download, "Missing SIFT archive; use --sources or explicitly --download")
            rt.download(manifest["mirror"], archive, manifest["archive_sha256"])
        spec = importlib.util.spec_from_file_location("pinned_prepare", rt.BENCHMARK / "datasets/prepare.py")
        preparer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(preparer)
        preparer.sift(SimpleNamespace(archive=archive, destination=args.data))
        observed = inputs(args.data, config, accept_new=args.accept_new_inputs)
        rt.save(args.data / "DATASET.json", {"complete": True, "config": config, "vectors": observed,
                "source_archive_sha256": rt.sha(archive), "conversion": "remove row headers; preserve all payload bits"})
        return
    check_upstream()
    selected = needed_sources(config)
    for row in selected:
        path = sources / row["path"]
        if not path.exists():
            rt.require(args.download, "Missing source; supply --sources or explicitly --download: " + str(path))
            rt.download(row["url"], path, row["sha256"])
        verify_source(path, row)
    receipt = build.programs(args.java_home)
    root, run, execute = stage(args, config, "prepare")
    try:
        text = args.data / "wiki-documents.txt"
        archive = next(sources / row["path"] for row in selected if row["path"].endswith(".lzma"))
        with lzma.open(archive, "rb") as source, text.open("xb") as output:
            header = source.readline()
            rt.require(header.startswith(b"FIELDS_HEADER_INDICATOR"), "Unexpected original Wiki header")
            for _ in range(config["documents"]):
                line = source.readline()
                rt.require(bool(line), "Truncated Wiki corpus")
                output.write(line)
        queries = args.data / "queries.txt"
        shutil.copyfile(WIKI / "queries.txt", queries)
        dim = config["dimensions"]
        if dim in (100, 300):
            dictionary = args.data / f"glove.6B.{dim}d.txt"
            with zipfile.ZipFile(sources / "downloads/glove.6B.zip") as zipped:
                with zipped.open(dictionary.name) as source, dictionary.open("xb") as target:
                    shutil.copyfileobj(source, target, 8 * 1024 * 1024)
            generator = [str(dictionary)]
            run["dictionary"] = {"bytes": dictionary.stat().st_size, "sha256": rt.sha(dictionary)}
        else:
            rt.require(args.inference_python is not None, "MiniLM/MPNet requires --inference-python; see requirements-inference.txt")
            execute([args.inference_python, "-m", "pip", "freeze", "--all"], root, "inference-runtime", run)
            frozen = (root / "raw/inference-runtime.out").read_text().splitlines()
            expected = (WIKI / "requirements-inference.txt").read_text().splitlines()
            rt.require(sorted(frozen) == sorted(expected), "Inference environment differs from pinned requirements")
            script = WIKI / "upstream/infer_token_vectors.py"
            tree = ast.parse(script.read_text())
            expression = next(ast.literal_eval(n.value.args[0]) for n in tree.body if isinstance(n, ast.Assign) and
                              any(isinstance(t, ast.Name) and t.id == "TOKENIZE_RE" for t in n.targets))
            tokenize = re.compile(expression)
            prefix = args.data / "token-dictionary-prefix.txt"
            counts = {}
            with lzma.open(archive, "rt", encoding="utf-8") as source, prefix.open("x", encoding="utf-8") as output:
                for line in source:
                    if len(counts) > 400000:
                        break
                    output.write(line)
                    for token in tokenize.split(line[:-1]):
                        token = token.lower()
                        counts[token] = counts.get(token, 0) + 1
            expected_tokens = [token for token, count in counts.items() if count > 1]
            model = "all-MiniLM-L6-v2" if dim == 384 else "all-mpnet-base-v2"
            tokens, vectors = args.data / "tokens.txt", args.data / "token-vectors.raw"
            # Run the pinned inference script, including its original token order,
            # >400000 boundary, count>1 rule, batch256, and model.encode behavior.
            execute([args.inference_python, script, prefix, tokens, vectors, sources / "models" / model],
                    root, "infer-token-vectors", run)
            rt.require(tokens.read_bytes().decode("utf-8").split("\n")[:-1] == expected_tokens, "Token order differs from pinned inference")
            rt.require(vectors.stat().st_size == len(expected_tokens) * dim * 4, "Wrong token vector dimensions")
            run["inference"] = {"packages": frozen, "tokens_sha256": rt.sha(tokens), "vectors_sha256": rt.sha(vectors),
                                "prefix_sha256": rt.sha(prefix), "source_script_sha256": rt.sha(script), "model": model}
            generator = [str(tokens), str(vectors), str(dim)]
        for role, source in (("base", text), ("query", queries)):
            command = build.command(receipt, "java", "generate", [*generator, source, args.data / (role + ".raw")])
            # Locale affects the upstream dictionary's case conversion.
            command[1:1] = ["-Duser.language=en", "-Duser.country=US"]
            execute(command, root, "generate-" + role, run)
        observed = inputs(args.data, config, accept_new=args.accept_new_inputs)
        build.programs(args.java_home)
        for row in selected:
            verify_source(sources / row["path"], row)
        run["status"] = "COMPLETE"
        rt.save(args.data / "DATASET.json", {"complete": True, "config": config, "vectors": observed,
                "source_files": selected, "generator": receipt, "preparation_receipt": str(root / "RUN.json"),
                "document_text_sha256": rt.sha(text), "query_text_sha256": rt.sha(queries),
                "document_header_skipped": True, "duplicates_and_zero_vectors_retained": True,
                "normalization": "only the pinned WikiVectors/VectorDictionary method; no additional normalization"})
    except KeyboardInterrupt:
        run["status"] = "STOPPED_BY_USER"
        raise
    except BaseException as error:
        run.update(status="FAILED", error=repr(error))
        raise
    finally:
        rt.save(root / "RUN.json", run)


def truth(args, config):
    from runner import inputs
    observed = inputs(args.data, config, accept_new=args.accept_new_inputs)
    rt.require(not any((args.data / name).exists() for name in ("groundtruth.raw", "groundtruth.scores.raw", "GROUNDTRUTH.json")),
               "Groundtruth already exists; reuse its receipt or choose a new dataset directory. Never overwrite evidence.")
    receipt = build.programs(args.java_home)
    root, run, execute = stage(args, config, "truth")
    try:
        execute(build.command(receipt, "java", "truth", [args.data / "base.raw", args.data / "query.raw", args.data]),
                root, "exhaustive-groundtruth", run)
        lines = (root / "raw/exhaustive-groundtruth.out").read_text().splitlines()
        done = next(line.split("\t")[1:] for line in lines if line.startswith("EXACT_COMPLETE\t"))
        rt.require(list(map(int, done)) == [config["documents"], config["queries"], config["dimensions"], 3], "Incomplete exact truth")
        stats = {}
        for line in lines:
            if line.startswith("INPUT\t"):
                f = line.split("\t")
                rt.require(f[4] == observed["files"][f[1] + ".raw"]["sha256"], "Groundtruth input changed")
                stats[f[1]] = dict(zero_vectors=int(f[5]), squared_norm_min=float(f[6]), squared_norm_max=float(f[7]), zero_query_ordinals=json.loads(f[8]))
        files = dict(observed["files"])
        for name in ("groundtruth.raw", "groundtruth.scores.raw"):
            file = args.data / name
            rt.require(file.stat().st_size == config["queries"] * 100 * 4, "Truncated groundtruth")
            files[name] = dict(bytes=file.stat().st_size, sha256=rt.sha(file))
        rt.require(inputs(args.data, config, accept_new=args.accept_new_inputs) == observed, "Inputs changed")
        build.programs(args.java_home)
        rt.save(args.data / "GROUNDTRUTH.json", {"complete": True, "files": files, "inputs": stats,
                "documents": config["documents"], "dimensions": config["dimensions"], "queries": config["queries"],
                "similarity": config["similarity"], "graph_independent": True, "independent_full_sort_queries": 3,
                "scored_pairs": config["documents"] * config["queries"], "scoring": "Java10.1 normal JIT/VectorAPI, >=60s actual exhaustive warmup",
                "tie_break": "score descending, document ID ascending", "preparation_receipt": str(root / "RUN.json")})
        run["status"] = "COMPLETE"
    except KeyboardInterrupt:
        run["status"] = "STOPPED_BY_USER"
        raise
    except BaseException as error:
        run.update(status="FAILED", error=repr(error))
        raise
    finally:
        rt.save(root / "RUN.json", run)

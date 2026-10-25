"""Explicit stages for the published SIFT/Wiki vector workloads, macOS only."""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

from common import runtime as rt
import build
import metrics

SUITE = Path(__file__).resolve().parents[1]
PRESETS = SUITE / "presets"


def preset(name):
    rt.require(name in {p.stem for p in PRESETS.glob("*.json")}, "Unknown preset: " + name)
    config = rt.load(PRESETS / (name + ".json"))
    rt.require(config["top_k"] == 100 and config["batches"] == 2 and config["rounds"] == 12,
               "Unsupported sampling configuration")
    return config


@contextmanager
def environment(config):
    values = {"DIM": config["dimensions"], "NDOC": config["documents"], "NQ": config["queries"],
              "M": config["M"], "BEAM": config["beam_width"], "RAM_MIB": config["ram_buffer_MiB"],
              "SIMILARITY": config["similarity"]}
    saved = {}
    for key, value in values.items():
        key = "RLUCENE_BENCH_" + key
        saved[key] = os.environ.get(key)
        os.environ[key] = str(value)
    try:
        yield
    finally:
        for key, value in saved.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def inputs(data, config, truth=False, accept_new=False):
    result = {}
    for name in ["base.raw", "query.raw"] + (["groundtruth.raw"] if truth else []):
        path = data / name
        expected = config["input_files"][name]
        rt.require(path.stat().st_size == expected["bytes"], "Input shape mismatch: " + str(path))
        result[name] = {"bytes": path.stat().st_size, "sha256": rt.sha(path)}
    known = all(result[n]["sha256"] == config["input_files"][n]["sha256"] for n in ("base.raw", "query.raw"))
    rt.require(known or accept_new, "Generated vector bits differ from the published inputs. Inspect them; "
               "--accept-new-inputs explicitly selects a newly identified dataset and requires new groundtruth.")
    if truth:
        receipt = data / "GROUNDTRUTH.json"
        if receipt.exists():
            gt = rt.load(receipt)
            rt.require(gt["complete"] and gt["documents"] == config["documents"] and
                       gt["dimensions"] == config["dimensions"] and gt["queries"] == config["queries"] and
                       gt["similarity"] == config["similarity"], "Groundtruth configuration mismatch")
            rt.require(all(gt["files"][n] == result[n] for n in result), "Stale groundtruth for these vector bytes")
        else:
            rt.require(known and result["groundtruth.raw"] == config["input_files"]["groundtruth.raw"],
                       "Unknown groundtruth without a matching GROUNDTRUTH.json receipt")
    return {"files": result, "published_vector_bits_match": known}


def execute(command, path, label, run):
    """Each child is serial; retain failure/cancellation and its resource receipt."""
    out, err = path / "raw" / (label + ".out"), path / "raw" / (label + ".err")
    rt.require(not out.exists() and not err.exists(), "Raw result already exists: " + label)
    row = {"label": label, "command": list(map(str, command)), "start": time.time()}
    run["commands"].append(row)
    rt.save(path / "RUN.json", run)
    child = None
    with out.open("wb") as stdout, err.open("wb") as stderr:
        try:
            child = subprocess.Popen(row["command"], env=rt.environment(), stdout=stdout, stderr=stderr,
                                     start_new_session=True)
            _, status, usage = os.wait4(child.pid, 0)
            child.returncode = os.waitstatus_to_exitcode(status)
            row.update(exit=child.returncode, end=time.time(), peak_rss_bytes=usage.ru_maxrss,
                       user_cpu_s=usage.ru_utime, system_cpu_s=usage.ru_stime)
        except BaseException:
            if child is not None and child.returncode is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                row.update(exit=child.returncode, end=time.time(), stopped=True)
            raise
        finally:
            row.update(stdout_sha256=rt.sha(out), stderr_sha256=rt.sha(err))
            rt.save(path / "RUN.json", run)
    rt.require(row["exit"] == 0, "Command failed; all output retained: " + str(err))
    return row


def writers(args, path, run):
    config = run["config"]
    (path / "indices").mkdir()
    for batch, engine in ((1, "rust"), (1, "java"), (2, "java"), (2, "rust")):
        label = f"b{batch}-{engine}"
        index = path / "indices" / label
        print("BUILD_INDEX", label, flush=True)
        observation = execute(build.command(run["build"], engine, "writer", [index, args.data / "base.raw"]),
                              path, label + "-writer", run)
        lines = (path / "raw" / (label + "-writer.out")).read_text().splitlines()
        fields = next(line.split("\t")[1:] for line in lines if line.startswith("RESULT\t"))
        rt.require(len(fields) == 10, "Writer metrics protocol changed")
        keys = ["input_decode_add_loop_ns", "loop_plus_commit_ns", "merge_wait_ns", "force_merge_ns",
                "close_ns", "total_ns", "segments", "documents", "commit_ns", "upstream_index_ns"]
        row = dict(zip(keys, map(int, fields)))
        rt.require(row["segments"] == 1 and row["documents"] == config["documents"], "Wrong index shape")
        text = "\n".join(lines)
        expected = [f'maxConn={config["M"]}', f'beamWidth={config["beam_width"]}', "seed=42"]
        rt.require(all(s in text for s in expected), "Actual HNSW configuration differs from preset")
        if engine == "rust":
            rt.require(f'RAM_MiB={config["ram_buffer_MiB"]}' in text and "compound=false" in text,
                       "Wrong Rust writer RAM/compound setting")
        else:
            rt.require(f'ramBufferSizeMB={config["ram_buffer_MiB"]}.0' in text and "useCompoundFile=false" in text,
                       "Wrong Java writer RAM/compound setting")
        field = next(line for line in lines if line.startswith("FIELD_CONFIG\t"))
        rt.require(f'dimension={config["dimensions"]}' in field and f'similarity={config["similarity"]}' in field,
                   "Wrong field configuration")
        files = rt.files(index, exclude=("write.lock",))
        rt.require(not any(name.endswith((".cfs", ".cfe")) for name in files), "Compound index")
        row.update(engine=engine, batch=batch, label=label, index=str(index), index_files=files,
                   index_bytes=sum(x["bytes"] for x in files.values()),
                   peak_rss_bytes=observation["peak_rss_bytes"], actual_config=text.split("INDEXED\t")[0],
                   total_vectors_per_second=config["documents"] * 1e9 / row["total_ns"])
        run["builds"].append(row)
        rt.save(path / "RUN.json", run)
        print("BUILT_INDEX", label, "total_s", row["total_ns"] / 1e9, flush=True)


def selected_indices(args, run):
    if run["builds"]:
        return run["builds"]
    if args.index_run:
        previous = rt.load(args.index_run / "RUN.json")
        rt.require(previous["config"] == run["config"], "Index preset differs from requested preset")
        rt.require(previous["inputs"]["files"]["base.raw"] == run["inputs"]["files"]["base.raw"], "Index input bytes differ")
        rt.require(previous.get("builds"), "Index run contains no completed index")
        for row in previous["builds"]:
            rt.require(rt.files(Path(row["index"]), exclude=("write.lock",)) == row["index_files"], "Index changed")
        run["index_source_run"] = {"path": str(args.index_run), "receipt_sha256": rt.sha(args.index_run / "RUN.json")}
        return previous["builds"]
    rt.require(args.index is not None, "Supply --index-run or --index; this command never builds an index implicitly")
    return [{"label": "external", "index": str(args.index), "index_files": rt.files(args.index, exclude=("write.lock",))}]


class Server:
    def __init__(self, receipt, engine, path, index, label, fanout):
        args = [index, path["data"] / "query.raw", path["raw"], fanout]
        if engine == "java":
            args.insert(0, "server")
        self.raw = path["raw"]
        self.protocol = (self.raw / (label + ".protocol")).open("w", encoding="utf-8")
        try:
            self.server = rt.Server(build.command(receipt, engine, "query", args), self.raw / (label + ".err"), timeout=3600)
            rt.require(self.server.read() == "READY", "Missing READY")
            self.protocol.write("READY\n")
            self.protocol.flush()
        except BaseException:
            self.abort()
            raise

    def ask(self, request):
        self.protocol.write("> " + request + "\n")
        self.protocol.flush()
        response, _ = self.server.ask(request)
        self.protocol.write(response + "\n")
        self.protocol.flush()
        fields = response.split("\t")
        rt.require(fields[0] == request.split()[0], "Wrong response: " + response)
        return fields

    def close(self):
        try:
            return self.server.close("QUIT")
        finally:
            self.protocol.close()

    def abort(self):
        try:
            if hasattr(self, "server"):
                self.server.abort()
        finally:
            self.protocol.close()


def compare(path, a, b, run, label):
    result = metrics.compare(path / "raw", a, b, run["config"], path / "differences" / (label + ".csv"))
    rt.save(path / "differences" / (label + ".json"), result)
    if run["score_policy"] == "strict":
        rt.require(result["ordered_IDs_and_f32_bits_equal"] and result["visited_equal"], "Strict query result check failed: " + label)
    return result


def verify_indices(args, path, run, indices):
    run["compatibility"] = {"complete": False, "indices": []}
    first = None
    for row in indices:
        label, index = row["label"], Path(row["index"])
        canonical = path / "raw" / (label + "-canonical.bin")
        before = rt.files(index, exclude=("write.lock",))
        execute(build.command(run["build"], "rust", "verify", [index, args.data / "base.raw", canonical]),
                path, label + "-rust-verify", run)
        execute(build.command(run["build"], "java", "verify", [index, args.data / "base.raw"]),
                path, label + "-java-verify", run)
        outputs = {}
        for engine in ("rust", "java"):
            lines = (path / "raw" / (label + "-" + engine + "-verify.out")).read_text().splitlines()
            outputs[engine] = {line.split("\t")[0]: line.split("\t")[1:] for line in lines}
        r, j = outputs["rust"], outputs["java"]
        digest = rt.sha(canonical)
        rt.require(r["VERIFIED"][:2] == j["VERIFIED"][:2] == [str(run["config"]["documents"]), str(run["config"]["dimensions"])], "Shape mismatch")
        rt.require(j["VERIFIED"][2] == run["inputs"]["files"]["base.raw"]["sha256"] and j["VERIFIED"][3] == digest and
                   r["VERIFIED"][2:] == j["VERIFIED"][4:], "Vector/graph digest mismatch")
        rt.require(r["MAPPING"][0] == j["MAPPING"][0] and rt.sha(str(canonical) + ".map.bin") == j["MAPPING"][1], "ID mapping mismatch")
        rt.require(rt.files(index, exclude=("write.lock",)) == before, "Index changed during verification")
        if first is None:
            first = digest
        record = {"index": str(index), "label": label, "vector_f32_bits_equal_to_original": True,
                  "stored_ID_doc_ord_mapping_equal": True, "permuted_stored_IDs": int(r["MAPPING"][0]),
                  "canonical_equal_between_readers": True, "canonical_sha256": digest,
                  "same_graph_as_first_writer": digest == first, "integrity_both_readers": True, "cold_queries": []}
        run["compatibility"]["indices"].append(record)
        rt.save(path / "RUN.json", run)
        for fanout in run["config"]["fanouts"]:
            tags = {}
            for engine in ("rust", "java"):
                tag = f"cold-{label}-f{fanout}-{engine}"
                server = Server(run["build"], engine, {"data": args.data, "raw": path / "raw"}, index, tag, fanout)
                try:
                    server.ask("CHECK " + tag)
                    server.ask("VISIT " + tag + "-direct")
                    check = compare(path, tag, tag + "-direct", run, tag + "-independent")
                    record["cold_queries"].append({"engine": engine, "fanout": fanout, "independent": check})
                    server.close()
                except BaseException:
                    server.abort()
                    raise
                tags[engine] = tag
            record["cold_queries"].append({"fanout": fanout, "cross_engine": compare(path, tags["rust"], tags["java"], run, f"cold-{label}-f{fanout}-cross")})
            rt.save(path / "RUN.json", run)
        print("VERIFIED_INDEX_BOTH_READERS", label, flush=True)
    run["compatibility"]["complete"] = True


def queries(args, path, run, index):
    before = rt.files(index, exclude=("write.lock",))
    run["query_index"] = {"path": str(index), "files": before}
    c = run["config"]
    for fanout in c["fanouts"]:
        profile = {"fanout": fanout, "batches": []}
        run["profiles"].append(profile)
        for batch in (1, 2):
            row = {"batch": batch, "rounds": [], "warmups": {}, "quality": {}, "checks": {}, "usage": {}, "complete": False}
            profile["batches"].append(row)
            rt.save(path / "RUN.json", run)
            servers = {}
            try:
                order = ["rust", "java"] if batch == 1 else ["java", "rust"]
                for engine in order:
                    servers[engine] = Server(run["build"], engine, {"data": args.data, "raw": path / "raw"},
                                             index, f"f{fanout}-b{batch}-{engine}", fanout)
                for engine in order:
                    print("WARM", fanout, batch, engine, flush=True)
                    warm = servers[engine].ask("WARM 60")
                    rt.require(int(warm[1]) >= 60_000_000_000, "Insufficient actual warmup")
                    row["warmups"][engine] = dict(ns=int(warm[1]), queries=int(warm[2]), sweeps=int(warm[3]))
                    tag = f"hot-f{fanout}-b{batch}-{engine}"
                    servers[engine].ask("CHECK " + tag)
                    servers[engine].ask("VISIT " + tag + "-direct")
                    compare(path, tag, tag + "-direct", run, tag + "-independent")
                    row["checks"][engine] = tag
                    row["quality"][engine] = metrics.quality(path / "raw", tag, c, args.data / "groundtruth.raw")
                    rt.save(path / "RUN.json", run)
                row["cross_engine"] = compare(path, row["checks"]["rust"], row["checks"]["java"], run, f"f{fanout}-b{batch}-cross")
                for n in range(c["rounds"]):
                    engines = ["rust", "java"] if (n + batch) % 2 else ["java", "rust"]
                    observation = {"round": n + 1, "order": engines, "RUN": {}, "LATENCY": {}}
                    row["rounds"].append(observation)
                    for phase, sequence in (("RUN", engines), ("LATENCY", list(reversed(engines)))):
                        for engine in sequence:
                            tag = f"f{fanout}-b{batch}-r{n+1:02}-{engine}-" + phase.lower()
                            response = servers[engine].ask(phase + " " + tag)
                            rt.require(response[1] == tag, "Query tag mismatch")
                            observation[phase][engine] = {"tag": tag, "elapsed_ns": int(response[2]), "checksum": int(response[3]),
                                "process_cpu_ns": int(response[4]), "thread_cpu_ns": int(response[5]), "thread_cpu_valid": response[6] == "true",
                                "visited_equal_to_hot_check": (path / "raw" / (tag + ".visited")).read_bytes() ==
                                                              (path / "raw" / (row["checks"][engine] + ".visited")).read_bytes()}
                            rt.save(path / "RUN.json", run)
                            if run["score_policy"] == "strict":
                                rt.require(observation[phase][engine]["visited_equal_to_hot_check"], "Visited changed after strict correctness check")
                    ratio = observation["RUN"]["rust"]["elapsed_ns"] / observation["RUN"]["java"]["elapsed_ns"]
                    print("QUERY_ROUND", fanout, batch, n + 1, "rust/java", ratio, flush=True)
                for engine in list(servers):
                    row["usage"][engine] = servers[engine].close()
                    del servers[engine]
                row["complete"] = True
                rt.save(path / "RUN.json", run)
            finally:
                for server in servers.values():
                    server.abort()
        profile["cross_batch"] = {e: compare(path, profile["batches"][0]["checks"][e], profile["batches"][1]["checks"][e],
                                            run, f"f{fanout}-{e}-cross-batch") for e in ("rust", "java")}
    rt.require(rt.files(index, exclude=("write.lock",)) == before, "Index changed during queries")


def measured(args, config):
    path = rt.new_run(SUITE, args.output, args.action + "-" + config["id"])
    for name in ("raw", "differences"):
        (path / name).mkdir()
    run = {"status": "RUNNING", "action": args.action, "config": config, "machine": rt.machine(),
           "start": time.time(), "commands": [], "builds": [], "profiles": [],
           "score_policy": args.score_policy or config["score_policy"], "standard_full_run": False}
    rt.save(path / "RUN.json", run)
    print("OUTPUT", path, flush=True)
    # Hold the lock until cleanup and report export have both completed.
    try:
        with rt.Lock(), environment(config):
            try:
                run["build"] = build.programs(args.java_home)
                run["inputs"] = inputs(args.data, config, truth=args.action in ("query", "all"), accept_new=args.accept_new_inputs)
                if args.action in ("build-index", "all"):
                    writers(args, path, run)
                indices = selected_indices(args, run)
                run["selected_indices"] = indices
                if args.action in ("verify-index", "all"):
                    verify_indices(args, path, run, indices)
                if args.action in ("query", "all"):
                    index = Path(next((x["index"] for x in indices if x["label"] == "b1-java"), indices[0]["index"]))
                    queries(args, path, run, index)
                rt.require(build.programs(args.java_home) == run["build"], "Source/program identity changed during run")
                for row in indices:
                    rt.require(rt.files(Path(row["index"]), exclude=("write.lock",)) == row["index_files"], "Index changed during run")
                rt.require(inputs(args.data, config, truth=args.action in ("query", "all"), accept_new=args.accept_new_inputs) == run["inputs"],
                           "Inputs changed during run")
                run["status"] = "COMPLETE"
                run["standard_full_run"] = (args.action == "all" and run["inputs"]["published_vector_bits_match"]
                                            and run["score_policy"] == config["score_policy"])
            except KeyboardInterrupt:
                run.update(status="STOPPED_BY_USER", automatic_resume=False)
                raise
            except BaseException as error:
                run.update(status="FAILED", error=repr(error))
                raise
            finally:
                run["end"] = time.time()
                rt.save(path / "RUN.json", run)
                metrics.summarize(path)
    except KeyboardInterrupt:
        # Interrupts during lock acquisition happen before the inner stage guard.
        if run["status"] == "RUNNING":
            run.update(status="STOPPED_BY_USER", automatic_resume=False, end=time.time())
            rt.save(path / "RUN.json", run)
            metrics.summarize(path)
        raise


def main():
    def stop(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, stop)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["presets", "plan", "build", "prepare-data", "truth", "build-index", "verify-index", "query", "all", "report"])
    parser.add_argument("--preset", choices=sorted(p.stem for p in PRESETS.glob("*.json")))
    parser.add_argument("--data", type=Path)
    parser.add_argument("--sources", type=Path, help="Verified Wiki/GloVe/model inputs; see datasets/vector-wiki")
    parser.add_argument("--download", action="store_true", help="Explicitly download missing pinned source files")
    parser.add_argument("--inference-python", type=Path, help="Pinned inference virtualenv Python, MiniLM/MPNet only")
    parser.add_argument("--index", type=Path, help="Read-only external single-segment index; build parameters unverified")
    parser.add_argument("--index-run", type=Path, help="Reuse completed indices from a retained build-index/all RUN.json")
    parser.add_argument("--output", type=Path, help="New run directory; report uses an existing directory")
    parser.add_argument("--java-home")
    parser.add_argument("--jars-dir", type=Path)
    parser.add_argument("--score-policy", choices=["strict", "report"], help="Result differences policy; structural index checks stay strict")
    parser.add_argument("--accept-new-inputs", action="store_true", help="Allow newly identified generated vector bytes, never stale groundtruth")
    args = parser.parse_args()
    if args.action == "presets":
        for p in sorted(PRESETS.glob("*.json")):
            c = rt.load(p)
            print(c["id"], f'{c["documents"]}x{c["dimensions"]}', f'{c["queries"]} queries', f'M{c["M"]}/beam{c["beam_width"]}/RAM{c["ram_buffer_MiB"]}')
        return
    if args.action == "report":
        rt.require(args.output is not None, "report requires --output")
        metrics.summarize(args.output.expanduser().resolve())
        return
    rt.require(args.preset is not None, "Select --preset; inspect available values with presets")
    config = preset(args.preset)
    for key in ("data", "sources", "index", "index_run", "output", "inference_python"):
        value = getattr(args, key)
        if value is not None:
            setattr(args, key, value.expanduser().resolve())
    args.data = args.data or rt.BENCHMARK / "data" / config["id"]
    if args.action == "plan":
        print(json.dumps({"config": config, "data": str(args.data), "commands_execute": False,
                          "index_reuse": str(args.index_run or args.index) if args.index_run or args.index else None}, indent=2))
        return
    rt.macos()
    if args.action == "build":
        with rt.Lock():
            build.build(args.java_home, args.jars_dir)
    elif args.action in ("prepare-data", "truth"):
        import data
        with rt.Lock(), environment(config):
            data.prepare(args, config) if args.action == "prepare-data" else data.truth(args, config)
    else:
        measured(args, config)

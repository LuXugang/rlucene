"""Five-mode Wikipedia comparison, reusing the validated macOS select protocol."""
import argparse
import collections
import csv
import json
from pathlib import Path
import random
import statistics
import subprocess
import sys
import time
import traceback

SUITE = Path(__file__).resolve().parent
sys.path.insert(0, str(SUITE.parent))
from common import runtime as rt

CONFIG = rt.load(SUITE / "configs/wiki-five-modes.json")
MODES = CONFIG["modes"]
DISPLAY_MODES = CONFIG["display_modes"]
ENGINES = rt.ENGINES
load, save, require = rt.load, rt.save, rt.require


def queries():
    expected = rt.load(SUITE.parent / "datasets/wiki/manifest.json")["queries"]
    require(rt.sha(SUITE / "queries.jsonl") == expected["sha256"], "Query file changed")
    rows = [json.loads(line) for line in (SUITE / "queries.jsonl").read_text().splitlines()]
    require(len(rows) == expected["count"] == 962, "Query count changed")
    return rows


def verify(args, output, rows, qids):
    output.mkdir()
    receipt = {"ok": False, "checks": []}
    references = {}
    active = None
    try:
        commands = rt.commands(SUITE, {engine: ["verify", args.index] for engine in ENGINES}, args.java_home)
        for engine in ENGINES:
            active = rt.Server(commands[engine], output / (engine + "-stderr.log"))
            count = hits = 0
            with (output / (engine + "-responses.jsonl")).open("w") as stream:
                for mode in MODES:
                    for qid in qids:
                        line, _ = active.ask(mode + "\t" + rows[qid]["query"])
                        value = json.loads(line)
                        stream.write(json.dumps({"mode": mode, "qid": qid, "query": rows[qid]["query"], "result": value}) + "\n")
                        stream.flush()
                        if engine == "rust":
                            references[mode, qid] = value
                        else:
                            require(value == references[mode, qid], "Full ordered output mismatch: %s qid=%d; both outputs retained" % (mode, qid))
                        count += 1
                        hits += len(value.get("docs", []))
                    print("VERIFIED", engine, mode, count, hits, flush=True)
            active.close()
            active = None
            receipt["checks"].append({"engine": engine, "requests": count, "ordered_hits": hits})
        receipt["ok"] = True
    except BaseException as error:
        receipt.update(error=repr(error), traceback=traceback.format_exc())
        raise
    finally:
        if active:
            active.abort()
        save(output / "RECEIPT.json", receipt)
    return receipt


def measure(args, output, rows, mode, batch):
    output.mkdir()
    record = {"ok": False, "mode": mode, "batch": batch, "warmup": [], "rounds": []}
    order = list(range(len(rows)))
    random.Random(CONFIG["seed_base"] + batch).shuffle(order)
    record.update(query_order=order, seed=CONFIG["seed_base"] + batch)
    servers = {}
    permutations = [ENGINES, list(reversed(ENGINES))]
    try:
        commands = rt.commands(SUITE, {engine: ["serve", args.index] for engine in ENGINES}, args.java_home)
        record["commands"] = commands
        for engine in ENGINES:
            servers[engine] = rt.Server(commands[engine], output / (engine + "-stderr.log"))
        warm = dict.fromkeys(ENGINES, 0)
        passes = 0
        while min(warm.values()) < args.warmup_seconds * 1e9:
            times = dict.fromkeys(ENGINES, 0)
            for position, qid in enumerate(order):
                values = []
                for engine in permutations[(batch + passes + position) % 2]:
                    response, ns = servers[engine].ask(mode + "\t" + rows[qid]["query"])
                    values.append(int(response))
                    times[engine] += ns
                require(values[0] == values[1], "Warmup result mismatch: %s qid=%d" % (mode, qid))
            for engine in ENGINES:
                warm[engine] += times[engine]
            passes += 1
            record["warmup"].append({"pass": passes, "ns": times})
            save(output / "PROGRESS.json", record)
            print("WARM", mode, batch, passes, {e: round(warm[e] / 1e9, 2) for e in ENGINES}, flush=True)
        with (output / "samples.jsonl").open("w") as stream:
            for rnd in range(args.rounds):
                times = dict.fromkeys(ENGINES, 0)
                for position, qid in enumerate(order):
                    sequence = permutations[(batch + rnd + position) % 2]
                    results = {}
                    for engine in sequence:
                        response, ns = servers[engine].ask(mode + "\t" + rows[qid]["query"])
                        results[engine] = {"ns": ns, "value": int(response)}
                        times[engine] += ns
                    stream.write(json.dumps({"mode": mode, "batch": batch, "round": rnd, "qid": qid,
                                             "query": rows[qid]["query"], "tags": rows[qid]["tags"],
                                             "order": sequence, "results": results}, separators=(",", ":")) + "\n")
                    stream.flush()
                    require(results["rust"]["value"] == results["java"]["value"], "Formal result mismatch")
                record["rounds"].append({"round": rnd, "ns": times})
                save(output / "PROGRESS.json", record)
                print("ROUND", mode, batch, rnd, {e: round(times[e] / 1e9, 6) for e in ENGINES}, flush=True)
        for engine in list(servers):
            servers[engine].close()
            del servers[engine]
        record["ok"] = True
    except BaseException as error:
        record.update(error=repr(error), traceback=traceback.format_exc())
        raise
    finally:
        for server in servers.values():
            server.abort()
        save(output / "RECEIPT.json", record)
    return record


def summarize(run):
    config = load(run / "RUN.json")
    result = {"schema_version": 1, "method_id": "search-benchmark-game", "protocol_version": config.get("protocol_version", "unknown"),
              "status": config["status"], "run_receipt_sha256": rt.sha(run / "RUN.json"),
              "complete": False, "run_status": config["status"], "modes": {},
              "formal_single_engine_requests": 0, "standard_full_run": config["standard_full_run"]}
    flat = []
    completed = 0
    for mode in DISPLAY_MODES:
        if mode not in config["modes"]:
            continue
        batches = []
        per = collections.defaultdict(dict)
        groups = {}
        for batch in range(1, config["batches"] + 1):
            folder = run / ("mixed-%s-%d" % (mode, batch))
            receipt = folder / "RECEIPT.json"
            if not receipt.is_file() or not load(receipt).get("ok"):
                continue
            rec = load(receipt)
            rows = [json.loads(line) for line in (folder / "samples.jsonl").read_text(encoding="utf-8").splitlines()]
            require(len(rows) == 962 * config["rounds"], "Sample count mismatch: " + str(folder))
            require(len({(x["qid"], x["round"]) for x in rows}) == len(rows), "Duplicate samples")
            require(len(rec["rounds"]) == config["rounds"], "Round count mismatch")
            warm = {e: sum(x["ns"][e] for x in rec["warmup"]) / 1e9 for e in ENGINES}
            require(min(warm.values()) >= config["warmup_seconds"], "Warmup shorter than requested")
            delta = [100 * (r["ns"]["rust"] / r["ns"]["java"] - 1) for r in rec["rounds"]]
            batches.append({"batch": batch, "paired_change_pct": statistics.median(delta),
                            "faster_rounds": sum(x < 0 for x in delta), "warmup_seconds": warm,
                            "median_round_seconds": {e: statistics.median(r["ns"][e] / 1e9 for r in rec["rounds"])
                                                     for e in ENGINES}})
            by_query = collections.defaultdict(list)
            by_tag = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
            tag_qids = collections.defaultdict(set)
            for row in rows:
                require(row["results"]["rust"]["value"] == row["results"]["java"]["value"], "Response mismatch")
                by_query[row["qid"]].append(row)
                for tag in row["tags"]:
                    tag_qids[tag].add(row["qid"])
                    for engine in ENGINES:
                        by_tag[tag][row["round"]][engine] += row["results"][engine]["ns"]
            for qid, samples in by_query.items():
                ratios = [100 * (s["results"]["rust"]["ns"] / s["results"]["java"]["ns"] - 1) for s in samples]
                entry = {"mode": mode, "batch": batch, "qid": qid, "query": samples[0]["query"],
                         "tags": samples[0]["tags"], "paired_change_pct": statistics.median(ratios),
                         "paired_delta_ms": statistics.median((s["results"]["rust"]["ns"] - s["results"]["java"]["ns"]) / 1e6 for s in samples),
                         "slower_samples": sum(x > 0 for x in ratios),
                         "rust_ms": statistics.median(s["results"]["rust"]["ns"] / 1e6 for s in samples),
                         "java_ms": statistics.median(s["results"]["java"]["ns"] / 1e6 for s in samples)}
                flat.append(entry)
                per[qid][str(batch)] = entry
            groups[str(batch)] = {tag: {"queries": len(tag_qids[tag]),
                                      "paired_change_pct": statistics.median(100 * (r["rust"] / r["java"] - 1) for r in rounds.values())}
                                  for tag, rounds in by_tag.items()}
            completed += 1
            result["formal_single_engine_requests"] += 2 * len(rows)
        slower = [qid for qid, values in per.items() if len(values) == config["batches"]
                  and all(x["paired_change_pct"] > 0 for x in values.values())]
        result["modes"][mode] = {"batches": batches, "slower_all_batches_qids": slower, "tag_groups": groups}
    result["completed_batches"] = completed
    result["complete"] = config["status"] == "COMPLETE" and completed == len(config["modes"]) * config["batches"]
    save(run / "RESULT.json", result)
    save(run / "REGRESSIONS.json", [x for x in flat if x["paired_change_pct"] > 0])
    with (run / "per-query.csv").open("w", newline="", encoding="utf-8-sig") as stream:
        fields = ["mode", "batch", "qid", "query", "rust_ms", "java_ms", "paired_change_pct", "paired_delta_ms", "slower_samples"]
        writer = csv.DictWriter(stream, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(flat)
    report = ["# Rust / Java benchmark", "", "Status: " + config["status"],
              "", "Negative change means Rust takes less time. Timing includes IPC and query parsing.",
              "macOS select/read IPC backend; full-hit verification is outside timing.",
              "", "| Mode | Batch | Rust / Java change | Faster rounds |", "|---|---:|---:|---:|"]
    for mode, data in result["modes"].items():
        for batch in data["batches"]:
            report.append("| %s | %d | %+.3f%% | %d/%d |" % (mode, batch["batch"], batch["paired_change_pct"], batch["faster_rounds"], config["rounds"]))
    report += ["", "All per-query data: per-query.csv. Queries with a slower paired median in any batch: REGRESSIONS.json.",
               "Overlapping tag groups must not be added together. Partial/stopped runs are not complete benchmarks.", ""]
    (run / "REPORT.md").write_text("\n".join(report), encoding="utf-8")
    print("REPORT", run / "REPORT.md", flush=True)
    return result


def execute(args, smoke):
    rows = queries()
    path = rt.new_run(SUITE, args.output, "smoke" if smoke else "full")
    record = {"status": "RUNNING", "method_id": "search-benchmark-game", "protocol_version": CONFIG["protocol_version"],
              "config": CONFIG,
              "modes": args.modes, "batches": args.batches, "rounds": args.rounds, "warmup_seconds": args.warmup_seconds,
              "standard_full_run": not smoke and args.modes == MODES and args.batches == 2 and args.rounds == 12 and args.warmup_seconds == 60,
              "measurement": CONFIG["measurement"], "machine": rt.machine(), "start": time.time()}
    print("OUTPUT", path, flush=True)
    try:
        with rt.Lock():
            record["build"] = rt.programs(SUITE, args.java_home)
            record["index_files"] = rt.files(args.index, exclude=("write.lock",))
            record["index_path"] = str(args.index)
            published = {x["path"]: {"bytes": x["bytes"], "sha256": x["sha256"]}
                         for x in load(SUITE.parent / "datasets/wiki/published-index.json") if x["path"] != "write.lock"}
            record["published_index_match"] = record["index_files"] == published
            if args.require_published_index:
                require(record["published_index_match"], "Not the exact published physical index")
            record["index_before"] = rt.snapshot(args.index)
            command = rt.commands(SUITE, {engine: ["info", args.index] for engine in ENGINES}, args.java_home)["java"]
            geometry = subprocess.run(command, capture_output=True, text=True, check=True, env=rt.environment())
            (path / "java-info-stderr.log").write_text(geometry.stderr)
            record["geometry"] = json.loads(geometry.stdout)
            expected = load(SUITE.parent / "datasets/wiki/manifest.json")["index"]["geometry"].copy()
            expected.pop("deletions")
            require(record["geometry"] == expected, "Wrong Wiki index geometry")
            record["verification"] = verify(args, path / "verify", rows, [0, 1, 2, 565, 900, 961] if smoke else range(len(rows)))
            if not smoke:
                for batch in range(1, args.batches + 1):
                    for mode in args.modes:
                        measure(args, path / ("mixed-%s-%d" % (mode, batch)), rows, mode, batch)
            require(rt.programs(SUITE, args.java_home) == record["build"], "Programs changed during run")
            require(rt.snapshot(args.index) == record["index_before"], "Index changed during run")
            record["status"] = "SMOKE_OK" if smoke else "COMPLETE"
    except BaseException as error:
        record.update(status="STOPPED_BY_USER" if isinstance(error, KeyboardInterrupt) else "FAILED",
                      error=repr(error), traceback=traceback.format_exc())
        raise
    finally:
        record["end"] = time.time()
        save(path / "RUN.json", record)
        if not smoke:
            summarize(path)
    print(record["status"], path, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["deps", "build", "check", "prepare-index", "smoke", "run", "summarize"])
    parser.add_argument("--index", type=Path, default=SUITE.parent / "data/wiki/index")
    parser.add_argument("--corpus", type=Path)
    parser.add_argument("--java-home")
    parser.add_argument("--jars-dir", type=Path, help="Optional existing directory of the pinned official JARs")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-published-index", action="store_true")
    parser.add_argument("--modes", nargs="+", choices=MODES, default=MODES)
    parser.add_argument("--batches", type=int, default=CONFIG["batches"])
    parser.add_argument("--rounds", type=int, default=CONFIG["rounds"])
    parser.add_argument("--warmup-seconds", type=int, default=CONFIG["warmup_seconds_per_engine"])
    args = parser.parse_args()
    rt.macos()
    args.index = args.index.expanduser().resolve()
    require(args.batches > 0 and args.rounds > 0 and args.rounds % 2 == 0 and args.warmup_seconds >= 60, "Use positive batches, even rounds, and >=60 seconds warmup")
    require(len(set(args.modes)) == len(args.modes), "Duplicate modes")
    if args.action == "deps":
        rt.dependencies(args.jars_dir)
    elif args.action == "build":
        rt.build(SUITE, "the-cost-diagnostic", "Reference", args.java_home, args.jars_dir)
    elif args.action == "summarize":
        require(args.output is not None, "summarize requires --output")
        summarize(args.output.resolve())
    elif args.action == "check":
        queries()
        print(json.dumps(rt.files(args.index, exclude=("write.lock",)), indent=2))
    elif args.action == "prepare-index":
        require(args.corpus is not None and args.corpus.is_file(), "prepare-index requires --corpus corpus.json")
        expected = load(SUITE.parent / "datasets/wiki/manifest.json")["corpus"]
        require(rt.sha(args.corpus) == expected["sha256"], "Wrong transformed Wiki corpus")
        require(not args.index.exists(), "Index output already exists; choose a new directory")
        with rt.Lock():
            command = rt.commands(SUITE, {engine: ["index", args.index] for engine in ENGINES}, args.java_home)["rust"]
            args.index.parent.mkdir(parents=True, exist_ok=True)
            with args.corpus.open("rb") as stream:
                subprocess.run(command, stdin=stream, env=rt.environment(), check=True)
            save(args.index.parent / (args.index.name + "-manifest.json"), rt.files(args.index, exclude=("write.lock",)))
    else:
        execute(args, args.action == "smoke")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)

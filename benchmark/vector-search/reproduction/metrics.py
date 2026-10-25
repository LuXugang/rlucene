"""Read retained observations. Reporting never launches a compiler or engine."""
import csv
import math
from pathlib import Path
import statistics
import struct

from common import runtime as rt

HEADERS = ["recall", "latency(ms)", "netCPU", "avgCpuCount", "nDoc", "searchType", "topK", "fanout",
           "resultSimilarity", "decay", "resultCount", "maxConn", "beamWidth", "quantized", "visited",
           "index(s)", "index_docs/s", "merge(s)", "force_merge(s)", "num_segments", "index_size(MB)",
           "filterStrategy", "filterSelectivity", "overSample", "vec_disk(MB)", "vec_RAM(MB)",
           "bp-reorder", "indexType", "rerank"]


def percentile(values, p):
    values = sorted(values)
    x = (len(values) - 1) * p
    lo = int(x)
    hi = min(lo + 1, len(values) - 1)
    return values[lo] + (values[hi] - values[lo]) * (x - lo)


def hits(path, config):
    raw = Path(path).read_bytes()
    result, pos = [], 0
    for _ in range(config["queries"]):
        rt.require(pos + 4 <= len(raw), "Truncated hits file")
        count = struct.unpack_from(">I", raw, pos)[0]
        pos += 4
        rt.require(count == config["top_k"] and pos + count * 8 <= len(raw), "Wrong TopK count")
        row = list(struct.iter_unpack(">iI", raw[pos:pos + count * 8]))
        pos += count * 8
        rt.require(len({doc for doc, _ in row}) == count, "Duplicate result IDs")
        for doc, bits in row:
            score = struct.unpack(">f", struct.pack(">I", bits))[0]
            rt.require(0 <= doc < config["documents"] and math.isfinite(score) and score >= 0, "Invalid result")
        result.append(row)
    rt.require(pos == len(raw), "Trailing hits data")
    return result


def visited(path, count):
    raw = Path(path).read_bytes()
    rt.require(len(raw) == count * 8, "Wrong visited count")
    return list(struct.unpack(">%dQ" % count, raw))


def compare(directory, a, b, config, output=None):
    directory = Path(directory)
    left, right = hits(directory / (a + ".hits"), config), hits(directory / (b + ".hits"), config)
    av, bv = [visited(directory / (tag + ".visited"), config["queries"]) for tag in (a, b)]
    totals = {"ordered_ID_position_differences": 0, "result_set_different_queries": 0,
              "same_document_score_bit_differences": 0, "max_positive_score_ULP": 0,
              "visited_different_queries": 0}
    stream = Path(output).open("w", newline="", encoding="utf-8") if output else None
    writer = csv.writer(stream) if stream else None
    if writer:
        writer.writerow(["query_id", "ordered_ID_position_differences", "left_only_IDs", "right_only_IDs",
                         "same_document_score_bit_differences", "max_positive_score_ULP", "left_visited", "right_visited"])
    try:
        for qid, (a_hits, b_hits) in enumerate(zip(left, right)):
            ad, bd = dict(a_hits), dict(b_hits)
            order = sum(x[0] != y[0] for x, y in zip(a_hits, b_hits))
            aonly, bonly = sorted(ad.keys() - bd.keys()), sorted(bd.keys() - ad.keys())
            diff = [abs(ad[doc] - bd[doc]) for doc in ad.keys() & bd.keys() if ad[doc] != bd[doc]]
            maximum = max(diff, default=0)
            totals["ordered_ID_position_differences"] += order
            totals["result_set_different_queries"] += bool(aonly or bonly)
            totals["same_document_score_bit_differences"] += len(diff)
            totals["max_positive_score_ULP"] = max(totals["max_positive_score_ULP"], maximum)
            totals["visited_different_queries"] += av[qid] != bv[qid]
            if writer:
                writer.writerow([qid, order, aonly, bonly, len(diff), maximum, av[qid], bv[qid]])
    finally:
        if stream:
            stream.close()
    totals.update(left=a, right=b, ordered_IDs_and_f32_bits_equal=left == right,
                  visited_equal=av == bv, queries=config["queries"])
    return totals


def quality(directory, tag, config, truth):
    rows = hits(Path(directory) / (tag + ".hits"), config)
    counts = visited(Path(directory) / (tag + ".visited"), config["queries"])
    raw = Path(truth).read_bytes()
    k = config["top_k"]
    rt.require(len(raw) == config["queries"] * k * 4, "Wrong groundtruth shape")
    recall = []
    for qid, row in enumerate(rows):
        expected = set(struct.unpack_from("<%dI" % k, raw, qid * k * 4))
        rt.require(len(expected) == k and all(0 <= x < config["documents"] for x in expected), "Invalid groundtruth IDs")
        recall.append(len(expected.intersection(doc for doc, _ in row)) / k)
    return {"recall_at_100": statistics.mean(recall), "recall_by_query": recall,
            "recall_min": min(recall), "recall_p05": percentile(recall, .05),
            "visited_mean": statistics.mean(counts), "visited_total": sum(counts),
            "visited_p95": percentile(counts, .95)}


def summarize(path):
    path = Path(path)
    run = rt.load(path / "RUN.json")
    c = run["config"]
    result = {"schema_version": 1, "method_id": "vector-search", "protocol_version": c["protocol_version"],
              "status": run["status"], "complete": run["status"] == "COMPLETE",
              "run_receipt_sha256": rt.sha(path / "RUN.json"), "config": c,
              "standard_full_run": run.get("standard_full_run", False),
              "builds": run.get("builds", []), "build_comparisons": [],
              "compatibility": run.get("compatibility"), "profiles": [],
              "all_samples_retained": True, "score_policy": run.get("score_policy", c["score_policy"]),
              "missing_build_metrics_are_not_zero": True}
    for batch in (1, 2):
        pair = {x["engine"]: x for x in result["builds"] if x["batch"] == batch}
        if set(pair) == {"rust", "java"}:
            result["build_comparisons"].append({"batch": batch,
                "rust_total_s": pair["rust"]["total_ns"] / 1e9, "java_total_s": pair["java"]["total_ns"] / 1e9,
                "rust_time_percent": (pair["rust"]["total_ns"] / pair["java"]["total_ns"] - 1) * 100})
    official, query_csv = [], []
    for profile in run.get("profiles", []):
        p = {"fanout": profile["fanout"], "batches": [], "cross_batch": profile.get("cross_batch")}
        for batch in profile["batches"]:
            rounds = [row for row in batch["rounds"] if all(set(row.get(key, {})) == {"rust", "java"}
                      for key in ("RUN", "LATENCY"))]
            if not rounds:
                continue
            b = {"batch": batch["batch"], "complete_rounds": len(rounds),
                 "complete": len(rounds) == c["rounds"] and batch.get("complete", False),
                 "quality_difference": batch.get("cross_engine"), "engines": {}}
            timings, visits = {}, {}
            for engine in ("rust", "java"):
                rows = [row["RUN"][engine] for row in rounds]
                ns = []
                for row in rounds:
                    f = path / "raw" / (row["LATENCY"][engine]["tag"] + ".ns")
                    ns.extend(visited(f, c["queries"]))
                timings[engine] = ns
                visits[engine] = visited(path / "raw" / (batch["checks"][engine] + ".visited"), c["queries"])
                count = len(rounds) * c["queries"]
                elapsed = sum(x["elapsed_ns"] for x in rows)
                process = sum(x["process_cpu_ns"] for x in rows)
                valid = all(x["thread_cpu_valid"] for x in rows)
                cpu = sum(x["thread_cpu_ns"] for x in rows) if valid else None
                qual = batch["quality"][engine]
                usage = batch.get("usage", {}).get(engine, {})
                metrics = {"mean_ms": elapsed / count / 1e6, "qps": count * 1e9 / elapsed,
                           "p50_ms": percentile(ns, .5) / 1e6, "p95_ms": percentile(ns, .95) / 1e6,
                           "p99_ms": percentile(ns, .99) / 1e6, "percentiles_source": "separate LATENCY passes",
                           "thread_cpu_valid": valid, "thread_cpu_ms_per_query": None if cpu is None else cpu / count / 1e6,
                           "thread_average_cores": None if cpu is None else cpu / elapsed,
                           "process_cpu_ms_per_query": process / count / 1e6, "process_average_cores": process / elapsed,
                           "peak_rss_MiB": usage.get("peak_rss_bytes", 0) / 1048576 if usage else None,
                           "quality": {k: v for k, v in qual.items() if k != "recall_by_query"},
                           "warmup": batch["warmups"][engine],
                           "invalid_CPU_rounds": [x["round"] for x in rounds if not x["RUN"][engine]["thread_cpu_valid"]]}
                b["engines"][engine] = metrics
                writer = next((x for x in result["builds"] if x["batch"] == b["batch"] and x["engine"] == engine), None)
                wall_ms = [x["elapsed_ns"] // 1000000 for x in rows]
                cpu_ms = [int(x["thread_cpu_ns"] / x["elapsed_ns"] * wall) for x, wall in zip(rows, wall_ms)] if valid else None
                index_s = (writer["upstream_index_ns"] // 1000000) / 1000 if writer else None
                values = [qual["recall_at_100"], sum(wall_ms) / count, sum(cpu_ms) / count if valid else None,
                          sum(cpu_ms) / sum(wall_ms) if valid and sum(wall_ms) else None, c["documents"], "KNN", 100, p["fanout"],
                          "N/A", "N/A", 100, c["M"], c["beam_width"], "no", qual["visited_total"] // c["queries"],
                          index_s, c["documents"] / index_s if index_s else None,
                          (writer["merge_wait_ns"] // 1000000) / 1000 if writer else None,
                          writer["force_merge_ns"] / 1e9 if writer else None, writer["segments"] if writer else None,
                          writer["index_bytes"] / 1048576 if writer else None,
                          "null", "N/A", 1, c["documents"] * c["dimensions"] * 4 / 1048576,
                          c["documents"] * c["dimensions"] * 4 / 1048576, "false", "HNSW", "no"]
                rt.require(len(values) == len(HEADERS) == 29, "luceneutil columns changed")
                official.append(dict(engine=engine, batch=b["batch"], thread_cpu_valid=valid, **dict(zip(HEADERS, values))))
            b["rust_time_percent"] = (b["engines"]["rust"]["mean_ms"] / b["engines"]["java"]["mean_ms"] - 1) * 100
            for qid in range(c["queries"]):
                med = {e: statistics.median(timings[e][qid::c["queries"]]) / 1e6 for e in timings}
                query_csv.append(dict(fanout=p["fanout"], batch=b["batch"], query_id=qid,
                                      rust_ms=med["rust"], java_ms=med["java"],
                                      change_percent=(med["rust"] / med["java"] - 1) * 100,
                                      rust_recall=batch["quality"]["rust"]["recall_by_query"][qid],
                                      java_recall=batch["quality"]["java"]["recall_by_query"][qid],
                                      rust_visited=visits["rust"][qid], java_visited=visits["java"][qid]))
            p["batches"].append(b)
        result["profiles"].append(p)
    rt.save(path / "RESULT.json", result)
    rt.save(path / "REGRESSIONS.json", [row for row in query_csv if row["change_percent"] > 0])
    for name, fields, rows in (("LUCENEUTIL_METRICS.csv", ["engine", "batch", "thread_cpu_valid"] + HEADERS, official),
                               ("per-query.csv", ["fanout", "batch", "query_id", "rust_ms", "java_ms", "change_percent",
                                                   "rust_recall", "java_recall", "rust_visited", "java_visited"], query_csv)):
        with (path / name).open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=fields)
            writer.writeheader()
            writer.writerows({k: "N/A" if v is None else v for k, v in row.items()} for row in rows)
    lines = ["# " + c["id"] + " / Rust vs Java 10.1", "", "Status: " + run["status"], "",
             f'{c["documents"]} documents, {c["dimensions"]}D, {c["queries"]} queries; M{c["M"]}, beam{c["beam_width"]}, RAM{c["ram_buffer_MiB"]} MiB.',
             "", "|Build batch|Rust total s|Java total s|Rust time change|", "|---|---:|---:|---:|"]
    for row in result["build_comparisons"]:
        lines.append("|{batch}|{rust_total_s:.6f}|{java_total_s:.6f}|{rust_time_percent:+.3f}%|".format(**row))
    lines += ["", "|fanout/batch|Complete rounds|Rust mean ms|Java mean ms|Rust time change|", "|---|---:|---:|---:|---:|"]
    for p in result["profiles"]:
        for b in p["batches"]:
            lines.append(f'|{p["fanout"]}/{b["batch"]}|{b["complete_rounds"]}|{b["engines"]["rust"]["mean_ms"]:.6f}|{b["engines"]["java"]["mean_ms"]:.6f}|{b["rust_time_percent"]:+.3f}%|')
    lines += ["", "Partial results remain incomplete. All raw observations, differences and CPU-invalid rounds are retained.",
              "Missing build metrics are N/A. Vector RAM/disk fields are payload estimates, not actual heap or RSS.",
              "Score policy: " + result["score_policy"] + ". See differences/*.csv and RUN.json for all checks.",
              "", "[All metrics](RESULT.json) · [luceneutil fields](LUCENEUTIL_METRICS.csv) · [Per-query latency](per-query.csv)"]
    (path / "REPORT.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return result

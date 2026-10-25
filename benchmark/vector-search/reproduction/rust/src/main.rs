/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
mod profiled;
mod thread_cpu;
use rlucene::core::index::index_reader_context::IndexReaderContext;
use rlucene::core::index::leaf_reader::LeafReader;
use rlucene::core::index::{directory_reader, index_reader::IndexReader};
use rlucene::core::index::{indexable_field::IndexableField, stored_fields::StoredFields};
use rlucene::core::search::{
  index_searcher::IndexSearcher, score_doc::ScoreDoc, top_docs::TopDocs,
};
use rlucene::core::store::mmap_directory::MMapDirectory;
use rlucene::core::util::bits::MatchNoBits;
use rlucene::core::util::close::CloseableRef;
use std::{
  fs::File,
  io::{self, BufRead, BufWriter, Read, Seek, SeekFrom, Write},
  sync::Arc,
  time::{Duration, Instant},
};

fn cpu_time() -> Result<u64> {
  let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
  // getrusage initializes the structure on success; this is standalone measurement code.
  if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
    return Err(io::Error::last_os_error().into());
  }
  let usage = unsafe { usage.assume_init() };
  Ok(
    ((usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) as u64) * 1_000_000_000
      + ((usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) as u64) * 1000,
  )
}

const K: usize = 100;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Vectors {
  file: File,
  bytes: Vec<u8>,
  target: Vec<f32>,
}
impl Vectors {
  fn new(path: &str, dim: usize) -> Result<Self> {
    Ok(Self {
      file: File::open(path)?,
      bytes: vec![0; dim * 4],
      target: vec![0.0; dim],
    })
  }
  fn reset(&mut self) -> Result<()> {
    self.file.seek(SeekFrom::Start(0))?;
    Ok(())
  }
  fn next(&mut self) -> Result<&Vec<f32>> {
    self.file.read_exact(&mut self.bytes)?;
    for (i, value) in self.target.iter_mut().enumerate() {
      *value = f32::from_le_bytes(self.bytes[i * 4..i * 4 + 4].try_into()?);
    }
    Ok(&self.target)
  }
}

fn main() -> Result<()> {
  let args: Vec<String> = std::env::args().collect();
  let fanout: usize = args[4].parse()?;
  let dim: usize = std::env::var("RLUCENE_BENCH_DIM")?.parse()?;
  let nq: usize = std::env::var("RLUCENE_BENCH_NQ")?.parse()?;
  let ndoc: i32 = std::env::var("RLUCENE_BENCH_NDOC")?.parse()?;
  let dir = Arc::new(MMapDirectory::new(std::path::Path::new(&args[1]))?);
  let reader = Arc::new(directory_reader::open(dir.clone())?);
  let mut searcher = IndexSearcher::new(reader.clone().get_context()?)?;
  searcher.set_query_cache(None);
  if reader.max_doc()? != ndoc || searcher.get_leaf_contexts()?.len() != 1 {
    return Err("wrong index".into());
  }
  eprintln!(
    "CONFIG\trust\tsegments=1\tdocs={}\tdimension={dim}\tsimilarity={}",
    reader.max_doc()?,
    std::env::var("RLUCENE_BENCH_SIMILARITY")?
  );
  let field_leaves = searcher.get_leaf_contexts()?;
  let field_infos = field_leaves[0].reader().get_field_infos()?;
  let field = field_infos
    .field_info_by_name("knn")?
    .ok_or("missing field")?;
  let expected_similarity = match std::env::var("RLUCENE_BENCH_SIMILARITY")?.as_str() {
    "EUCLIDEAN" => {
      rlucene::core::index::vector_similarity_function::VectorSimilarityFunction::Euclidean
    },
    "DOT_PRODUCT" => {
      rlucene::core::index::vector_similarity_function::VectorSimilarityFunction::DotProduct
    },
    _ => return Err("unsupported similarity".into()),
  };
  if field.get_vector_similarity_function() != &expected_similarity {
    return Err("wrong similarity".into());
  }
  let stable_ids = {
    let mut stored = reader.stored_fields()?;
    let mut ids = Vec::with_capacity(ndoc as usize);
    let mut seen = vec![false; ndoc as usize];
    for doc_id in 0..ndoc {
      let doc = stored.document(doc_id)?;
      let id = doc
        .get_field("id")
        .ok_or("missing id")?
        .numeric_value()?
        .ok_or("missing numeric id")?
        .to_i32()
        .ok_or("id type")?;
      if id < 0 || id >= ndoc || seen[id as usize] {
        return Err("invalid stored ID permutation".into());
      }
      seen[id as usize] = true;
      ids.push(id);
    }
    eprintln!(
      "ID_MAP\tpermuted={}",
      ids
        .iter()
        .enumerate()
        .filter(|(i, id)| **id != *i as i32)
        .count()
    );
    ids
  };
  let mut vectors = Vectors::new(&args[2], dim)?;
  let mut results: Vec<Option<TopDocs<ScoreDoc>>> = (0..nq).map(|_| None).collect();
  println!("READY");
  io::stdout().flush()?;
  for line in io::stdin().lock().lines() {
    let line = line?;
    let cmd: Vec<_> = line.split_whitespace().collect();
    if cmd[0] == "QUIT" {
      break;
    }
    if cmd[0] == "WARM" {
      let duration = Duration::from_secs(cmd[1].parse()?);
      let start = Instant::now();
      let mut count = 0;
      let mut sweeps = 0;
      loop {
        vectors.reset()?;
        for slot in &mut results {
          *slot = Some(profiled::query(&searcher, vectors.next()?.clone(), K, fanout)?.0);
        }
        count += nq;
        sweeps += 1;
        if start.elapsed() >= duration {
          break;
        }
      }
      println!("WARM\t{}\t{count}\t{sweeps}", start.elapsed().as_nanos());
      io::stdout().flush()?;
      continue;
    }
    if cmd[0] == "VISIT" {
      vectors.reset()?;
      let leaves = searcher.get_leaf_contexts()?;
      let leaf = leaves[0].reader();
      let mut counts = BufWriter::new(File::create(format!("{}/{}.visited", args[3], cmd[1]))?);
      let mut hits = BufWriter::new(File::create(format!("{}/{}.hits", args[3], cmd[1]))?);
      for _ in 0..nq {
        let mut docs = leaf.search_nearest_vectors_f32_with_limit::<MatchNoBits, _>(
          "knn",
          vectors.next()?.clone(),
          K + fanout,
          None,
          i32::MAX as usize,
        )?;
        counts.write_all(&(docs.total_hits.value as u64).to_be_bytes())?;
        docs.score_docs.truncate(K);
        hits.write_all(&(docs.score_docs.len() as u32).to_be_bytes())?;
        for h in docs.score_docs {
          hits.write_all(&stable_ids[h.doc as usize].to_be_bytes())?;
          hits.write_all(&h.score.to_bits().to_be_bytes())?;
        }
      }
      counts.flush()?;
      hits.flush()?;
      println!("VISIT\t{}", cmd[1]);
      io::stdout().flush()?;
      continue;
    }
    if cmd[0] != "RUN" && cmd[0] != "CHECK" && cmd[0] != "LATENCY" {
      return Err("unknown command".into());
    }
    vectors.reset()?;
    let mut ns = vec![0u64; nq];
    let mut visited = vec![0u64; nq];
    let cpu_start = cpu_time()?;
    let first = thread_cpu::Snapshot::capture()?;
    if cmd[0] == "LATENCY" {
      for (i, slot) in results.iter_mut().enumerate() {
        let start = Instant::now();
        let (docs, count) = profiled::query(&searcher, vectors.next()?.clone(), K, fanout)?;
        *slot = Some(docs);
        visited[i] = count;
        ns[i] = start.elapsed().as_nanos() as u64;
      }
    } else {
      for (i, slot) in results.iter_mut().enumerate() {
        let (docs, count) = profiled::query(&searcher, vectors.next()?.clone(), K, fanout)?;
        *slot = Some(docs);
        visited[i] = count;
      }
    }
    let last = thread_cpu::Snapshot::capture()?;
    let elapsed = last.at.duration_since(first.at).as_nanos();
    let thread_cpu = last.delta(&first);
    let cpu_elapsed = cpu_time()? - cpu_start;
    let prefix = format!("{}/{}", args[3], cmd[1]);
    if cmd[0] == "CHECK" {
      let mut out = BufWriter::new(File::create(format!("{prefix}.hits"))?);
      for docs in &results {
        let docs = docs.as_ref().ok_or("missing result")?;
        out.write_all(&(docs.score_docs.len() as u32).to_be_bytes())?;
        for hit in &docs.score_docs {
          out.write_all(&stable_ids[hit.doc as usize].to_be_bytes())?;
          out.write_all(&hit.score.to_bits().to_be_bytes())?;
        }
      }
      out.flush()?;
    } else if cmd[0] == "LATENCY" {
      let mut out = BufWriter::new(File::create(format!("{prefix}.ns"))?);
      for n in ns {
        out.write_all(&n.to_be_bytes())?;
      }
      out.flush()?;
    }
    let mut out = BufWriter::new(File::create(format!("{prefix}.visited"))?);
    for n in &visited {
      out.write_all(&n.to_be_bytes())?;
    }
    out.flush()?;
    let raw = format!(
      "{{\"wall_ns\":{elapsed},\"process_cpu_ns\":{cpu_elapsed},\"thread_cpu_ns\":{},\"thread_cpu_valid\":{},\"thread_start\":{},\"thread_end\":{},\"fanout\":{fanout},\"visited_total\":{}}}\n",
      thread_cpu.map_or(-1, |x| x as i128),
      thread_cpu.is_some(),
      first.json(),
      last.json(),
      visited.iter().sum::<u64>()
    );
    std::fs::write(format!("{prefix}.metrics.json"), raw)?;
    let checksum: i64 = results
      .iter()
      .filter_map(Option::as_ref)
      .flat_map(|d| d.score_docs.iter())
      .map(|h| i64::from(h.doc))
      .sum();
    println!(
      "{}\t{}\t{elapsed}\t{checksum}\t{cpu_elapsed}\t{}\t{}",
      cmd[0],
      cmd[1],
      thread_cpu.map_or(-1, |x| x as i128),
      thread_cpu.is_some()
    );
    io::stdout().flush()?;
  }
  reader.close()?;
  dir.close()?;
  Ok(())
}

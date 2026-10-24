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
use rlucene::core::index::index_reader_context::IndexReaderContext;
use rlucene::core::index::leaf_reader::LeafReader;
use rlucene::core::index::{directory_reader, index_reader::IndexReader};
use rlucene::core::search::{
  index_searcher::IndexSearcher, knn_float_vector_query::KnnFloatVectorQuery, score_doc::ScoreDoc,
  top_docs::TopDocs,
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

const DIM: usize = 128;
const K: usize = 100;
const NQ: usize = 10000;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Vectors {
  file: File,
  bytes: [u8; DIM * 4],
  target: Vec<f32>,
}
impl Vectors {
  fn new(path: &str) -> Result<Self> {
    Ok(Self {
      file: File::open(path)?,
      bytes: [0; DIM * 4],
      target: vec![0.0; DIM],
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
  let dir = Arc::new(MMapDirectory::new(std::path::Path::new(&args[1]))?);
  let reader = Arc::new(directory_reader::open(dir.clone())?);
  let mut searcher = IndexSearcher::new(reader.clone().get_context()?)?;
  searcher.set_query_cache(None);
  if reader.max_doc()? != 1000000 || searcher.get_leaf_contexts()?.len() != 1 {
    return Err("wrong index".into());
  }
  eprintln!("CONFIG\trust\tsegments=1\tdocs={}", reader.max_doc()?);
  let mut vectors = Vectors::new(&args[2])?;
  let mut results: Vec<Option<TopDocs<ScoreDoc>>> = (0..NQ).map(|_| None).collect();
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
          *slot = Some(searcher.search(
            KnnFloatVectorQuery::new("knn", vectors.next()?.clone(), K)?,
            K,
          )?);
        }
        count += NQ;
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
      for _ in 0..NQ {
        let docs = leaf.search_nearest_vectors_f32_with_limit::<MatchNoBits, _>(
          "knn",
          vectors.next()?.clone(),
          K,
          None,
          i32::MAX as usize,
        )?;
        counts.write_all(&(docs.total_hits.value as u64).to_be_bytes())?;
        hits.write_all(&(docs.score_docs.len() as u32).to_be_bytes())?;
        for h in docs.score_docs {
          hits.write_all(&h.doc.to_be_bytes())?;
          hits.write_all(&h.score.to_bits().to_be_bytes())?;
        }
      }
      counts.flush()?;
      hits.flush()?;
      println!("VISIT\t{}", cmd[1]);
      io::stdout().flush()?;
      continue;
    }
    if cmd[0] != "RUN" && cmd[0] != "CHECK" {
      return Err("unknown command".into());
    }
    vectors.reset()?;
    let mut ns = vec![0u64; NQ];
    let cpu_start = cpu_time()?;
    let start = Instant::now();
    for (i, slot) in results.iter_mut().enumerate() {
      let query_start = Instant::now();
      *slot = Some(searcher.search(
        KnnFloatVectorQuery::new("knn", vectors.next()?.clone(), K)?,
        K,
      )?);
      ns[i] = query_start.elapsed().as_nanos() as u64;
    }
    let elapsed = start.elapsed().as_nanos();
    let cpu_elapsed = cpu_time()? - cpu_start;
    let prefix = format!("{}/{}", args[3], cmd[1]);
    if cmd[0] == "CHECK" {
      let mut out = BufWriter::new(File::create(format!("{prefix}.hits"))?);
      for docs in &results {
        let docs = docs.as_ref().ok_or("missing result")?;
        out.write_all(&(docs.score_docs.len() as u32).to_be_bytes())?;
        for hit in &docs.score_docs {
          out.write_all(&hit.doc.to_be_bytes())?;
          out.write_all(&hit.score.to_bits().to_be_bytes())?;
        }
      }
      out.flush()?;
    } else {
      let mut out = BufWriter::new(File::create(format!("{prefix}.ns"))?);
      for n in ns {
        out.write_all(&n.to_be_bytes())?;
      }
      out.flush()?;
    }
    let checksum: i64 = results
      .iter()
      .filter_map(Option::as_ref)
      .flat_map(|d| d.score_docs.iter())
      .map(|h| i64::from(h.doc))
      .sum();
    println!(
      "{}\t{}\t{elapsed}\t{checksum}\t{cpu_elapsed}",
      cmd[0], cmd[1]
    );
    io::stdout().flush()?;
  }
  reader.close()?;
  dir.close()?;
  Ok(())
}

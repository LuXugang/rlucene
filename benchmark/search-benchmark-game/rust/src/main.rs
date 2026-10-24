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
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use rlucene::core::analysis::{
  analyzer::Analyzer, reader::ReaderEnum, standard::standard_analyzer::StandardAnalyzer,
  token_stream::TokenStream,
};
use rlucene::core::document::{
  document::Document, field::Store, numeric_doc_values_field::NumericDocValuesField,
  stored_field::StoredField, text_field::TextField,
};
use rlucene::core::index::{
  directory_reader,
  index_reader::IndexReader,
  index_writer::IndexWriter,
  index_writer_config::{IndexWriterConfig, OpenMode},
  live_index_writer_config::LiveIndexWriterConfig,
  stored_fields::StoredFields,
  term::Term,
};
use rlucene::core::search::{
  boolean_clause::Occur, boolean_query, index_searcher::IndexSearcher, phrase_query, query::Query,
  similarities_impl::bm25_similarity::BM25Similarity, term_query::TermQuery,
  top_score_doc_collector_manager::TopScoreDocCollectorManager,
};
use rlucene::core::store::mmap_directory::MMapDirectory;
use rlucene::core::util::attribute_source::AttributeSource;
use rlucene::core::util::close::{Closeable, CloseableRef};
use serde::Deserialize;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Deliberately limited to the grammar used by the pinned benchmark query set.
/// Unknown Lucene syntax is rejected, never silently treated as literal terms.
fn parse_query(input: &str, analyzer: &StandardAnalyzer) -> Result<Query> {
  let bytes = input.as_bytes();
  let mut pos = 0;
  let mut clauses = Vec::new();
  while pos < bytes.len() {
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
      pos += 1;
    }
    if pos == bytes.len() {
      break;
    }
    let occur = match bytes[pos] {
      b'+' => {
        pos += 1;
        Occur::Must
      },
      b'-' => {
        pos += 1;
        Occur::MustNot
      },
      _ => Occur::Should,
    };
    if pos == bytes.len() {
      return Err("missing clause after modifier".into());
    }
    let quoted = bytes[pos] == b'"';
    if quoted {
      pos += 1;
    }
    let start = pos;
    if quoted {
      while pos < bytes.len() && bytes[pos] != b'"' {
        pos += 1;
      }
      if pos == bytes.len() {
        return Err("unclosed phrase".into());
      }
    } else {
      while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
        pos += 1;
      }
    }
    let text = &input[start..pos];
    if text.is_empty()
      || !text
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || (quoted && c.is_ascii_whitespace()))
    {
      return Err(format!("unsupported query syntax: {input:?}").into());
    }
    if !quoted && matches!(text, "AND" | "OR" | "NOT") {
      return Err("keyword operators are outside this benchmark grammar".into());
    }
    if quoted {
      pos += 1;
      if pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
        return Err("expected whitespace after phrase".into());
      }
    }
    let mut stream = analyzer.token_stream("text", ReaderEnum::from(text))?;
    stream.reset()?;
    let mut terms = Vec::new();
    let mut position: i32 = -1;
    while stream.increment_token()? {
      let attrs = stream.get_attribute_source();
      position += attrs.get_position_increment()?;
      terms.push((
        Term::from_text("text", attrs.to_string()),
        position as usize,
      ));
    }
    stream.end()?;
    stream.close()?;
    if terms.is_empty() {
      continue;
    }
    let query: Query = if terms.len() == 1 {
      TermQuery::new(terms.remove(0).0).into()
    } else if quoted {
      let mut builder = phrase_query::Builder::new();
      for (term, position) in terms {
        builder.add(term, position)?;
      }
      builder.build()?.into()
    } else {
      let mut builder = boolean_query::Builder::new();
      for (term, _) in terms {
        builder.add(TermQuery::new(term), Occur::Should)?;
      }
      builder.build().into()
    };
    clauses.push((query, occur));
  }
  // Classic QueryParser unwraps a single non-prohibited clause.
  if clauses.len() == 1 && clauses[0].1 != Occur::MustNot {
    return Ok(clauses.remove(0).0);
  }
  let mut builder = boolean_query::Builder::new();
  for (query, occur) in clauses {
    builder.add(query, occur)?;
  }
  Ok(builder.build().into())
}

#[derive(Deserialize)]
struct InputDocument {
  id: String,
  text: String,
  #[serde(default)]
  sort_field: i64,
}

fn index(path: &Path) -> Result<()> {
  let directory = Arc::new(MMapDirectory::new(path)?);
  let mut config = IndexWriterConfig::with_analyzer(StandardAnalyzer::new())?;
  config
    .set_open_mode(OpenMode::Create)
    .set_ram_buffer_size_mb(1024.0);
  let writer = IndexWriter::new(directory.clone(), config)?;
  let mut indexed = 0usize;
  for line in io::stdin().lock().lines() {
    let line = line?;
    if line.trim().is_empty() {
      continue;
    }
    let input: InputDocument = serde_json::from_str(&line)?;
    let mut doc = Document::new();
    doc.add(StoredField::from_string("id", input.id)?);
    doc.add(TextField::from_string("text", input.text, Store::No)?);
    doc.add(NumericDocValuesField::new("sort_field", input.sort_field));
    writer.add_document(doc)?;
    indexed += 1;
    if indexed.is_multiple_of(100_000) {
      eprintln!("Indexed: {indexed}");
    }
  }
  writer.force_merge(1)?;
  writer.close()?;
  directory.close()?;
  eprintln!("Indexed {indexed} documents; merged to one segment");
  Ok(())
}

fn serve(path: &Path, verify: bool) -> Result<()> {
  let directory = Arc::new(MMapDirectory::new(path)?);
  let reader = Arc::new(directory_reader::open(directory.clone())?);
  let mut searcher = IndexSearcher::new(reader.clone().get_context()?)?;
  searcher.set_query_cache(None);
  searcher.set_similarity(BM25Similarity::with_k1_b(1.2, 0.75)?);
  eprintln!(
    "INDEX max_doc={} num_docs={} deletions={}",
    reader.max_doc()?,
    reader.num_docs()?,
    reader.has_deletions()?
  );
  let analyzer = StandardAnalyzer::new();
  let mut output = io::BufWriter::new(io::stdout().lock());
  for line in io::stdin().lock().lines() {
    let line = line?;
    let (command, text) = line.split_once('\t').ok_or("expected COMMAND<TAB>QUERY")?;
    let (k, exact) = match command {
      "COUNT" | "UNOPTIMIZED_COUNT" => (0, true),
      "TOP_10" => (10, false),
      "TOP_100" => (100, false),
      "TOP_1000" => (1000, false),
      "TOP_10_COUNT" => (10, true),
      "TOP_100_COUNT" => (100, true),
      "TOP_1000_COUNT" => (1000, true),
      _ => {
        writeln!(output, "UNSUPPORTED")?;
        output.flush()?;
        continue;
      },
    };
    let query = parse_query(text, &analyzer)?;
    if k == 0 {
      let count = searcher.count(query)?;
      if verify {
        writeln!(output, "{}", serde_json::json!({"count": count}))?;
      } else {
        writeln!(output, "{count}")?;
      }
    } else {
      let threshold = if exact { i32::MAX as usize } else { k };
      let hits = searcher
        .search_with_collector_manager(query, &TopScoreDocCollectorManager::new(k, threshold)?)?;
      if verify {
        let mut stored = searcher.stored_fields()?;
        let mut docs = Vec::new();
        for hit in hits.score_docs {
          let doc = stored.document(hit.doc)?;
          let id = doc
            .get("id")?
            .ok_or("missing stored document id")?
            .into_owned();
          docs
            .push(serde_json::json!({"id": id, "doc": hit.doc, "score_bits": hit.score.to_bits()}));
        }
        writeln!(
          output,
          "{}",
          serde_json::json!({"count": hits.total_hits.value(), "relation": format!("{:?}", hits.total_hits.relation()), "docs": docs})
        )?;
      } else {
        writeln!(
          output,
          "{}",
          if exact { hits.total_hits.value() } else { 1 }
        )?;
      }
    }
    output.flush()?;
  }
  reader.close()?;
  directory.close()?;
  Ok(())
}

fn main() -> Result<()> {
  let args: Vec<_> = std::env::args().collect();
  if args.len() != 3 {
    return Err("usage: search-benchmark-rlucene <index|serve|verify> <index-dir>".into());
  }
  match args[1].as_str() {
    "index" => index(Path::new(&args[2])),
    "serve" => serve(Path::new(&args[2]), false),
    "verify" => serve(Path::new(&args[2]), true),
    _ => Err("unknown mode".into()),
  }
}

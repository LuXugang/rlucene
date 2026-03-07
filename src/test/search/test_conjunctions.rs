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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::similarities_impl::raw_tf_similarity::RawTFSimilarity;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::DefaultIndexSearch;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_log_merge_policy, new_searcher_with_reader,
    random,
};
use rand::Rng;

#[allow(dead_code)] // for quick search
pub struct TestConjunctions;

const F1: &str = "title";
const F2: &str = "body";

fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearch> {
    let dir = new_directory_shared(random)?;
    // TODO: 未实现MockAnalyzer/newLogMergePolicy
    let mut config = new_index_writer_config(random);
    config.set_merge_policy(new_log_merge_policy(random)?);
    let w = IndexWriter::new(dir.clone(), config)?;

    w.add_document(doc(
        "lucene",
        "lucene is a very popular search engine library",
    )?)?;
    w.add_document(doc(
        "solr",
        "solr is a very popular search server and is using lucene",
    )?)?;
    w.add_document(doc(
        "nutch",
        "nutch is an internet search engine with web crawler and is using lucene and hadoop",
    )?)?;

    let reader = directory_reader_util::open_with_writer(&w)?;
    w.close()?;

    let mut searcher = new_searcher_with_reader(reader)?;
    searcher.set_similarity(RawTFSimilarity::default());

    Ok(searcher)
}
fn doc(v1: &str, v2: &str) -> Result<Document> {
    let mut doc = Document::new();
    doc.add(StringField::from_string(F1, v1, Store::Yes)?);
    doc.add(TextField::from_string(F2, v2, Store::Yes)?);
    Ok(doc)
}
#[test]
fn test_term_conjunctions_with_omit_tf() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let mut builder = Builder::new();
    builder
        .add(TermQuery::new(Term::from_text(F1, "nutch")), Occur::Must)?
        .add(TermQuery::new(Term::from_text(F2, "is")), Occur::Must)?;
    let query = builder.build();

    let top_docs = searcher.search(query, 3)?;
    assert_eq!(1, top_docs.total_hits().value());
    assert!(
        (top_docs.score_docs()[0].score - 3.0).abs() < 0.001,
        "expected score 3.0, got {}",
        top_docs.score_docs()[0].score
    );

    Ok(())
}
#[test]
fn test_scorer_get_children() -> Result<()> {
    // TODO
    Ok(())
}

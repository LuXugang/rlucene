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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::test_boolean_min_should_match::Callback;
use rand::{Rng, RngExt};

#[allow(dead_code)] // for quick search
pub struct TestBoolean2;
// const FIELD: &str = "field";
// const NUM_EXTRA_DOCS: usize = 6000;
// const DOC_FIELDS: [&str; 4] = [
//     "w1 w2 w3 w4 w5",
//     "w1 w3 w2 w3",
//     "w1 xx w2 yy w3",
//     "w1 w3 xx w2 yy mm",
// ];
// fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<()> {
//     let num_filler_docs = if random.random_bool(0.5) { 0 } else { SIZE };
//     let pre_filler_docs = TestUtil::next_usize(random, 0, num_filler_docs / 2);
//
//     if cfg!(feature = "test_log_verbose") {
//         println!(
//             "TEST: num_filler_docs={} pre_filler_docs={}",
//             num_filler_docs, pre_filler_docs
//         );
//     }
//
//     let directory = if num_filler_docs * pre_filler_docs > 100000 {
//         new_fs_directory(random, create_temp_dir()?)?
//     } else {
//         new_directory_shared(random)?
//     };
//
//     let analyzer = MockAnalyzer::new(random);
//     let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
//     iwc.set_merge_policy(new_log_merge_policy(random)?);
//     let writer = RandomIndexWriter::with_config(random, directory.clone(), iwc);
//     let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
//     ft.set_omit_norms(true)?;
//
//     let mut doc = Document::new();
//     for _ in 0..pre_filler_docs {
//         writer.add_document(doc.clone())?;
//     }
//     let mut field_types = HashMap::new();
//
//     for i in 0..DOC_FIELDS.len() {
//         doc.add(new_field(
//             random,
//             FIELD,
//             DOC_FIELDS[i],
//             &ft,
//             &mut field_types,
//         )?);
//         writer.add_document(doc.clone())?;
//
//         doc = Document::new();
//         for _ in 0..num_filler_docs {
//             writer.add_document(doc.clone())?;
//         }
//     }
//
//     writer.close()?;
//     let little_reader = directory_reader_util::open(directory.clone())?;
//     let mut searcher = new_searcher_with_reader(little_reader)?;
//     // this is intentionally using the baseline sim, because it compares against bigSearcher (which
//     // uses a random one)
//     searcher.set_similarity(ClassicSimilarity::new());
//
//     // make a copy of our index using a single segment
//     let single_segment_directory = if num_filler_docs * pre_filler_docs > 100000 {
//         new_fs_directory(random, create_temp_dir()?)?
//     } else {
//         new_directory_shared(random)?
//     };
//
//     // TODO: this test does not need to be doing this crazy stuff. please improve it!
//     for file_name in directory.list_all()? {
//         if file_name.starts_with("extra") {
//             continue;
//         }
//         single_segment_directory.copy_from(
//             directory.as_ref(),
//             &file_name,
//             &file_name,
//             &IOContext::default_io_context()?,
//         )?;
//         single_segment_directory.sync(&[file_name])?;
//     }
//
//     let analyzer = MockAnalyzer::new(random);
//     let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
//     // we need docID order to be preserved:
//     // randomized codecs are sometimes too costly for this test:
//     iwc.set_merge_policy(new_log_merge_policy(random)?);
//     {
//         let w = IndexWriter::new(single_segment_directory.clone(), iwc)?;
//         w.force_merge_with_wait(1, true)?;
//         w.close()?;
//     }
//
//     let single_segment_reader = directory_reader_util::open(single_segment_directory.clone())?;
//     let mut single_segment_searcher = new_searcher_with_reader(single_segment_reader)?;
//     single_segment_searcher.set_similarity(searcher.get_similarity());
//
//     let dir2 = copy_of(random, directory.as_ref())?;
//
//     // First multiply small test index:
//     let mut mul_factor = 1;
//     let mut doc_count = 0;
//
//     if cfg!(feature = "test_log_verbose") {
//         println!("\nTEST: now copy index...");
//     }
//
//     loop {
//         let _copy = copy_of(random, dir2.as_ref())?;
//
//         let analyzer = MockAnalyzer::new(random);
//         let iwc = new_index_writer_config_with_analyzer(random, analyzer);
//         let w = RandomIndexWriter::with_config(random, dir2.clone(), iwc);
//         // w.add_indexes(vec![copy.clone()])?;
//         doc_count = w.get_doc_stats()?.max_doc as usize;
//         w.close()?;
//         mul_factor *= 2;
//
//         if doc_count >= 3000 * num_filler_docs {
//             break;
//         }
//     }
//
//     let analyzer = MockAnalyzer::new(random);
//     let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
//     iwc.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));
//     // randomized codecs are sometimes too costly for this test:
//     let w = RandomIndexWriter::with_config(random, dir2.clone(), iwc);
//
//     doc = Document::new();
//     doc.add(new_field(random, "field2", "xxx", &ft, &mut field_types)?);
//     for _ in 0..(NUM_EXTRA_DOCS / 2) {
//         w.add_document(doc.clone())?;
//     }
//
//     doc = Document::new();
//     doc.add(new_field(
//         random,
//         "field2",
//         "big bad bug",
//         &ft,
//         &mut field_types,
//     )?);
//     for _ in 0..(NUM_EXTRA_DOCS / 2) {
//         w.add_document(doc.clone())?;
//     }
//
//     let reader = w.get_reader()?;
//     let _big_searcher = new_searcher_with_reader(reader)?;
//     w.close()?;
//
//     Ok(())
// }
// fn copy_of<R: Rng + ?Sized, D>(random: &mut R, dir: &D) -> Result<Arc<DirEnum>>
// where
//     D: Directory,
// {
//     let copy = new_fs_directory(random, create_temp_dir()?)?;
//
//     for name in dir.list_all()? {
//         if name.starts_with("extra") {
//             continue;
//         }
//         copy.copy_from(dir, &name, &name, &IOContext::default_io_context()?)?;
//         copy.sync(&[name])?;
//     }
//     Ok(copy)
// }
pub(crate) fn rand_bool_query<R: Rng + ?Sized, C: Callback>(
    rnd: &mut R,
    allow_must: bool,
    level: i32,
    field: &str,
    vals: &[String],
    cb: Option<&C>,
) -> Result<Builder> {
    let mut current = Builder::new();

    for _ in 0..(rnd.random_range(0..vals.len()) + 1) {
        let mut q_type = 0;
        if level > 0 {
            q_type = rnd.random_range(0..10);
        }

        let q: Query = if q_type < 3 {
            TermQuery::new(Term::from_text(
                field,
                &vals[rnd.random_range(0..vals.len())],
            ))
            .into()
        } else if q_type < 4 {
            let t1 = &vals[rnd.random_range(0..vals.len())];
            let t2 = &vals[rnd.random_range(0..vals.len())];
            PhraseQuery::from_terms(10, field, &[t1.as_str(), t2.as_str()])?.into()
        } else if q_type < 7 {
            WildcardQuery::new(Term::from_text(field, "w*"))?.into()
        } else {
            rand_bool_query(rnd, allow_must, level - 1, field, vals, cb)?
                .build()
                .into()
        };

        let r = rnd.random_range(0..10);
        let occur = if r < 2 {
            Occur::MustNot
        } else if r < 5 {
            if allow_must {
                Occur::Must
            } else {
                Occur::Should
            }
        } else {
            Occur::Should
        };

        current.add(q, occur)?;
    }

    if let Some(cb) = cb {
        cb.post_create(rnd, &mut current)?;
    }

    Ok(current)
}

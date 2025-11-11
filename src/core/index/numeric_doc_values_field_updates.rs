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
use parking_lot::Mutex;
use std::borrow::Cow;
use std::sync::Arc;

use crate::core::index::BytesRef;
use crate::core::index::doc_values_field_updates::{
    AbstractIterator, AbstractIteratorBase, DocValuesFieldInnerIter, DocValuesFieldIterator,
    DocValuesFieldIteratorEnum, DocValuesFieldUpdatesBase, PAGE_SIZE,
    SingleValueDocValuesFieldUpdatesBase,
};
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use crate::core::util::packed::abstract_paged_mutable::{
    AbstractPagedMutable, AbstractPagedMutableBaseEnum,
};
use crate::core::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::core::util::packed::paged_mutable::PagedMutable;

/// A `DocValuesFieldUpdates` which holds updates of documents, of a single `NumericDocValuesField`.
pub(crate) struct NumericDocValuesFieldUpdates {
    values: AbstractPagedMutable<AbstractPagedMutableBaseEnum>,
    min_value: i64,
    lock: Mutex<()>,

    values_iter: Option<Arc<AbstractPagedMutable<AbstractPagedMutableBaseEnum>>>,
}
impl NumericDocValuesFieldUpdates {
    pub(crate) fn new() -> Result<NumericDocValuesFieldUpdates> {
        let sub_reader = AbstractPagedMutableBaseEnum::GrowableWriter(
            PagedGrowableWriter::with_fill_page(1, PackedInts::DEFAULT),
        );
        let values = AbstractPagedMutable::new(1, PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value: 0,
            lock: Mutex::new(()),
            values_iter: None,
        })
    }
    pub(crate) fn with_range(
        min_value: i64,
        max_value: i64,
    ) -> Result<NumericDocValuesFieldUpdates> {
        let bits_per_value = PackedInts::unsigned_bits_required(max_value - min_value);
        let sub_reader = AbstractPagedMutableBaseEnum::Mutable(PagedMutable::with_overhead_ratio(
            PAGE_SIZE,
            bits_per_value,
            PackedInts::DEFAULT,
        ));
        let values = AbstractPagedMutable::new(1, PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value,
            lock: Mutex::new(()),
            values_iter: None,
        })
    }
}

impl DocValuesFieldUpdatesBase for NumericDocValuesFieldUpdates {
    fn finish(&mut self) {
        self.values_iter = Some(Arc::new(std::mem::take(&mut self.values)));
    }

    fn add_value(&mut self, _doc: i32, value: i64, index: i32) -> Result<()> {
        let _guard = self.lock.lock();
        self.values.set(index as i64, value - self.min_value);
        Ok(())
    }

    fn add_byte_ref(&mut self, _doc: i32, _value: &BytesRef<Vec<u8>>, _index: i32) -> Result<()> {
        Err(LuceneError::unreachable(
            "numericDocValuesFieldUpdates does not support add_byte_ref",
        ))
    }

    fn add_iterator<I: DocValuesFieldIterator>(
        &mut self,
        doc_id: i32,
        iterator: &mut I,
    ) -> Result<()> {
        self.add_value(doc_id, iterator.long_value()?, 0)
    }

    fn iterator(
        &self,
        inner: DocValuesFieldInnerIter,
        del_gen: i64,
    ) -> Result<DocValuesFieldIteratorEnum> {
        debug_assert!(self.values_iter.is_some());
        let base = AbstractIteratorNumeric::new(
            self.values_iter.as_ref().unwrap().clone(),
            0,
            self.min_value,
        );
        Ok(DocValuesFieldIteratorEnum::AbstractNumeric(
            AbstractIterator::new(inner, del_gen, base),
        ))
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let tmp_val = self.values.get_immutable(j as i64)?;
        let value = self.values.get_immutable(i as i64)?;
        self.values.set(j as i64, value);
        self.values.set(i as i64, tmp_val);
        Ok(())
    }

    fn grow(&mut self, size: i32) -> Result<()> {
        let value_result = self.values.grow_with_size(size as i64)?;
        if let Some(values) = value_result {
            self.values = values;
        }
        Ok(())
    }

    fn resize(&mut self, size: i32) -> Result<()> {
        self.values = self.values.resize(size as i64)?;
        Ok(())
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}

impl Accountable for NumericDocValuesFieldUpdates {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
pub(crate) struct AbstractIteratorNumeric {
    values: Arc<AbstractPagedMutable<AbstractPagedMutableBaseEnum>>,
    value: i64,
    min_value: i64,
}
impl AbstractIteratorNumeric {
    pub(crate) fn new(
        values: Arc<AbstractPagedMutable<AbstractPagedMutableBaseEnum>>,
        value: i64,
        min_value: i64,
    ) -> Self {
        AbstractIteratorNumeric {
            values,
            value,
            min_value,
        }
    }
}
impl AbstractIteratorBase for AbstractIteratorNumeric {
    fn set(&mut self, idx: i64) -> Result<()> {
        self.value = self.values.get_immutable(idx)? + self.min_value;
        Ok(())
    }

    fn long_value(&self) -> Result<i64> {
        Ok(self.value)
    }

    fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        unreachable!("NumericDocValuesFieldUpdatesIterator does not support binary_value")
    }
}

#[derive(Default)]
pub struct SingleValueNumericDocValuesFieldUpdates {
    value: i64,
}
impl SingleValueNumericDocValuesFieldUpdates {
    pub(crate) fn new(value: i64) -> SingleValueNumericDocValuesFieldUpdates {
        SingleValueNumericDocValuesFieldUpdates { value }
    }
}
impl SingleValueDocValuesFieldUpdatesBase for SingleValueNumericDocValuesFieldUpdates {
    fn binary_value(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        Err(LuceneError::unreachable(
            "SingleValueNumericDocValuesFieldUpdates does not support binary_value",
        ))
    }

    fn long_value(&self) -> Result<i64> {
        Ok(self.value)
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
    use crate::core::document::string_field::StringField;
    use crate::core::index::composite_reader::get_context;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_reader_context::IndexReaderContext;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
    use crate::core::index::leaf_reader::LeafReader;
    use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
    use crate::core::index::numeric_doc_values::NumericDocValues;
    use crate::core::index::sort::Sort;
    use crate::core::index::term::Term;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::sort_field::{SortField, SortFieldType};
    use crate::core::search::term_query::TermQuery;
    use crate::core::search::top_docs::TopDocsLike;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory, new_index_writer_config, random,
    };
    use rand::Rng;
    use std::sync::Arc;
    #[allow(dead_code)]
    struct TestNumericDocValuesUpdates;
    fn doc(id: i32) -> Result<Document> {
        // make sure we don't set the doc's value to 0, to not confuse with a document that's missing values
        doc_with_val(id, (id + 1) as i64)
    }

    fn doc_with_val(id: i32, val: i64) -> Result<Document> {
        let mut doc = Document::new();
        doc.add(StringField::with_string(
            "id",
            format!("doc-{}", id),
            Store::No,
        )?);
        doc.add(NumericDocValuesField::new("val", val));
        Ok(doc)
    }
    #[test]
    fn test_multiple_updates_same_doc() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现MockAnalyzer
        let mut config = new_index_writer_config(&mut random);
        config.set_max_buffered_docs(3); // small number of docs
        let mut writer = IndexWriter::new(dir.clone(), config)?;

        writer.update_documents_with_term(
            Term::from_text("id", "doc-1"),
            doc_with_val(1, 1_000_000_000)?,
        )?;
        writer.update_numeric_doc_value(Term::from_text("id", "doc-1"), "val", 1_000_001_111)?;
        writer.update_documents_with_term(
            Term::from_text("id", "doc-2"),
            doc_with_val(2, 2_000_000_000)?,
        )?;
        writer.update_documents_with_term(
            Term::from_text("id", "doc-2"),
            doc_with_val(2, 2_222_222_222)?,
        )?;
        writer.update_numeric_doc_value(Term::from_text("id", "doc-1"), "val", 1_111_111_111)?;

        let reader = if random.random_bool(0.5) {
            writer.commit()?;
            directory_reader_util::open(dir.clone())?
        } else {
            directory_reader_util::open_with_writer(&mut writer)?
        };
        let reader = get_context(Arc::new(reader))?;
        let mut searcher = IndexSearcher::new(reader)?;

        let td = searcher.search_with_sort(
            TermQuery::new(Term::from_text("id", "doc-1")),
            1,
            Sort::with_fields(vec![
                SortField::new(Some("val"), SortFieldType::Long)?.into(),
            ])?,
        )?;
        assert_eq!(td.score_docs().len(), 1, "doc-1 missing?");
        assert_eq!(
            *td.base.score_docs[0].fields()?[0].as_i64().unwrap(),
            1_111_111_111,
            "doc-1 value mismatch"
        );

        let td = searcher.search_with_sort(
            TermQuery::new(Term::from_text("id", "doc-2")),
            1,
            Sort::with_fields(vec![
                SortField::new(Some("val"), SortFieldType::Long)?.into(),
            ])?,
        )?;
        assert_eq!(td.score_docs().len(), 1, "doc-2 missing?");
        assert_eq!(
            *td.base.score_docs[0].fields()?[0].as_i64().unwrap(),
            2_222_222_222,
            "doc-2 value mismatch"
        );

        writer.close()?;
        Ok(())
    }

    #[test]
    fn test_simple() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现 MockAnalyzer
        let mut config = new_index_writer_config(&mut random);
        // make sure random config doesn't flush on us
        config.set_max_buffered_docs(10);
        config.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
        let mut writer = IndexWriter::new(dir.clone(), config)?;

        writer.add_document(doc(0)?)?; // val=1
        writer.add_document(doc(1)?)?; // val=2
        if random.random_bool(0.5) {
            // randomly commit before the update is sent
            writer.commit()?;
        }

        writer.update_numeric_doc_value(Term::from_text("id", "doc-0"), "val", 2)?;

        let reader = if random.random_bool(0.5) {
            writer.close()?;
            directory_reader_util::open(dir.clone())?
        } else {
            let r = directory_reader_util::open_with_writer(&mut writer)?;
            writer.close()?;
            r
        };

        let reader = get_context(Arc::new(reader))?;
        assert_eq!(reader.leaves()?.len(), 1);
        let r = reader.leaves()?[0].reader();
        let mut ndv = r.get_numeric_doc_values("val")?.unwrap();
        assert_eq!(ndv.next_doc()?, 0);
        assert_eq!(ndv.long_value()?, 2);
        assert_eq!(ndv.next_doc()?, 1);
        assert_eq!(ndv.long_value()?, 2);

        Ok(())
    }
}

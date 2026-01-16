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
use crate::core::index::fields::Fields;
use crate::core::index::multi_terms::MultiTerms;
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::merged_iterator::MergedIterator;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Provides a single [`Fields`] term index view over an [`IndexReader`](crate::core::index::index_reader::IndexReader).
///
/// This is useful when interacting with an [`IndexReader`](crate::core::index::index_reader::IndexReader) implementation that consists of
/// sequential sub-readers (for example, `DirectoryReader` or `MultiReader`) and you must treat it
/// as a [`LeafReader`](crate::core::index::leaf_reader::LeafReader).
///
/// **NOTE**: For composite readers, you will generally get better performance by gathering the
/// sub-readers via `IndexReader::get_context()` to obtain the atomic leaves and then operating
/// per-`LeafReader`, instead of using this type.
pub struct MultiFields<F>
where
    F: Fields,
{
    pub(crate) subs: Vec<F>,
    sub_slices: Vec<Rc<ReaderSlice>>,
    terms: RefCell<HashMap<String, Rc<TermsType<F>>>>,
}
pub type TermsType<F> = MultiTerms<<F as Fields>::Terms>;
impl<F> MultiFields<F>
where
    F: Fields,
{
    /// Sole constructor.
    pub fn new(subs: Vec<F>, sub_slices: Vec<Rc<ReaderSlice>>) -> Self {
        Self {
            subs,
            sub_slices,
            terms: RefCell::new(HashMap::new()),
        }
    }
}
pub type MultiFieldsTerms<T> = Rc<MultiTerms<T>>;
impl<F> Fields for MultiFields<F>
where
    F: Fields,
{
    type FieldIter<'a>
        = MergedIterator<<F as Fields>::FieldIter<'a>>
    where
        Self: 'a;

    fn iterator(&self) -> Result<Self::FieldIter<'_>> {
        let mut sub_iterators = Vec::new();
        for sub in &self.subs {
            sub_iterators.push(sub.iterator()?);
        }
        MergedIterator::new(sub_iterators)
    }

    type Terms = MultiFieldsTerms<<F as Fields>::Terms>;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        if let Some(v) = self.terms.borrow().get(field) {
            return Ok(Some(v.clone()));
        }

        // Lazy init: first time this field is requested
        let mut subs2 = Vec::new();
        let mut slices2 = Vec::new();
        // Gather all sub-readers that share this field
        for i in 0..self.subs.len() {
            if let Some(terms) = self.subs[i].terms(field)? {
                subs2.push(terms);
                slices2.push(self.sub_slices[i].clone());
            }
        }

        if !subs2.is_empty() {
            let result = Rc::new(MultiTerms::new(subs2, slices2)?);
            self.terms
                .borrow_mut()
                .insert(field.to_string(), result.clone());
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn size(&self) -> Result<i32> {
        Ok(-1)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store::No;
    use crate::core::document::field_type::FieldType;
    use crate::core::index::BytesRef;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::multi_terms::get_term_postings_enum;
    use crate::core::index::postings_enum::{FREQS, NONE};
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_index_writer_config, new_string_field, random,
    };
    use crate::test::util::test_util::TestUtil;
    use std::collections::HashMap;

    #[allow(dead_code)] // for quick search
    struct TestMultiFields;

    #[test]
    fn test_random() -> Result<()> {
        // TODO keepFullyDeletedSegment  未实现
        Ok(())
    }

    #[test]
    fn test_separate_enums() -> Result<()> {
        let mut random = random();
        // TODO: 未实现 MockAnalyzer
        let dir = new_directory_shared(&mut random)?;
        let iw = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

        let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
        let mut doc = Document::new();
        doc.add(new_string_field("f", "j", No, &mut field_to_type)?);

        iw.add_document(doc.clone())?;
        iw.commit()?;
        iw.add_document(doc)?;

        let reader = directory_reader_util::open_with_writer(&iw)?;
        iw.close()?;

        let mut d1 = TestUtil::docs_with_reader(
            &mut random,
            &reader,
            "f",
            &BytesRef::from_string("j"),
            None,
            NONE as i32,
        )?
        .unwrap();

        let mut d2 = TestUtil::docs_with_reader(
            &mut random,
            &reader,
            "f",
            &BytesRef::from_string("j"),
            None,
            NONE as i32,
        )?
        .unwrap();

        assert_eq!(0, d1.next_doc()?);
        assert_eq!(0, d2.next_doc()?);

        Ok(())
    }

    #[test]
    fn test_term_docs_enum() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO: 未实现 MockAnalyzer
        let iw = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
        let mut doc = Document::new();
        doc.add(new_string_field("f", "j", No, &mut field_to_type)?);
        iw.add_document(doc.clone())?;
        iw.commit()?;
        iw.add_document(doc)?;

        let reader = directory_reader_util::open_with_writer(&iw)?;
        iw.close()?;

        let mut de =
            get_term_postings_enum(&reader, "f", &BytesRef::from_string("j"), FREQS as i32)?
                .unwrap();

        assert_eq!(0, de.next_doc()?);
        assert_eq!(1, de.next_doc()?);
        assert_eq!(NO_MORE_DOCS, de.next_doc()?);
        Ok(())
    }
}

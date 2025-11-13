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
use crate::core::search::sort_field::{SortField, SortFiledBase};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;
/// Encapsulates sort criteria for returned hits.
///
/// A [`Sort`] can be created with an empty constructor, yielding an object
/// that instructs searches to return hits sorted by relevance; or it can be
/// created with one or more [`SortField`]s.
///
/// See also: [`SortField`].
#[derive(Clone)]
pub struct Sort {
    pub(crate) fields: Vec<SortFieldEnum>,
}

impl Sort {
    /// Represents sorting by index order.
    pub fn get_index_order() -> Result<Self> {
        let sort_field = SortFieldEnum::Sorter(SortField::get_field_doc()?);
        Self::with_fields(vec![sort_field])
    }
    /// Represents sorting by computed relevance. Using this sort criteria returns the same results as
    /// calling [`IndexSearcher::search(Query, i32)`](crate::core::search::index_searcher::IndexSearcher::search) without a sort criteria,
    /// only with slightly more overhead.
    pub fn get_relevance() -> Result<Self> {
        Self::new()
    }
    /// Returns true if the relevance score is needed to sort documents.
    pub fn needs_scores(&self) -> bool {
        for sort_field in &self.fields {
            if sort_field.needs_scores() {
                return true;
            }
        }
        false
    }
}

impl Sort {
    /// Sorts by computed relevance.
    ///
    /// This is the same sort criteria as calling `IndexSearcher::search`
    /// without a sort criteria, only with slightly more overhead.
    pub fn new() -> Result<Self> {
        let sort_field = SortFieldEnum::Sorter(SortField::get_field_score()?);
        Self::with_fields(vec![sort_field])
    }

    /// Sets the sort to the given criteria in succession.
    ///
    /// The first `SortField` is checked first, but if it produces a tie, then
    /// the second `SortField` is used to break the tie, and so on. Finally,
    /// if there is still a tie after all `SortField`s are checked, the
    /// internal Lucene doc ID is used to break it.
    ///
    /// # Arguments
    /// - `fields`: A vector of `SortField` to define the sorting order.
    ///
    /// # Errors
    /// Returns an error if the provided `fields` vector is empty.
    /// # Note
    /// You could use
    /// [`push`](crate::core::search::sort_field_enum::SortFieldVecExt::push_iterm)
    /// to init SortFieldEnum vector. # Example
    /// ```rust
    /// use rlucene::core::index::sort::Sort;
    /// use rlucene::core::search::sort_field::{SortField, SortFieldType};
    /// use rlucene::core::search::sort_field_enum::SortFieldVecExt;
    /// use rlucene::core::search::sorted_numeric_sort_field::SortedNumericSortField;
    /// use rlucene::core::search::sorted_set_sort_field::SortedSetSortField;
    /// let sort_field1 = SortField::new(Some("field1"), SortFieldType::Custom).unwrap();
    /// let sort_field2 = SortedSetSortField::new("field2", false).unwrap();
    /// let mut fileds = Vec::new();
    /// fileds.push_iterm(sort_field1);
    /// fileds.push_iterm(sort_field2);
    /// let sort = Sort::with_fields(fileds);
    /// assert!(sort.is_ok());
    /// ```
    pub fn with_fields(fields: Vec<SortFieldEnum>) -> Result<Self> {
        if fields.is_empty() {
            Err(LuceneError::illegal_argument(
                "There must be at least 1 sort field".to_string(),
            ))
        } else {
            Ok(Self { fields })
        }
    }

    /// Representation of the sort criteria.
    ///
    /// # Returns
    /// Array (Vec) of `SortField` objects used in this sort criteria.
    pub fn get_sort(&self) -> &[SortFieldEnum] {
        &self.fields
    }
    pub fn take_sort(&mut self) -> Vec<SortFieldEnum> {
        std::mem::take(&mut self.fields)
    }
}

impl Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fields_string = self
            .fields
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{fields_string}")
    }
}
impl PartialEq for Sort {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}
impl Eq for Sort {}

impl Hash for Sort {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fields.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
    use crate::core::document::string_field::StringField;

    use crate::core::index::sort::Sort;
    use crate::core::index::stored_fields::StoredFields;
    use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
    use crate::core::search::sort_field::MissingValueEnum::StringFirst;
    use crate::core::search::sort_field::{SortField, SortFieldType, SortFiledBase};
    use crate::core::search::top_docs::TopDocsLike;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_bytes_ref_from_string, new_directory, new_searcher_with_reader, random,
    };
    use std::hash::DefaultHasher;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestSort;
    fn assert_equals_sort(a: &Sort, b: &Sort) {
        assert!(a == b);
        assert!(b == a);

        use std::hash::{Hash, Hasher};
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }
    fn assert_different_sort(a: &Sort, b: &Sort) {
        assert!(a != b);
        assert!(b != a);

        use std::hash::{Hash, Hasher};
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_ne!(ha.finish(), hb.finish());
    }
    #[test]
    fn test_equals() -> Result<()> {
        let sort_field1 = SortField::new("foo".into(), SortFieldType::String)?;

        let mut sort_field2 = SortField::new("foo".into(), SortFieldType::String)?;
        assert_equals_sort(
            &Sort::with_fields(vec![sort_field1.clone().into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        sort_field2 = SortField::new("bar".into(), SortFieldType::String)?;
        assert_different_sort(
            &Sort::with_fields(vec![sort_field1.clone().into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        sort_field2 = SortField::new("foo".into(), SortFieldType::Long)?;
        assert_different_sort(
            &Sort::with_fields(vec![sort_field1.clone().into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        sort_field2 = SortField::new("foo".into(), SortFieldType::String)?;
        sort_field2.set_missing_value(StringFirst)?;
        assert_different_sort(
            &Sort::with_fields(vec![sort_field1.clone().into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        sort_field2 = SortField::with_reverse("foo".into(), SortFieldType::String, false)?;
        assert_equals_sort(
            &Sort::with_fields(vec![sort_field1.clone().into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        sort_field2 = SortField::with_reverse("foo".into(), SortFieldType::String, true)?;
        assert_different_sort(
            &Sort::with_fields(vec![sort_field1.into()])?,
            &Sort::with_fields(vec![sort_field2.into()])?,
        );

        Ok(())
    }
    /// Tests sorting on type string
    #[test]
    fn test_string() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let writer = RandomIndexWriter::new(&mut random, dir);

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        doc.add(StringField::with_string("value", "foo", Store::Yes)?);
        writer.add_document(doc)?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "bar")?,
        ));
        doc.add(StringField::with_string("value", "bar", Store::Yes)?);
        writer.add_document(doc)?;

        let ir = writer.get_reader()?;
        writer.close()?;

        let mut searcher = new_searcher_with_reader(Arc::new(ir))?;
        let sort = Sort::with_fields(vec![
            SortField::new("value".into(), SortFieldType::String)?.into(),
        ])?;

        let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
        assert_eq!(2, td.total_hits().value);

        // 'bar' comes before 'foo'
        let v0 = searcher
            .stored_fields()?
            .document(td.score_docs()[0].doc())?;
        assert_eq!("bar", v0.get("value")?.unwrap().as_ref());

        let v1 = searcher
            .stored_fields()?
            .document(td.score_docs()[1].doc())?;
        assert_eq!("foo", v1.get("value")?.unwrap().as_ref());

        Ok(())
    }
    /// Tests reverse sorting on type string
    #[test]
    fn test_string_reverse() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let writer = RandomIndexWriter::new(&mut random, dir);

        // doc 1: bar
        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "bar")?,
        ));
        doc.add(StringField::with_string("value", "bar", Store::Yes)?);
        writer.add_document(doc)?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        doc.add(StringField::with_string("value", "foo", Store::Yes)?);
        writer.add_document(doc)?;

        let ir = writer.get_reader()?;
        writer.close()?;

        let mut searcher = new_searcher_with_reader(Arc::new(ir))?;
        let sort = Sort::with_fields(vec![
            SortField::with_reverse("value".into(), SortFieldType::String, true)?.into(),
        ])?;

        let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
        assert_eq!(2, td.total_hits().value);

        // reverse order: foo first, bar second
        let v0 = searcher
            .stored_fields()?
            .document(td.score_docs()[0].doc())?;
        assert_eq!("foo", v0.get("value")?.unwrap().as_ref());

        let v1 = searcher
            .stored_fields()?
            .document(td.score_docs()[1].doc())?;
        assert_eq!("bar", v1.get("value")?.unwrap().as_ref());

        Ok(())
    }
    /// Tests sorting on type string_val
    #[test]
    fn test_string_val() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let writer = RandomIndexWriter::new(&mut random, dir);
        let mut doc = Document::new();
        doc.add(BinaryDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        doc.add(StringField::with_string("value", "foo", Store::Yes)?);
        writer.add_document(doc)?;

        let mut doc = Document::new();
        doc.add(BinaryDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "bar")?,
        ));
        doc.add(StringField::with_string("value", "bar", Store::Yes)?);
        writer.add_document(doc)?;

        let ir = writer.get_reader()?;
        writer.close()?;

        let mut searcher = new_searcher_with_reader(Arc::new(ir))?;
        let sort = Sort::with_fields(vec![
            SortField::new("value".into(), SortFieldType::StringVal)?.into(),
        ])?;

        let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
        assert_eq!(2, td.total_hits().value);

        let v0 = searcher
            .stored_fields()?
            .document(td.score_docs()[0].doc())?;
        assert_eq!("bar", v0.get("value")?.unwrap().as_ref());

        let v1 = searcher
            .stored_fields()?
            .document(td.score_docs()[1].doc())?;
        assert_eq!("foo", v1.get("value")?.unwrap().as_ref());

        Ok(())
    }
    /// Tests reverse sorting on type string_val
    #[test]
    fn test_string_val_reverse() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let writer = RandomIndexWriter::new(&mut random, dir);
        let mut doc = Document::new();
        doc.add(BinaryDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "bar")?,
        ));
        doc.add(StringField::with_string("value", "bar", Store::Yes)?);
        writer.add_document(doc)?;

        let mut doc = Document::new();
        doc.add(BinaryDocValuesField::new(
            "value",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        doc.add(StringField::with_string("value", "foo", Store::Yes)?);
        writer.add_document(doc)?;

        let ir = writer.get_reader()?;
        writer.close()?;

        let mut searcher = new_searcher_with_reader(Arc::new(ir))?;
        let sort = Sort::with_fields(vec![
            SortField::with_reverse("value".into(), SortFieldType::StringVal, true)?.into(),
        ])?;

        let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
        assert_eq!(2, td.total_hits().value);

        // reverse: foo first, bar second
        let v0 = searcher
            .stored_fields()?
            .document(td.score_docs()[0].doc())?;
        assert_eq!("foo", v0.get("value")?.unwrap().as_ref());

        let v1 = searcher
            .stored_fields()?
            .document(td.score_docs()[1].doc())?;
        assert_eq!("bar", v1.get("value")?.unwrap().as_ref());

        Ok(())
    }
}

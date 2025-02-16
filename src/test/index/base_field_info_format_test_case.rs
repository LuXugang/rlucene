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
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::lucene101_codec::Lucene101Codec;
use crate::codecs::Codec;
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::FieldInfos;
use crate::index::index_options::IndexOptions;
use crate::index::segment_info::SegmentInfo;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::test::util::index_package_access::{
    FieldInfosBuilder, IndexPackageAccess, IndexPackageAccessImpl,
};
use crate::test::util::lucene_test_case::new_directory;
use crate::test::util::test_error::TestError;
use crate::util::{StringHelper, LATEST};
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait BaseFieldInfoFormatTestCase {
    fn support_doc_values_skip_index(&self) -> bool {
        true
    }
    fn test_one_field(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let codec = Lucene101Codec;
        let segment_info = Self::new_segment_info(random, dir.clone(), "_123")?;

        let fi = Arc::new(Self::create_field_info());
        Self::add_attributes(&fi);

        let infos = IndexPackageAccessImpl
            .new_field_infos_builder(None, None)?
            .add(fi)?
            .finish()?;

        codec.field_infos_format().write(
            dir.clone(),
            &segment_info,
            "",
            &infos,
            &IOContext::default_io_context()?,
        )?;
        let infos2 = codec.field_infos_format().read(
            dir.clone(),
            &segment_info,
            "",
            &IOContext::default_io_context()?,
        )?;

        assert_eq!(1, infos2.size());
        match infos2.field_info_by_name("field") {
            None => {
                unreachable!("field not found");
            }
            Some(field) => {
                assert_ne!(field.get_index_options(), IndexOptions::None);
                assert_eq!(DocValuesType::None, field.get_doc_values_type());
                assert!(!field.omits_norms());
                assert!(!field.has_payloads());
                assert!(!field.has_term_vectors());
                assert_eq!(0, field.get_point_dimension_count());
                assert_eq!(0, field.get_vector_dimension());
                assert!(!field.is_soft_deletes_field());
            }
        }
        Ok(())
    }
    /// Test field infos attributes coming back are not mutable.
    fn test_immutable_attributes(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // no necessary to implement
        Ok(())
    }

    fn test_exception_on_create_output(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // TODO
        // no necessary to implement
        Ok(())
    }
    fn test_exception_on_close_output(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // TODO
        // no necessary to implement
        Ok(())
    }
    fn test_exception_on_open_input(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // TODO
        // no necessary to implement
        Ok(())
    }
    fn test_exception_on_close_input(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // TODO
        // no necessary to implement
        Ok(())
    }
    /// Hook to add any codec attributes to fieldinfo instances added in this test.
    fn add_attributes(_fi: &FieldInfo) {}
    /// Asserts equality for the entirety of FieldInfos
    fn assert_field_infos_equals(
        expected: &FieldInfos,
        actual: &FieldInfos,
    ) -> Result<(), TestError> {
        // Assert that both FieldInfos have the same size
        assert_eq!(expected.size(), actual.size());

        for expected_field in expected.iter() {
            let actual_field = actual.field_info_by_number(expected_field.number)?;
            match actual_field {
                None => unreachable!("should be Some"),
                Some(actual_field) => {
                    Self::assert_field_info_equals(expected_field, &actual_field);
                }
            }
        }
        Ok(())
    }

    /// Asserts equality for two individual FieldInfo objects
    fn assert_field_info_equals(expected: &FieldInfo, actual: &FieldInfo) {
        assert_eq!(expected.number, actual.number);
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.get_doc_values_type(), actual.get_doc_values_type());
        assert_eq!(
            expected.doc_values_skip_index_type(),
            actual.doc_values_skip_index_type()
        );
        assert_eq!(expected.get_index_options(), actual.get_index_options());

        assert_eq!(expected.has_norms(), actual.has_norms());

        assert_eq!(expected.has_payloads(), actual.has_payloads());

        assert_eq!(expected.has_term_vectors(), actual.has_term_vectors());
        assert_eq!(expected.omits_norms(), actual.omits_norms());
        assert_eq!(expected.get_doc_values_gen(), actual.get_doc_values_gen());
    }

    fn new_segment_info<D: Directory>(
        random: &mut StdRng,
        dir: Arc<Mutex<D>>,
        name: &str,
    ) -> Result<SegmentInfo<D>, TestError> {
        let min_version = if random.gen_bool(0.5) {
            None
        } else {
            Some((*LATEST).clone())
        };
        let id = StringHelper::random_id();
        let value = SegmentInfo::new(
            dir,
            Some((*LATEST).clone()),
            min_version,
            name.to_string(),
            Option::from(10_000),
            false,
            false,
            HashMap::new(),
            Vec::from(id),
            HashMap::new(),
            None,
        )?;
        Ok(value)
    }
    // TODO: addRandomFields()

    fn create_field_info() -> FieldInfo {
        FieldInfo::new(
            "field".to_string(),
            -1,
            false,
            false,
            false,
            IndexOptions::DocsAndFreqsAndPositions,
            DocValuesType::None,
            DocValuesSkipIndexType::None,
            -1,
            Arc::new(Mutex::new(HashMap::new())),
            0,
            0,
            0,
            0,
            VectorEncoding::FLOAT32(4),
            VectorSimilarityFunction::Euclidean,
            false,
            false,
        )
    }
}

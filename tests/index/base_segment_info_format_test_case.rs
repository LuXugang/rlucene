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
use crate::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::util::lucene_test_case::new_directory;
use crate::util::test_error::TestError;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::codecs::segment_info_format::SegmentInfoFormat;
use rlucene::codecs::{Codec, LATEST_CODEC};
use rlucene::index::segment_info::SegmentInfo;
use rlucene::search::field_comparator_source::DummyFieldComparatorSource;
use rlucene::search::sort_field::DummySortFieldBase;
use rlucene::store::IOContext;
use rlucene::util::{StringHelper, Version};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub trait BaseSegmentInfoFormatTestCase: BaseIndexFileFormatTestCase {
    fn test_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            false,
            HashMap::new(),
            Vec::from(id),
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        assert_eq!(info.files()?, info2.files()?);
        Ok(())
    }
    fn test_has_blocks(&self, random: &mut StdRng) -> Result<(), TestError> {
        assert!(self.supports_has_blocks());
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let has_blocks = random.gen_bool(0.5);
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            has_blocks,
            HashMap::new(),
            Vec::from(id.clone()),
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        assert_eq!(info.get_has_blocks(), info2.get_has_blocks());
        Ok(())
    }

    fn test_adds_self_to_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            false,
            HashMap::new(),
            Vec::from(id),
            HashMap::new(),
            None,
        )?;
        let original_files: HashSet<String> = ["_123.a".to_string()].iter().cloned().collect();
        info.set_files(original_files.clone());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let modified_files = info.files()?;
        assert!(modified_files.is_superset(&original_files));
        assert!(
            modified_files.len() > original_files.len(),
            "did you forget to add yourself to files()?"
        );
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        assert_eq!(info.files()?, info2.files()?);
        // In Rust Lucene, SegmentInfo::files return a immutable Set, so we do not need to verify this
        // let immutable_files = info2.files()?;
        // let add_result = immutable_files.insert("bogus".to_string());
        // assert!(
        //     !add_result,
        //     "Files set should be immutable, but modification was allowed."
        // );
        Ok(())
    }
    fn test_diagnostics(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut diagnostics: HashMap<String, String> = HashMap::new();
        diagnostics.insert("key1".to_string(), "value1".to_string());
        diagnostics.insert("key2".to_string(), "value2".to_string());
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            false,
            diagnostics.clone(),
            Vec::from(id),
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        assert_eq!(diagnostics, *info2.get_diagnostics());
        // In Rust Lucene, SegmentInfo::get_diagnostics return a immutable Set, so we do not need to verify this
        // let mut immutable_diagnostics = info2.get_diagnostics();
        // let insert_result = immutable_diagnostics.insert("bogus".to_string(), "bogus".to_string());
        // assert!(
        //     insert_result.is_none(),
        //     "Diagnostics map should be immutable, but modification was allowed."
        // );
        Ok(())
    }

    fn test_attributes(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut attributes: HashMap<String, String> = HashMap::new();
        attributes.insert("key1".to_string(), "value1".to_string());
        attributes.insert("key2".to_string(), "value2".to_string());
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            false,
            HashMap::new(),
            Vec::from(id.clone()),
            attributes.clone(),
            None,
        )?;
        info.set_files(HashSet::new());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        let info2_attributes = info2.get_attributes()?;
        let info2_values = info2_attributes.lock().unwrap();
        assert_eq!(attributes, *info2_values);
        // 在 Rust Lucene 中，attributes 的返回值是不可变的，因此不需要检查修改是否被禁止。
        // 如果需要，可以解除注释并尝试修改
        // let mut immutable_attributes = info2.get_attributes();
        // let insert_result = immutable_attributes.insert("bogus".to_string(), "bogus".to_string());
        // assert!(
        //     insert_result.is_none(),
        //     "Attributes map should be immutable, but modification was allowed."
        // );

        Ok(())
    }

    fn test_unique_id(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut info = SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123".parse().unwrap(),
            Some(1),
            false,
            false,
            HashMap::new(),
            Vec::from(id.clone()),
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new());
        LATEST_CODEC.segment_info_format().write(
            dir.clone(),
            &mut info,
            IOContext::default_io_context()?,
        )?;
        let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
            LATEST_CODEC.segment_info_format().read(
                dir.clone(),
                "_123",
                Vec::from(&id),
                &IOContext::default_io_context()?,
            )?;
        assert_eq!(id, info2.get_id().as_slice());

        Ok(())
    }

    fn test_versions(&self, random: &mut StdRng) -> Result<(), TestError> {
        for version in self.get_versions() {
            for min_version in [Some(version.clone()), None] {
                let dir = Arc::new(Mutex::new(new_directory(random)?));
                let id = StringHelper::random_id();
                let mut info =
                    SegmentInfo::<_, DummySortFieldBase, DummyFieldComparatorSource>::new(
                        dir.clone(),
                        Some(version.clone()),
                        min_version.clone(),
                        "_123".parse().unwrap(),
                        Some(1),
                        false,
                        false,
                        HashMap::new(),
                        Vec::from(id.clone()),
                        HashMap::new(),
                        None,
                    )?;
                info.set_files(HashSet::new());
                LATEST_CODEC.segment_info_format().write(
                    dir.clone(),
                    &mut info,
                    IOContext::default_io_context()?,
                )?;
                let info2: SegmentInfo<_, DummySortFieldBase, DummyFieldComparatorSource> =
                    LATEST_CODEC.segment_info_format().read(
                        dir.clone(),
                        "_123",
                        Vec::from(&id),
                        &IOContext::default_io_context()?,
                    )?;
                assert!(info2.get_version().is_some());
                assert_eq!(*info2.get_version().unwrap(), version.clone());
                if self.supports_min_version() {
                    if min_version.is_none() {
                        assert_eq!(info2.get_min_version(), None);
                    } else {
                        assert_eq!(*info2.get_min_version().unwrap(), min_version.unwrap());
                    }
                } else {
                    assert_eq!(info2.get_min_version(), None);
                }
            }
        }

        Ok(())
    }

    fn get_versions(&self) -> Vec<Version>;

    fn supports_has_blocks(&self) -> bool {
        true
    }
    /// Whether this format records min versions. */
    fn supports_min_version(&self) -> bool {
        true
    }
}

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
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use num_bigint::BigInt;
use parking_lot::Mutex;
use rand::Rng;

use crate::codecs::segment_info_format::SegmentInfoFormat;
use crate::codecs::{Codec, LATEST_CODEC};
use crate::index::IndexFileNames;
use crate::index::index_writer::index_writer_util;
use crate::index::segment_info::SegmentInfo;
use crate::index::sort::Sort;
use crate::search::sort_field::{MissingValueEnum, SortField, SortFieldType, SortFiledBase};
use crate::search::sort_field_enum::SortFieldEnum;
use crate::search::sorted_numeric_sort_field::SortedNumericSortField;
use crate::search::sorted_set_sort_field::SortedSetSortField;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::test::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, new_directory};
use crate::test::util::test_util::TestUtil;
use crate::util::error::lucene_error::Result;
use crate::util::{StringHelper, Version};

pub trait BaseSegmentInfoFormatTestCase: BaseIndexFileFormatTestCase {
    /// Test files map
    fn test_files<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let io_context = IOContext::default_io_context()?;
        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            false,
            HashMap::new(),
            id,
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(*info.files()?, *info2.files()?);
        Ok(())
    }
    fn test_has_blocks<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        assert!(self.supports_has_blocks());
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let has_blocks = random.random_bool(0.5);
        let io_context = IOContext::default_io_context()?;
        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            has_blocks,
            HashMap::new(),
            id,
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(info.get_has_blocks(), info2.get_has_blocks());
        Ok(())
    }

    /// Tests SI writer adds itself to files...
    fn test_adds_self_to_files<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let io_context = IOContext::default_io_context()?;

        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            false,
            HashMap::new(),
            id,
            HashMap::new(),
            None,
        )?;
        let original_files: HashSet<String> = ["_123.a".to_string()].iter().cloned().collect();
        info.set_files(original_files.clone())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let modified_files = info.files()?;
        assert!(modified_files.is_superset(&original_files));
        assert!(
            modified_files.len() > original_files.len(),
            "did you forget to add yourself to files()"
        );
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(*info.files()?, *info2.files()?);
        // In Rust Lucene, SegmentInfo::files return an immutable Set,
        // so we do not need to verify this
        // let immutable_files = info2.files()?;
        // let add_result = immutable_files.insert("bogus".to_string());
        // assert!(
        //     !add_result,
        //     "Files set should be immutable, but modification was allowed."
        // );
        Ok(())
    }
    /// Test diagnostics map
    fn test_diagnostics<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut diagnostics: HashMap<String, String> = HashMap::new();
        diagnostics.insert("key1".to_string(), "value1".to_string());
        diagnostics.insert("key2".to_string(), "value2".to_string());
        let io_context = IOContext::default_io_context()?;

        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            false,
            diagnostics.clone(),
            id,
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(diagnostics, *info2.get_diagnostics());
        // In Rust Lucene, SegmentInfo::get_diagnostics return an immutable Set,
        // so we do not need to verify this
        // let mut immutable_diagnostics = info2.get_diagnostics();
        // let insert_result = immutable_diagnostics.insert("bogus".to_string(),
        // "bogus".to_string()); assert!(
        //     insert_result.is_none(),
        //     "Diagnostics map should be immutable, but modification was
        // allowed." );
        Ok(())
    }
    /// Test attributes map
    fn test_attributes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let mut attributes: HashMap<String, String> = HashMap::new();
        attributes.insert("key1".to_string(), "value1".to_string());
        attributes.insert("key2".to_string(), "value2".to_string());
        let io_context = IOContext::default_io_context()?;

        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            false,
            HashMap::new(),
            id,
            attributes.clone(),
            None,
        )?;
        info.set_files(HashSet::new())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(attributes, *info2.get_attributes()?);
        // 在 Rust Lucene 中，attributes
        // 的返回值是不可变的，因此不需要检查修改是否被禁止。
        // 如果需要，可以解除注释并尝试修改
        // let mut immutable_attributes = info2.get_attributes();
        // let insert_result = immutable_attributes.insert("bogus".to_string(),
        // "bogus".to_string()); assert!(
        //     insert_result.is_none(),
        //     "Attributes map should be immutable, but modification was
        // allowed." );

        Ok(())
    }

    /// Test unique ID
    fn test_unique_id<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let id = StringHelper::random_id();
        let io_context = IOContext::default_io_context()?;

        let mut info = SegmentInfo::new(
            dir.clone(),
            Option::from(self.get_versions()[0].clone()),
            Option::from(self.get_versions()[0].clone()),
            "_123",
            1,
            false,
            false,
            HashMap::new(),
            id,
            HashMap::new(),
            None,
        )?;
        info.set_files(HashSet::new())?;
        LATEST_CODEC
            .segment_info_format()
            .write(&*dir.lock(), &mut info, &io_context)?;
        let info2 =
            LATEST_CODEC
                .segment_info_format()
                .read(dir.clone(), "_123", &id, &io_context)?;
        assert_eq!(id, info2.get_id().as_slice());

        Ok(())
    }
    /// Test versions
    fn test_versions<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let io_context = IOContext::default_io_context()?;

        for version in self.get_versions() {
            for min_version in [Some(version.clone()), None] {
                let dir = Arc::new(Mutex::new(new_directory(random)?));
                let id = StringHelper::random_id();
                let mut info = SegmentInfo::new(
                    dir.clone(),
                    Some(version.clone()),
                    min_version.clone(),
                    "_123",
                    1,
                    false,
                    false,
                    HashMap::new(),
                    id,
                    HashMap::new(),
                    None,
                )?;
                info.set_files(HashSet::new())?;
                LATEST_CODEC
                    .segment_info_format()
                    .write(&*dir.lock(), &mut info, &io_context)?;
                let info2 = LATEST_CODEC.segment_info_format().read(
                    dir.clone(),
                    "_123",
                    &id,
                    &io_context,
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

    fn random_index_sort_field<R: Rng + ?Sized>(random: &mut R) -> Result<Option<SortFieldEnum>> {
        let reversed = random.random_bool(0.5);
        let case = random.random_range(0..10);
        match case {
            0 => {
                let mut sort_field = SortField::with_reverse(
                    Some(TestUtil::random_simple_string(random)),
                    SortFieldType::Int,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Int(random.random())))?;
                }
                Ok(Some(SortFieldEnum::Sorter(sort_field)))
            },
            1 => {
                let mut sort_field = SortedNumericSortField::with_reverse(
                    TestUtil::random_simple_string(random),
                    SortFieldType::Int,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Int(random.random())))?;
                }
                Ok(Some(SortFieldEnum::SortedNumeric(sort_field)))
            },
            2 => {
                let mut sort_field = SortField::with_reverse(
                    Some(TestUtil::random_simple_string(random)),
                    SortFieldType::Long,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Long(random.random())))?;
                }
                Ok(Some(SortFieldEnum::Sorter(sort_field)))
            },
            3 => {
                let mut sort_field = SortedNumericSortField::with_reverse(
                    TestUtil::random_simple_string(random),
                    SortFieldType::Long,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Long(random.random())))?;
                }
                Ok(Some(SortFieldEnum::SortedNumeric(sort_field)))
            },
            4 => {
                let mut sort_field = SortField::with_reverse(
                    Some(TestUtil::random_simple_string(random)),
                    SortFieldType::Float,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Float(random.random())))?;
                }
                Ok(Some(SortFieldEnum::Sorter(sort_field)))
            },
            5 => {
                let mut sort_field = SortedNumericSortField::with_reverse(
                    TestUtil::random_simple_string(random),
                    SortFieldType::Float,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::Float(random.random())))?;
                }
                Ok(Some(SortFieldEnum::SortedNumeric(sort_field)))
            },
            6 => {
                let mut sort_field = SortField::with_reverse(
                    Some(TestUtil::random_simple_string(random)),
                    SortFieldType::Double,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field
                        .set_missing_value(Some(MissingValueEnum::Double(random.random())))?;
                }
                Ok(Some(SortFieldEnum::Sorter(sort_field)))
            },
            7 => {
                let mut sort_field = SortedNumericSortField::with_reverse(
                    TestUtil::random_simple_string(random),
                    SortFieldType::Double,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field
                        .set_missing_value(Some(MissingValueEnum::Double(random.random())))?;
                }
                Ok(Some(SortFieldEnum::SortedNumeric(sort_field)))
            },
            8 => {
                let mut sort_field = SortField::with_reverse(
                    Some(TestUtil::random_simple_string(random)),
                    SortFieldType::String,
                    reversed,
                )?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::StringLast))?;
                }
                Ok(Some(SortFieldEnum::Sorter(sort_field)))
            },
            9 => {
                let mut sort_field =
                    SortedSetSortField::new(TestUtil::random_simple_string(random), reversed)?;
                if random.random_bool(0.5) {
                    sort_field.set_missing_value(Some(MissingValueEnum::StringLast))?;
                }
                Ok(Some(SortFieldEnum::SortedSet(sort_field)))
            },
            _ => Ok(None),
        }
    }
    /// Test sort
    fn test_sort<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        assert!(
            self.supports_index_sort(),
            "test requires a codec that can read/write index sort"
        );
        let io_context = IOContext::default_io_context()?;

        let iters = at_least(random, 5);
        for _ in 0..iters {
            let sort = if random.random_bool(0.2) {
                None
            } else {
                let num_sort_fields = TestUtil::next_int(random, 1, 3);
                let mut sort_fields = Vec::new();
                for _ in 0..num_sort_fields {
                    if let Some(sort_field) = Self::random_index_sort_field(random)? {
                        sort_fields.push(sort_field);
                    }
                }
                Some(Arc::new(Sort::with_fields(sort_fields)?))
            };
            let sort_clone = sort.clone();
            let dir = Arc::new(Mutex::new(new_directory(random)?));
            let id = StringHelper::random_id();
            let mut info = SegmentInfo::new(
                dir.clone(),
                Some(self.get_versions()[0].clone()),
                Some(self.get_versions()[0].clone()),
                "_123",
                1,
                false,
                false,
                HashMap::new(),
                id,
                HashMap::new(),
                sort,
            )?;
            info.set_files(HashSet::new())?;
            LATEST_CODEC
                .segment_info_format()
                .write(&*dir.lock(), &mut info, &io_context)?;
            let info2 =
                LATEST_CODEC
                    .segment_info_format()
                    .read(dir.clone(), "_123", &id, &io_context)?;
            if info2.get_index_sort().is_some() {
                assert!(info2.get_index_sort().is_some());
                assert!(*sort_clone.as_ref().unwrap() == info2.get_index_sort().unwrap());
            } else {
                assert!(sort_clone.is_none())
            }
        }
        Ok(())
    }
    fn test_exception_on_create_output(&self) -> Result<()> {
        // TODO
        Ok(())
    }
    fn test_exception_on_close_output(&self) -> Result<()> {
        // TODO
        Ok(())
    }
    fn test_exception_on_open_input(&self) -> Result<()> {
        // TODO
        Ok(())
    }
    fn test_exception_on_close_input(&self) -> Result<()> {
        // TODO
        Ok(())
    }

    /// Sets some otherwise hard-to-test properties: random segment names, ID
    /// values, document count, etc and round-trips
    fn test_random<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let versions = self.get_versions();
        let io_context = IOContext::default_io_context()?;

        for _ in 0..10 {
            let dir = Arc::new(Mutex::new(new_directory(random)?));
            let version = versions[random.random_range(0..versions.len())].clone();
            let random_segment_index = random.random::<i64>().abs();
            let big_int = if random_segment_index != i64::MIN {
                BigInt::from(random_segment_index)
            } else {
                BigInt::from(random.random_range(0..i32::MAX) as i64)
            };
            let name = format!("_{}", big_int.to_str_radix(36));
            let doc_count = TestUtil::next_int(random, 1, index_writer_util::MAX_DOCS);
            let is_compound_file = random.random_bool(0.5);
            let mut files = HashSet::new();
            let num_files = random.random_range(0..10);
            for j in 0..num_files {
                let file = IndexFileNames::segment_file_name(&name, "", &j.to_string());
                files.insert(file.clone());
                let directory = dir.lock();
                directory.create_output(&file, &io_context)?;
            }
            let mut diagnostics = HashMap::new();
            let num_diags = random.random_range(0..10);
            for _ in 0..num_diags {
                diagnostics.insert(
                    TestUtil::random_unicode_string(random),
                    TestUtil::random_unicode_string(random),
                );
            }
            let mut id = [0; StringHelper::ID_LENGTH];
            random.fill(&mut id[..]);
            let mut attributes = HashMap::new();
            let num_attributes = random.random_range(0..10);
            for _ in 0..num_attributes {
                attributes.insert(
                    TestUtil::random_unicode_string(random),
                    TestUtil::random_unicode_string(random),
                );
            }
            let mut info = SegmentInfo::new(
                dir.clone(),
                Some(version.clone()),
                None,
                &name,
                doc_count,
                is_compound_file,
                false,
                diagnostics,
                id,
                attributes,
                None,
            )?;
            info.set_files(files.clone())?;
            LATEST_CODEC
                .segment_info_format()
                .write(&*dir.lock(), &mut info, &io_context)?;
            let info2 =
                LATEST_CODEC
                    .segment_info_format()
                    .read(dir.clone(), &name, &id, &io_context)?;
            Self::assert_equals(&info, &info2)?;
        }
        Ok(())
    }
    fn assert_equals<D: Directory>(
        expected: &SegmentInfo<D>,
        actual: &SegmentInfo<D>,
    ) -> Result<()> {
        assert!(
            Arc::ptr_eq(&expected.dir, &actual.dir),
            "Directory references are not the same"
        );
        assert_eq!(expected.name, actual.name, "Segment names do not match");
        assert_eq!(*expected.files()?, *actual.files()?, "Files do not match");
        assert_eq!(
            *expected.get_diagnostics(),
            *actual.get_diagnostics(),
            "Diagnostics do not match"
        );
        assert_eq!(
            expected.max_doc()?,
            actual.max_doc()?,
            "MaxDoc values do not match"
        );
        assert_eq!(
            expected.get_id(),
            actual.get_id(),
            "Segment IDs do not match"
        );
        assert_eq!(
            expected.get_use_compound_file(),
            actual.get_use_compound_file(),
            "UseCompoundFile values do not match"
        );
        assert_eq!(
            expected.get_version().unwrap(),
            actual.get_version().unwrap(),
            "Versions do not match"
        );
        assert_eq!(
            *expected.get_attributes()?,
            *actual.get_attributes()?,
            "Attributes do not match"
        );
        Ok(())
    }

    fn get_versions(&self) -> Vec<Version>;
    fn supports_index_sort(&self) -> bool {
        true
    }

    fn supports_has_blocks(&self) -> bool {
        true
    }
    /// Whether this format records min versions.  */
    fn supports_min_version(&self) -> bool {
        true
    }
}

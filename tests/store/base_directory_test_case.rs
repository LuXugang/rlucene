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
use crate::common::{is_night_mode, my_random_with_seed};
use crate::util::lucene_test_case::{new_directory, new_io_context, slow_file_exists};
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rand::rngs::StdRng;
use rand::{Rng, RngCore};
use rlucene::index::IndexFileNames;
use rlucene::store::check_sum_index_input::ChecksumIndexInput;
use rlucene::store::directory::Directory;
use rlucene::store::random_access_input::RandomAccessInput;
use rlucene::store::DataInput;
use rlucene::store::IndexInput;
use rlucene::store::IndexOutput;
use rlucene::store::{DataOutput, IOContext};
use rlucene::util::error::illegal_state::IllegalStateError;
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::group_vint_util::GroupVIntUtil;
use rlucene::util::packed::PackedInts;
use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::Builder;

pub const EXTRA_FILE_NAME: &str = "extra0";
pub trait BaseDirectoryTestCase {
    type Directory: Directory<Output = Self::Output> + Send + Sync + 'static;
    type Output: IndexInput + RandomAccessInput + Send + Sync + 'static;
    fn get_directory(&self, path: PathBuf) -> Result<Self::Directory, TestError>;

    fn test_copy_from(&self, random: &mut StdRng) -> Result<(), TestError> {
        let mut temp_dir = Builder::new().prefix("testCopy").tempdir()?;
        let mut source = self.get_directory(temp_dir.into_path())?;
        let mut dest = new_directory(random)?;
        Self::run_copy_from(&mut source, &mut dest, random)?;

        let mut source = new_directory(random)?;
        temp_dir = Builder::new().prefix("testCopyDestination").tempdir()?;
        let mut dest = self.get_directory(temp_dir.into_path())?;
        Self::run_copy_from(&mut source, &mut dest, random)?;
        Ok(())
    }

    fn run_copy_from(
        source: &mut impl Directory,
        dest: &mut impl Directory,
        random: &mut StdRng,
    ) -> Result<(), TestError> {
        let mut bytes = vec![0u8; 20000];
        random.fill(&mut bytes[..]);
        {
            let mut output = source.create_output("foobar", new_io_context(random)?)?;

            output.write_bytes_with_len(&bytes, bytes.len() as u32)?;
        }
        dest.copy_from(source, "foobar", "foobaz", new_io_context(random)?)?;
        assert!(slow_file_exists(dest, "foobaz")?);
        let bytes2_len = bytes.len();
        let mut bytes2 = vec![0u8; bytes2_len];
        {
            let mut input = dest.open_input("foobaz", new_io_context(random)?)?;
            DataInput::read_bytes(&mut input, &mut bytes2, 0, bytes2_len as u32)?;
        }

        assert_eq!(bytes, bytes2);

        Ok(())
    }
    fn test_rename(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testRename").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;
        let num_bytes = random.gen_range(0..20000);
        let mut bytes = vec![0u8; num_bytes];
        random.fill(&mut bytes[..]);
        {
            let mut output = dir.create_output("foobar", new_io_context(random)?)?;
            output.write_bytes_with_len(&bytes, bytes.len() as u32)?;
        }

        dir.rename("foobar", "foobaz")?;

        let mut bytes2 = vec![0u8; num_bytes];
        {
            let mut input = dir.open_input("foobaz", new_io_context(random)?)?;
            DataInput::read_bytes(&mut input, &mut bytes2, 0, num_bytes as u32)?;
            assert_eq!(IndexInput::length(&input), num_bytes as u64);
        }

        assert_eq!(bytes, bytes2);

        Ok(())
    }

    fn test_delete_file(&self) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testDeleteFile").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        let file = "foo.txt";

        assert!(!dir.list_all()?.contains(&file.to_string()));

        dir.create_output(file, IOContext::default_io_context()?)?;
        assert!(dir.list_all()?.contains(&file.to_string()));

        dir.delete_file(file)?;
        assert!(!dir.list_all()?.contains(&file.to_string()));

        let result = dir.delete_file(file);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(LuceneError::IoWithPath {
                source,
                path
            }) if source.kind() == std::io::ErrorKind::NotFound && path.contains(file)
        ));
        Ok(())
    }
    fn test_byte(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testByte").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("byte", new_io_context(random)?)?;
            output.write_byte(128)?;
        }

        {
            let mut input = dir.open_input("byte", new_io_context(random)?)?;
            assert_eq!(1, IndexInput::length(&input));
            assert_eq!(128u8, DataInput::read_byte(&mut input)?);
        }

        Ok(())
    }
    fn test_short(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testShort").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("short", new_io_context(random)?)?;
            output.write_short(-20)?;
        }

        {
            let mut input = dir.open_input("short", new_io_context(random)?)?;
            assert_eq!(2, IndexInput::length(&input));
            assert_eq!(-20i16, DataInput::read_short(&mut input)?);
        }

        Ok(())
    }
    fn test_int(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testInt").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("int", new_io_context(random)?)?;
            output.write_int(-500)?;
        }

        {
            let mut input = dir.open_input("int", new_io_context(random)?)?;
            assert_eq!(4, IndexInput::length(&input));
            assert_eq!(-500, DataInput::read_int(&mut input)?);
        }

        Ok(())
    }
    fn test_long(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testLong").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("long", new_io_context(random)?)?;
            output.write_long(-5000)?;
        }

        {
            let mut input = dir.open_input("long", new_io_context(random)?)?;
            assert_eq!(8, IndexInput::length(&input));
            assert_eq!(-5000, DataInput::read_long(&mut input)?);
        }

        Ok(())
    }
    fn test_aligned_little_endian_longs(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new()
            .prefix("testAlignedLittleEndianLongs")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut out = dir.create_output("littleEndianLongs", new_io_context(random)?)?;
            out.write_long(3)?;
            out.write_long(i64::MAX)?;
            out.write_long(-3)?;
        }

        {
            let mut input = dir.open_input("littleEndianLongs", new_io_context(random)?)?;
            assert_eq!(24, IndexInput::length(&input));

            let mut l = vec![0; 4];
            input.read_longs(&mut l, 1, 3)?;

            assert_eq!(vec![0, 3, i64::MAX, -3], l);
            assert_eq!(24, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_unaligned_little_endian_longs(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new()
            .prefix("testUnalignedLittleEndianLongs")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut out = dir.create_output("littleEndianLongs", new_io_context(random)?)?;
            out.write_byte(2)?;
            out.write_long(3)?;
            out.write_long(i64::MAX)?;
            out.write_long(-3)?;
        }

        {
            let mut input = dir.open_input("littleEndianLongs", new_io_context(random)?)?;
            assert_eq!(25, IndexInput::length(&input));
            assert_eq!(2u8, DataInput::read_byte(&mut input)?);
            let mut longs = vec![0; 4];
            input.read_longs(&mut longs, 1, 3)?;
            assert_eq!(vec![0, 3, i64::MAX, -3], longs);
            assert_eq!(25, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_little_endian_longs_underflow(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new()
            .prefix("testLittleEndianLongsUnderflow")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        let offset = random.gen_range(0..8);
        let length = random.gen_range(1..=16);
        let padding = offset + length * std::mem::size_of::<i64>()
            - random.gen_range(1..=std::mem::size_of::<i64>());

        {
            let mut out = dir.create_output("littleEndianLongs", new_io_context(random)?)?;
            let mut bytes = vec![0u8; padding];
            random.fill(&mut bytes[..]);
            out.write_bytes_with_len(&bytes, bytes.len() as u32)?;
        }

        {
            let mut input = dir.open_input("littleEndianLongs", new_io_context(random)?)?;
            input.seek(offset as u64)?;

            let result = input.read_longs(&mut vec![0i64; length], 0, length as u32);
            assert!(matches!(result, Err(LuceneError::Eof(_))));
        }

        Ok(())
    }
    fn test_aligned_ints(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testAlignedInts").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut out = dir.create_output("Ints", new_io_context(random)?)?;
            out.write_int(3)?;
            out.write_int(i32::MAX)?;
            out.write_int(-3)?;
        }

        {
            let mut input = dir.open_input("Ints", new_io_context(random)?)?;
            assert_eq!(12, IndexInput::length(&input));
            let mut ints = vec![0; 4];
            input.read_ints(&mut ints, 1, 3)?;
            assert_eq!(vec![0, 3, i32::MAX, -3], ints);
            assert_eq!(12, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_unaligned_ints(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testUnalignedInts").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;
        let padding = random.gen_range(1..=3);

        {
            let mut out = dir.create_output("Ints", new_io_context(random)?)?;
            for _ in 0..padding {
                out.write_byte(2)?;
            }
            out.write_int(3)?;
            out.write_int(i32::MAX)?;
            out.write_int(-3)?;
        }

        {
            let mut input = dir.open_input("Ints", new_io_context(random)?)?;
            assert_eq!(12 + padding, IndexInput::length(&input));
            for _ in 0..padding {
                assert_eq!(2u8, DataInput::read_byte(&mut input)?);
            }
            let mut ints = vec![0; 4];
            input.read_ints(&mut ints, 1, 3)?;
            assert_eq!(vec![0, 3, i32::MAX, -3], ints);
            assert_eq!(12 + padding, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_ints_underflow(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testIntsUnderflow").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        let offset = random.gen_range(0..4);
        let length = random.gen_range(1..=16);

        let total_size = offset + length * std::mem::size_of::<i32>();
        let truncation = random.gen_range(1..=std::mem::size_of::<i32>());
        let data_size = total_size - truncation;
        let mut bytes = vec![0u8; data_size];
        random.fill(&mut bytes[..]);

        {
            let mut output = dir.create_output("Ints", new_io_context(random)?)?;
            output.write_bytes_with_len(&bytes, bytes.len() as u32)?;
        }

        let mut input = dir.open_input("Ints", new_io_context(random)?)?;
        input.seek(offset as u64)?;

        let mut ints = vec![0; length];
        let result = input.read_ints(&mut ints, 0, length as u32);
        assert!(matches!(result, Err(LuceneError::Eof(_))));

        Ok(())
    }

    fn test_aligned_floats(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testAlignedFloats").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut out = dir.create_output("Floats", new_io_context(random)?)?;
            out.write_int(3f32.to_bits() as i32)?;
            out.write_int(f32::MAX.to_bits() as i32)?;
            out.write_int((-3f32).to_bits() as i32)?;
        }

        {
            let mut input = dir.open_input("Floats", new_io_context(random)?)?;
            assert_eq!(12, IndexInput::length(&input));
            let mut floats = vec![0.0f32; 4];
            input.read_floats(&mut floats, 1, 3)?;
            assert_eq!(vec![0.0, 3.0, f32::MAX, -3.0], floats);
            assert_eq!(12, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_unaligned_floats(&self, random: &mut StdRng) -> Result<(), TestError> {
        let padding = random.gen_range(1..=3);

        let temp_dir = Builder::new().prefix("testUnalignedFloats").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("Floats", new_io_context(random)?)?;
            for _ in 0..padding {
                output.write_byte(2)?;
            }
            output.write_int(3f32.to_bits() as i32)?;
            output.write_int(f32::MAX.to_bits() as i32)?;
            output.write_int((-3f32).to_bits() as i32)?;
        }

        let mut input = dir.open_input("Floats", new_io_context(random)?)?;
        assert_eq!(12 + padding as u64, IndexInput::length(&input));
        for _ in 0..padding {
            assert_eq!(2u8, DataInput::read_byte(&mut input)?);
        }

        let mut ff = vec![0f32; 4];
        input.read_floats(&mut ff, 1, 3)?;
        assert_eq!(ff, vec![0.0, 3.0, f32::MAX, -3.0]);
        assert_eq!(12 + padding as u64, input.get_file_pointer());

        Ok(())
    }
    fn test_floats_underflow(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testFloatsUnderflow").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        let offset = random.gen_range(0..4);
        let length = random.gen_range(1..=16);
        {
            let size = offset + length * std::mem::size_of::<f32>()
                - random.gen_range(1..=std::mem::size_of::<f32>());
            let mut b = vec![0u8; size];
            random.fill(&mut b[..]);

            let mut output = dir.create_output("Floats", new_io_context(random)?)?;
            output.write_bytes_with_len(&b, b.len() as u32)?;
        }

        let mut input = dir.open_input("Floats", new_io_context(random)?)?;
        input.seek(offset as u64)?;
        let result = input.read_floats(&mut vec![0.0; length], 0, length as u32);
        assert!(matches!(result, Err(LuceneError::Eof(_))));

        Ok(())
    }
    fn test_string(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testString").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("string", new_io_context(random)?)?;
            output.write_string("hello!")?;
        }

        {
            let mut input = dir.open_input("string", new_io_context(random)?)?;
            assert_eq!("hello!", input.read_string()?);
            assert_eq!(7, IndexInput::length(&input));
        }

        Ok(())
    }
    fn test_vint(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testVInt").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("vint", new_io_context(random)?)?;
            output.write_vint(500)?;
        }

        {
            let mut input = dir.open_input("vint", new_io_context(random)?)?;
            assert_eq!(2, IndexInput::length(&input));
            assert_eq!(500, input.read_vint()?);
        }

        Ok(())
    }
    fn test_vlong(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testVLong").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("vlong", new_io_context(random)?)?;
            output.write_vlong(i64::MAX)?;
        }

        {
            let mut input = dir.open_input("vlong", new_io_context(random)?)?;
            assert_eq!(9, IndexInput::length(&input));
            assert_eq!(i64::MAX, input.read_vlong()?);
        }

        Ok(())
    }
    fn test_zint(&self, random: &mut StdRng) -> Result<(), TestError> {
        let mut ints = Vec::new();
        let num_ints = random.gen_range(0..10);

        for _ in 0..num_ints {
            let value = match random.gen_range(0..3) {
                0 => random.gen::<i32>(),
                1 => {
                    if random.gen_bool(0.5) {
                        i32::MIN
                    } else {
                        i32::MAX
                    }
                }
                2 => {
                    let sign = if random.gen_bool(0.5) { -1 } else { 1 };
                    sign * random.gen_range(0..1024)
                }
                _ => unreachable!(),
            };
            ints.push(value);
        }

        let temp_dir = Builder::new().prefix("testZInt").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("zint", new_io_context(random)?)?;
            for &i in &ints {
                output.write_zint(i)?;
            }
        }

        {
            let mut input = dir.open_input("zint", new_io_context(random)?)?;
            for &i in &ints {
                assert_eq!(i, input.read_zint()?);
            }
            assert_eq!(IndexInput::length(&input), input.get_file_pointer());
        }

        Ok(())
    }

    fn test_zlong(&self, random: &mut StdRng) -> Result<(), TestError> {
        let mut longs = Vec::new();
        let num_longs = random.gen_range(0..10);

        for _ in 0..num_longs {
            let value = match random.gen_range(0..3) {
                0 => random.gen::<i64>(), // Random 64-bit integer
                1 => {
                    if random.gen_bool(0.5) {
                        i64::MIN // Minimum value for i64
                    } else {
                        i64::MAX // Maximum value for i64
                    }
                }
                2 => {
                    let sign = if random.gen_bool(0.5) { -1 } else { 1 };
                    sign * random.gen_range(0..1024) as i64 // Small range value with random sign
                }
                _ => unreachable!(),
            };
            longs.push(value);
        }

        let temp_dir = Builder::new().prefix("testZLong").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("zlong", new_io_context(random)?)?;
            for &l in &longs {
                output.write_zlong(l)?;
            }
        }

        {
            let mut input = dir.open_input("zlong", new_io_context(random)?)?;
            for &l in &longs {
                assert_eq!(l, input.read_zlong()?);
            }
            assert_eq!(IndexInput::length(&input), input.get_file_pointer());
        }

        Ok(())
    }
    fn test_set_of_strings(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = Builder::new().prefix("testSetOfStrings").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("stringset", new_io_context(random)?)?;
            output.write_set_of_strings(
                &["test1".to_string(), "test2".to_string()]
                    .iter()
                    .cloned()
                    .collect(),
            )?;
            output.write_set_of_strings(&HashSet::new())?;
            output.write_set_of_strings(&["test3".to_string()].iter().cloned().collect())?;
        }

        {
            let mut input = dir.open_input("stringset", new_io_context(random)?)?;

            let set1 = input.read_set_of_strings()?;
            assert_eq!(
                set1,
                ["test1".to_string(), "test2".to_string()]
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            );

            let set2 = input.read_set_of_strings()?;
            assert_eq!(set2, HashSet::new());

            let set3 = input.read_set_of_strings()?;
            assert_eq!(
                set3,
                ["test3".to_string()]
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            );

            assert_eq!(IndexInput::length(&input), input.get_file_pointer());
        }

        Ok(())
    }

    fn test_map_of_strings(&self, random: &mut StdRng) -> Result<(), TestError> {
        let mut map = HashMap::new();
        map.insert("test1".to_string(), "value1".to_string());
        map.insert("test2".to_string(), "value2".to_string());

        let temp_dir = Builder::new().prefix("testMapOfStrings").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("stringmap", new_io_context(random)?)?;
            output.write_map_of_strings(&map)?;
            output.write_map_of_strings(&HashMap::new())?;
            let singleton_map: HashMap<String, String> =
                [(String::from("key"), String::from("value"))]
                    .into_iter()
                    .collect();
            output.write_map_of_strings(&singleton_map)?;
        }

        {
            let mut input = dir.open_input("stringmap", new_io_context(random)?)?;

            let map1 = input.read_map_of_strings()?;
            assert_eq!(map1, map);

            // Attempt to mutate the map to ensure it's immutable in context
            let mut map1_clone = map1.clone(); // Rust enforces immutability by default
            map1_clone.insert("bogus1".to_string(), "bogus2".to_string()); // This will not affect the original `map1`

            let map2 = input.read_map_of_strings()?;
            assert!(map2.is_empty());

            let mut map2_clone = map2.clone();
            map2_clone.insert("bogus1".to_string(), "bogus2".to_string()); // This will not affect the original `map2`

            let map3 = input.read_map_of_strings()?;
            let expected_singleton_map: HashMap<String, String> =
                [(String::from("key"), String::from("value"))]
                    .into_iter()
                    .collect();
            assert_eq!(map3, expected_singleton_map);

            let mut map3_clone = map3.clone();
            map3_clone.insert("bogus1".to_string(), "bogus2".to_string()); // This will not affect the original `map3`

            assert_eq!(IndexInput::length(&input), input.get_file_pointer());
        }

        Ok(())
    }
    fn test_checksum(&self, random: &mut StdRng) -> Result<(), TestError> {
        use crc32fast::Hasher;

        let num_bytes = random.gen_range(0..20000);
        let mut bytes = vec![0u8; num_bytes];
        random.fill(&mut bytes[..]);

        let mut hasher = Hasher::new();
        hasher.update(&bytes);
        let expected_checksum = hasher.finalize();

        let temp_dir = Builder::new().prefix("testChecksum").tempdir()?;
        let mut dir = self.get_directory(temp_dir.into_path())?;

        {
            let mut output = dir.create_output("checksum", new_io_context(random)?)?;
            output.write_bytes_range(&bytes, 0, bytes.len() as u32)?;
        }

        {
            let mut input = dir.open_checksum_input("checksum")?;
            IndexInput::skip_bytes(&mut input, num_bytes as u64)?;
            let actual_checksum = input.get_checksum();
            assert_eq!(expected_checksum as u64, actual_checksum);
        }

        Ok(())
    }
    #[allow(unused)]
    fn test_detect_close(&self) -> Result<(), TestError> {
        //in Rust, it is not necessary to explicitly call close.
        // Resources are automatically closed when they go out of scope,
        // and the drop method is invoked.
        Ok(())
    }
    fn test_thread_safety_in_list_all(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testThreadSafety")
            .tempdir()?;
        let dir = Arc::new(Mutex::new(
            self.get_directory(temp_dir.path().to_path_buf())?,
        ));

        let stop = Arc::new(AtomicBool::new(false));

        // Writer thread
        let dir_writer = Arc::clone(&dir);
        let stop_writer = Arc::clone(&stop);
        let seed: u64 = random.gen();
        let writer = thread::spawn(move || -> Result<(), TestError> {
            let mut rng = my_random_with_seed(seed);
            let file_count = rng.gen_range(500..=1000);

            for i in 0..file_count {
                let file_name = format!("file-{}", i);
                if let Ok(mut dir) = dir_writer.lock() {
                    if let Ok(_output) =
                        dir.create_output(&file_name, IOContext::default_io_context()?)
                    {
                        thread::yield_now();
                    }
                    assert!(slow_file_exists(&*dir, &file_name)?);
                } else {
                    return Err(TestError::IllegalState(IllegalStateError::new(
                        "Failed to acquire lock in writer",
                    )));
                }
            }

            stop_writer.store(true, Ordering::SeqCst);
            Ok(())
        });

        // Reader thread
        let dir_reader = Arc::clone(&dir);
        let stop_reader = Arc::clone(&stop);
        let reader = thread::spawn(move || -> Result<(), TestError> {
            let mut rng = my_random_with_seed(seed);

            while !stop_reader.load(Ordering::SeqCst) {
                let files: Vec<String> = {
                    let dir = dir_reader.lock().unwrap();
                    dir.list_all()?
                        .into_iter()
                        .filter(|name| !name.eq(EXTRA_FILE_NAME))
                        .collect()
                };

                if !files.is_empty() {
                    loop {
                        let file = files[rng.gen_range(0..files.len())].as_str();
                        match dir_reader
                            .lock()
                            .unwrap()
                            .open_input(file, new_io_context(&mut rng)?)
                        {
                            Ok(_input) => {
                                thread::sleep(Duration::from_millis(1));
                            }
                            Err(LuceneError::IoWithPath { source, .. })
                                if source.kind() == ErrorKind::PermissionDenied =>
                            {
                                // 忽略 AccessDenied 错误
                            }
                            Err(e) => {
                                return Err(TestError::LuceneError(LuceneError::IoWithPath {
                                    path: file.to_string(),
                                    source: Error::new(ErrorKind::Other, format!("{:?}", e)),
                                }));
                            }
                        }
                        if rng.gen_range(0..3) == 0 {
                            break;
                        }
                    }
                }
            }
            Ok(())
        });

        match writer.join() {
            Ok(Ok(())) => (),
            Ok(Err(e)) => {
                eprintln!("Writer thread error: {:?}", e);
                return Err(e);
            }
            Err(_) => {
                eprintln!("Writer thread panicked!");
                unreachable!()
            }
        }

        match reader.join() {
            Ok(Ok(())) => (),
            Ok(Err(e)) => {
                eprintln!("Reader thread error: {:?}", e);
                return Err(e);
            }
            Err(_) => {
                eprintln!("Reader thread panicked!");
                unreachable!()
            }
        }

        Ok(())
    }

    fn test_file_exists_in_list_after_created(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testFileExistsInListAfterCreated")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let name = "file";

        {
            let _output = dir.create_output(name, new_io_context(random)?)?;
        }

        assert!(
            slow_file_exists(&dir, name)?,
            "File '{}' should exist after creation.",
            name
        );

        let files: HashSet<String> = dir.list_all()?.into_iter().collect();
        assert!(
            files.contains(name),
            "File '{}' should be present in the directory listing.",
            name
        );

        Ok(())
    }

    fn test_seek_to_eof_then_back(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testSeekToEOFThenBack")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let buffer_length = 1024;
        let total_length = 3 * buffer_length;
        let bytes = vec![0u8; total_length];

        {
            let mut output = dir.create_output("out", new_io_context(random)?)?;
            output.write_bytes_range(&bytes, 0, total_length as u32)?;
        }

        {
            let mut input = dir.open_input("out", new_io_context(random)?)?;
            input.seek((2 * buffer_length - 1) as u64)?;
            input.seek((3 * buffer_length) as u64)?;
            input.seek(buffer_length as u64)?;

            let mut read_bytes = vec![0u8; 2 * buffer_length];
            DataInput::read_bytes(&mut input, &mut read_bytes, 0, (2 * buffer_length) as u32)?;
            assert_eq!(&read_bytes, &bytes[buffer_length..3 * buffer_length]);
        }

        Ok(())
    }

    fn test_illegal_eof(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testIllegalEOF")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let buffer = vec![0u8; 1024];
        {
            let mut output = dir.create_output("out", new_io_context(random)?)?;
            output.write_bytes_range(&buffer, 0, buffer.len() as u32)?;
        }

        {
            let mut input = dir.open_input("out", new_io_context(random)?)?;
            input.seek(1024)?;
        }

        Ok(())
    }
    fn test_seek_past_eof(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testSeekPastEOF")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let len = random.gen_range(0..2048);
        let buffer = vec![0u8; len];
        {
            let mut output = dir.create_output("out", new_io_context(random)?)?;
            output.write_bytes_range(&buffer, 0, len as u32)?;
        }

        let mut input = dir.open_input("out", new_io_context(random)?)?;

        // Seeking past EOF should always return an error
        assert!(matches!(
            input.seek(len as u64 + random.gen_range(1..2048) as u64),
            Err(LuceneError::Eof(_))
        ));

        input.seek(len as u64)?;

        assert!(matches!(
            DataInput::read_byte(&mut input),
            Err(LuceneError::Eof(_))
        ));
        assert!(matches!(
            DataInput::read_bytes(&mut input, &mut [0u8; 1], 0, 1),
            Err(LuceneError::Eof(_))
        ));

        Ok(())
    }
    fn test_slice_out_of_bounds(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testSliceOutOfBounds")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let len = random.gen_range(8..2048);
        let buffer = vec![0u8; len];
        {
            let mut output = dir.create_output("out", new_io_context(random)?)?;
            output.write_bytes_range(&buffer, 0, len as u32)?;
        }

        let input = dir.open_input("out", new_io_context(random)?)?;

        assert!(matches!(
            input.slice("slice1", 0, len as u64 + 1),
            Err(LuceneError::IllegalArgument(_))
        ));

        let slice = input.slice("slice3", 4, (len / 2) as u64)?;

        // Attempting to create a nested slice that goes out of bounds
        assert!(matches!(
            slice.slice("slice3sub", 1, (len / 2) as u64),
            Err(LuceneError::IllegalArgument(_))
        ));

        Ok(())
    }
    #[allow(unused)]
    fn test_no_dir(&self) -> Result<(), TestError> {
        // TODO
        unimplemented!("DirectoryReader not Implemented")
    }
    fn test_copy_bytes(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("testCopyBytes").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let bytes_len = random.gen_range(1..=77777);
        let mut bytes = vec![0u8; bytes_len];

        let size = random.gen_range(1..=1777777);
        let mut upto = 0;
        let mut byte_upto = 0;
        {
            let mut output = dir.create_output("test", new_io_context(random)?)?;

            while upto < size {
                bytes[byte_upto] = Self::value(upto);
                byte_upto += 1;
                upto += 1;

                if byte_upto == bytes.len() {
                    output.write_bytes_range(&bytes, 0, bytes.len() as u32)?;
                    byte_upto = 0;
                }
            }

            output.write_bytes_range(&bytes, 0, byte_upto as u32)?;
            assert_eq!(size as u64, output.get_file_pointer());
        }
        assert_eq!(size as u64, dir.file_length("test")?);

        {
            let mut input = dir.open_input("test", new_io_context(random)?)?;
            let mut output = dir.create_output("test2", new_io_context(random)?)?;

            upto = 0;
            while upto < size {
                if random.gen_bool(0.5) {
                    output.write_byte(DataInput::read_byte(&mut input)?)?;
                    upto += 1;
                } else {
                    let chunk = std::cmp::min(random.gen_range(1..=bytes.len()), size - upto);
                    output.copy_bytes(&mut input, chunk as u64)?;
                    upto += chunk;
                }
            }
            assert_eq!(size, upto);
        }

        {
            let mut input2 = dir.open_input("test2", new_io_context(random)?)?;
            upto = 0;
            while upto < size {
                if random.gen_bool(0.5) {
                    let v = DataInput::read_byte(&mut input2)?;
                    assert_eq!(Self::value(upto), v);
                    upto += 1;
                } else {
                    let limit = std::cmp::min(random.gen_range(1..=bytes.len()), size - upto);
                    DataInput::read_bytes(&mut input2, &mut bytes, 0, limit as u32)?;
                    for &byte in bytes.iter().take(limit) {
                        assert_eq!(Self::value(upto), byte);
                        upto += 1;
                    }
                }
            }
        }

        dir.delete_file("test")?;
        dir.delete_file("test2")?;

        Ok(())
    }

    fn value(idx: usize) -> u8 {
        ((idx % 256) * (1 + (idx / 256))) as u8
    }

    fn test_copy_bytes_with_threads(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testCopyBytesWithThreads")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;
        let dir_new = Arc::new(Mutex::new(
            self.get_directory(temp_dir.path().to_path_buf())?,
        ));

        let header_len = 3;
        let data_len = random.gen_range(header_len + 1..10000);
        let mut data = vec![0u8; data_len];
        random.fill_bytes(&mut data);
        let data_clone = data.clone();

        {
            let mut output = dir.create_output("data", IOContext::default_io_context()?)?;
            output.write_bytes_with_len(&data, data_len as u32)?;
        }

        let mut input = dir.open_input("data", IOContext::default_io_context()?)?;

        {
            let mut output_header =
                dir.create_output("header", IOContext::default_io_context()?)?;
            output_header.copy_bytes(&mut input, header_len as u64)?;
        }

        let threads = 10;
        {
            let barrier = Arc::new(Barrier::new(threads));
            let mut handles = vec![];

            for i in 0..threads {
                let dir_clone = Arc::clone(&dir_new);
                let mut src = input.clone();
                let barrier_clone = Arc::clone(&barrier);
                let handle = thread::spawn(move || {
                    barrier_clone.wait();
                    let file_name = format!("copy{}", i);
                    let mut dir_guard = dir_clone.lock().unwrap();
                    let mut dst = dir_guard
                        .create_output(&file_name, IOContext::default_io_context().unwrap())
                        .unwrap();
                    let src_length = IndexInput::length(&src);
                    dst.copy_bytes(&mut src, src_length - header_len as u64)
                        .unwrap();
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        }

        let new_dir = self.get_directory(temp_dir.path().to_path_buf())?;
        for i in 0..threads {
            let file_name = format!("copy{}", i);
            let mut data_copy = vec![0u8; data_len];
            let mut input_copy =
                new_dir.open_input(&file_name, IOContext::default_io_context()?)?;

            data_copy[..header_len].copy_from_slice(&data_clone[..header_len]);

            DataInput::read_bytes(
                &mut input_copy,
                &mut data_copy[header_len..],
                0,
                (data_len - header_len) as u32,
            )?;

            assert_eq!(data_clone, data_copy, "Data mismatch in copy{}", i);
        }

        Ok(())
    }
    fn test_fsync_doesnt_create_new_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("nocreate").tempdir()?;
        let path = temp_dir.path().to_path_buf();

        let mut fsdir = self.get_directory(path.clone())?;

        // Ensure the directory is an FSDirectory subclass
        if !fsdir.is_fs_directory() {
            // This test only applies to FSDirectory-like implementations
            return Ok(());
        }

        {
            let mut out = fsdir.create_output("afile", new_io_context(random)?)?;
            out.write_string("boo")?;
        }

        // Delete the file directly via the filesystem
        std::fs::remove_file(path.join("afile"))?;

        let file_count_before = fsdir.list_all()?.len();

        let result = fsdir.sync(&["afile"]);
        assert!(matches!(
            result,
            Err(LuceneError::IoWithPath { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound
        ));

        // Ensure no new files were created
        let file_count_after = fsdir.list_all()?.len();
        assert_eq!(file_count_before, file_count_after);

        Ok(())
    }
    fn test_random_long(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("testLongs").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let num = random.gen_range(50..=3000);
        let mut longs = vec![0i64; num];
        {
            let mut output = dir.create_output("longs", new_io_context(random)?)?;
            for value in &mut longs {
                *value = random.gen_range(i64::MIN..=i64::MAX);
                output.write_long(*value)?;
            }
        }

        // Slice
        {
            let mut input = dir.open_input("longs", new_io_context(random)?)?;
            let length = IndexInput::length(&input);
            {
                let mut slice = input.random_access_slice(0, length)?;
                assert_eq!(length, RandomAccessInput::length(&slice));
                for (i, &expected) in longs.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_long(&mut slice, i as u64 * 8)?
                    );
                }
            }

            // Subslices
            for i in 1..longs.len() {
                let offset = i as u64 * 8;
                let mut subslice = input.random_access_slice(offset, length - offset)?;
                assert_eq!(length - offset, RandomAccessInput::length(&subslice));
                for (j, &expected) in longs.iter().skip(i).enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_long(&mut subslice, j as u64 * 8)?
                    );
                }
            }

            // With padding
            for i in 0..7 {
                let name = format!("longs-{}", i);
                {
                    let mut o = dir.create_output(&name, new_io_context(random)?)?;
                    let junk: Vec<u8> = (0..i).map(|_| random.gen()).collect();
                    o.write_bytes_with_len(&junk, junk.len() as u32)?;
                    input.seek(0)?;
                    let length = IndexInput::length(&input);
                    o.copy_bytes(&mut input, length)?;
                }

                let padded = dir.open_input(&name, new_io_context(random)?)?;
                let mut whole =
                    padded.random_access_slice(i as u64, IndexInput::length(&padded) - i as u64)?;
                assert_eq!(
                    IndexInput::length(&padded) - i as u64,
                    RandomAccessInput::length(&whole)
                );
                for (j, &expected) in longs.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_long(&mut whole, j as u64 * 8)?
                    );
                }
            }
        }

        Ok(())
    }
    fn test_random_int(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("testInts").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let num = random.gen_range(50..=3000);
        let mut ints = vec![0i32; num];
        {
            let mut output = dir.create_output("ints", new_io_context(random)?)?;
            for value in &mut ints {
                *value = random.gen_range(i32::MIN..=i32::MAX);
                output.write_int(*value)?;
            }
        }

        // Slice
        {
            let mut input = dir.open_input("ints", new_io_context(random)?)?;
            let length = IndexInput::length(&input);
            {
                let mut slice = input.random_access_slice(0, length)?;
                assert_eq!(length, RandomAccessInput::length(&slice));
                for (i, &expected) in ints.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_int(&mut slice, i as u64 * 4)?
                    );
                }
            }

            // Subslices
            for i in 1..ints.len() {
                let offset = i as u64 * 4;
                let mut subslice = input.random_access_slice(offset, length - offset)?;
                assert_eq!(length - offset, RandomAccessInput::length(&subslice));
                for (j, &expected) in ints.iter().skip(i).enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_int(&mut subslice, j as u64 * 4)?
                    );
                }
            }

            // With padding
            for i in 0..7 {
                let name = format!("ints-{}", i);
                {
                    let mut o = dir.create_output(&name, new_io_context(random)?)?;
                    let junk: Vec<u8> = (0..i).map(|_| random.gen()).collect();
                    o.write_bytes_with_len(&junk, junk.len() as u32)?;
                    input.seek(0)?;
                    let length = IndexInput::length(&input);
                    o.copy_bytes(&mut input, length)?;
                }

                let padded = dir.open_input(&name, new_io_context(random)?)?;
                let mut whole =
                    padded.random_access_slice(i as u64, IndexInput::length(&padded) - i as u64)?;
                assert_eq!(
                    IndexInput::length(&padded) - i as u64,
                    RandomAccessInput::length(&whole)
                );
                for (j, &expected) in ints.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_int(&mut whole, j as u64 * 4)?
                    );
                }
            }
        }

        Ok(())
    }

    fn test_random_short(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("testShorts").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let num = random.gen_range(50..=3000);
        let mut shorts = vec![0i16; num];
        {
            let mut output = dir.create_output("shorts", new_io_context(random)?)?;
            for value in &mut shorts {
                *value = random.gen_range(i16::MIN..=i16::MAX);
                output.write_short(*value)?;
            }
        }

        // Slice
        {
            let mut input = dir.open_input("shorts", new_io_context(random)?)?;
            let length = IndexInput::length(&input);
            {
                let mut slice = input.random_access_slice(0, length)?;
                assert_eq!(length, RandomAccessInput::length(&slice));
                for (i, &expected) in shorts.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_short(&mut slice, i as u64 * 2)?
                    );
                }
            }

            // Subslices
            for i in 1..shorts.len() {
                let offset = i as u64 * 2;
                let mut subslice = input.random_access_slice(offset, length - offset)?;
                assert_eq!(length - offset, RandomAccessInput::length(&subslice));
                for (j, &expected) in shorts.iter().skip(i).enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_short(&mut subslice, j as u64 * 2)?
                    );
                }
            }

            // With padding
            for i in 0..7 {
                let name = format!("shorts-{}", i);
                {
                    let mut o = dir.create_output(&name, new_io_context(random)?)?;
                    let junk: Vec<u8> = (0..i).map(|_| random.gen()).collect();
                    o.write_bytes_with_len(&junk, junk.len() as u32)?;
                    input.seek(0)?;
                    let length = IndexInput::length(&input);
                    o.copy_bytes(&mut input, length)?;
                }

                let padded = dir.open_input(&name, new_io_context(random)?)?;
                let mut whole =
                    padded.random_access_slice(i as u64, IndexInput::length(&padded) - i as u64)?;
                assert_eq!(
                    IndexInput::length(&padded) - i as u64,
                    RandomAccessInput::length(&whole)
                );
                for (j, &expected) in shorts.iter().enumerate() {
                    assert_eq!(
                        expected,
                        RandomAccessInput::read_short(&mut whole, j as u64 * 2)?
                    );
                }
            }
        }

        Ok(())
    }
    fn test_random_byte(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("testBytes").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let num = if is_night_mode() {
            random.gen_range(1000..=3000)
        } else {
            random.gen_range(50..=1000)
        };
        let mut bytes = vec![0u8; num];
        random.fill_bytes(&mut bytes);

        {
            let mut output = dir.create_output("bytes", new_io_context(random)?)?;
            for &byte in &bytes {
                output.write_byte(byte)?;
            }
        }

        // Slice

        let mut input = dir.open_input("bytes", new_io_context(random)?)?;
        let length = IndexInput::length(&input);
        {
            let mut slice = input.random_access_slice(0, length)?;
            assert_eq!(length, RandomAccessInput::length(&slice));
            Self::assert_bytes(&mut slice, &bytes, 0, random)?;
        }

        // Subslices
        let length = IndexInput::length(&input);
        for offset in 1..bytes.len() {
            let mut subslice = input.random_access_slice(offset as u64, length - offset as u64)?;
            assert_eq!(length - offset as u64, RandomAccessInput::length(&subslice));
            Self::assert_bytes(&mut subslice, &bytes, offset, random)?;
        }

        // With padding
        {
            for i in 1..7 {
                let name = format!("bytes-{}", i);
                {
                    let mut output = dir.create_output(&name, new_io_context(random)?)?;
                    let junk: Vec<u8> = (0..i).map(|_| random.gen()).collect();
                    output.write_bytes_with_len(&junk, junk.len() as u32)?;
                    let length = IndexInput::length(&input);
                    input.seek(0)?;
                    output.copy_bytes(&mut input, length)?;
                }

                let padded = dir.open_input(&name, new_io_context(random)?)?;
                let length = IndexInput::length(&padded);
                let mut whole = padded.random_access_slice(i as u64, length - i as u64)?;
                assert_eq!(length - i as u64, RandomAccessInput::length(&whole));
                Self::assert_bytes(&mut whole, &bytes, 0, random)?;
            }
        }

        Ok(())
    }
    fn assert_bytes(
        slice: &mut impl RandomAccessInput,
        bytes: &[u8],
        bytes_offset: usize,
        random: &mut StdRng,
    ) -> Result<(), TestError> {
        let to_read = bytes.len() - bytes_offset;

        for i in 0..to_read {
            assert_eq!(bytes[bytes_offset + i], slice.read_byte(i as u64)?);

            let offset = random.gen_range(0..1000);

            let mut sub1 = vec![0u8; offset + i];
            slice.read_bytes(0, &mut sub1, offset as u32, i as u32)?;
            assert_eq!(
                &bytes[bytes_offset..bytes_offset + i],
                &sub1[offset..offset + i]
            );

            let mut sub2 = vec![0u8; offset + to_read - i];
            slice.read_bytes(i as u64, &mut sub2, offset as u32, (to_read - i) as u32)?;
            assert_eq!(
                &bytes[bytes_offset + i..],
                &sub2[offset..offset + to_read - i]
            );
        }

        Ok(())
    }
    fn test_slice_of_slice(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("sliceOfSlice").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let num = if is_night_mode() {
            random.gen_range(250..=2500)
        } else {
            random.gen_range(50..=250)
        };

        let mut bytes = vec![0u8; num];
        random.fill_bytes(&mut bytes);

        {
            let mut output = dir.create_output("bytes", new_io_context(random)?)?;
            for &byte in &bytes {
                output.write_byte(byte)?;
            }
        }

        let mut input = dir.open_input("bytes", new_io_context(random)?)?;

        // Seek to a random spot to ensure it doesn't affect slicing
        input.seek(random.gen_range(0..=IndexInput::length(&input)))?;

        for i in (0..num).step_by(16) {
            let mut slice1 = input.slice("slice1", i as u64, (num - i) as u64)?;
            assert_eq!(0, slice1.get_file_pointer());
            assert_eq!((num - i) as u64, RandomAccessInput::length(&slice1));

            // Seek to a random spot to ensure it doesn't affect slicing
            slice1.seek(random.gen_range(0..=RandomAccessInput::length(&slice1)))?;

            for j in (0..RandomAccessInput::length(&slice1)).step_by(16) {
                let mut slice2 = slice1.slice("slice2", j, (num - i) as u64 - j)?;
                assert_eq!(0, slice2.get_file_pointer());
                assert_eq!((num - i) as u64 - j, RandomAccessInput::length(&slice2));

                let mut data = vec![0u8; num];
                data[..i + j as usize].copy_from_slice(&bytes[..i + j as usize]);

                if random.gen_bool(0.5) {
                    // Read the bytes for this slice-of-slice
                    DataInput::read_bytes(
                        &mut slice2,
                        &mut data[i + j as usize..],
                        0,
                        (num - i - j as usize) as u32,
                    )?;
                } else {
                    // Seek to a random spot in between, read some, seek back, and read the rest
                    let seek = random.gen_range(0..RandomAccessInput::length(&slice2));
                    slice2.seek(seek)?;
                    DataInput::read_bytes(
                        &mut slice2,
                        &mut data[(i + j as usize + seek as usize)..],
                        0,
                        (num - i - j as usize - seek as usize) as u32,
                    )?;
                    slice2.seek(0)?;
                    DataInput::read_bytes(
                        &mut slice2,
                        &mut data[i + j as usize..(i + j as usize + seek as usize)],
                        0,
                        seek as u32,
                    )?;
                }

                assert_eq!(bytes, data);
            }
        }

        Ok(())
    }
    /// This test verifies that writes larger than the size of the buffer output will correctly
    /// increment the file pointer.
    fn test_large_writes(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().prefix("largeWrites").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let mut output = dir.create_output("testBufferStart.txt", new_io_context(random)?)?;

        let mut large_buf = vec![0u8; 2048];
        random.fill_bytes(&mut large_buf);

        let current_pos = output.get_file_pointer();
        let large_buf_len = large_buf.len();
        output.write_bytes_with_len(&large_buf, large_buf_len as u32)?;

        assert_eq!(
            current_pos + large_buf.len() as u64,
            output.get_file_pointer()
        );
        Ok(())
    }
    /// This test verifies that the `to_string` implementation of `IndexOutput` contains the file name.
    fn test_index_output_to_string(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let output = dir.create_output("camelCase.txt", new_io_context(random)?)?;
        let output_description = output.to_string();
        assert!(
            output_description.contains("camelCase.txt"),
            "Expected `to_string` to contain 'camelCase.txt', but got: {}",
            output_description
        );
        Ok(())
    }
    /// This test ensures that double-closing an `IndexOutput` does not cause any issues.
    /// Rust Lucene automatically closes resources when they go out of scope, so this test is not applicable.
    #[allow(unused)]
    fn test_double_close_output(&self, _random: &mut StdRng) -> Result<(), TestError> {
        Ok(())
    }
    /// Rust Lucene automatically closes resources when they go out of scope, so this test is not applicable.
    #[allow(unused)]
    fn test_double_close_input(&self) -> Result<(), TestError> {
        Ok(())
    }
    /// This test ensures that `create_temp_output` generates unique files and writes/reads data correctly.
    fn test_create_temp_output(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let mut names = Vec::new();
        let iterations = random.gen_range(50..10000);

        for iter in 0..iterations {
            let mut output = dir.create_temp_output("foo", "bar", new_io_context(random)?)?;
            names.push(output.get_name().to_string());
            output.write_vint(iter)?;
        }

        for iter in 0..iterations {
            let mut input = dir.open_input(&names[iter as usize], new_io_context(random)?)?;
            assert_eq!({ iter }, input.read_vint()?);
        }

        // List all files in the directory, excluding files named "extra0"
        let files: HashSet<String> = dir
            .list_all()?
            .into_iter()
            .filter(|file| file != "extra0")
            .collect();

        // Verify that all created temp files exist in the directory
        assert_eq!(names.into_iter().collect::<HashSet<_>>(), files);

        Ok(())
    }
    /// This test ensures that attempting to create an output for an existing file results in an error,
    /// and after deleting the file, it can be created again.
    fn test_create_output_for_existing_file(&self) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let name = "file";

        {
            let output = dir.create_output(name, IOContext::default_io_context()?)?;
            assert_eq!(output.get_name(), name);
        }

        {
            // Attempt to create the same file again, which should fail
            let result = dir.create_output(name, IOContext::default_io_context()?);
            assert!(
                matches!(result, Err(LuceneError::IoWithPath { source, .. }) if source.kind() == std::io::ErrorKind::AlreadyExists)
            );
        }

        // Delete the file and attempt to recreate it
        dir.delete_file(name)?;
        dir.create_output(name, IOContext::default_io_context()?)?;

        Ok(())
    }

    fn test_seek_to_end_of_file(&self) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        {
            let mut out = dir.create_output("a", IOContext::default_io_context()?)?;
            for _ in 0..1024 {
                out.write_byte(0)?;
            }
        }

        {
            let mut input = dir.open_input("a", IOContext::default_io_context()?)?;
            input.seek(100)?;
            assert_eq!(100, input.get_file_pointer());

            input.seek(1024)?;
            assert_eq!(1024, input.get_file_pointer());
        }

        Ok(())
    }
    fn test_seek_beyond_end_of_file(&self) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new().tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        // Write a file with 1024 bytes
        {
            let mut out = dir.create_output("a", IOContext::default_io_context()?)?;
            for _ in 0..1024 {
                out.write_byte(0)?;
            }
        }

        // Test seeking within and beyond the file's end
        {
            let mut input = dir.open_input("a", IOContext::default_io_context()?)?;
            input.seek(100)?;
            assert_eq!(100, input.get_file_pointer());

            // Attempting to seek beyond the end of the file should return an EOF error
            assert!(matches!(input.seek(1025), Err(LuceneError::Eof(_))));
        }

        Ok(())
    }
    fn test_pending_deletions(&self, _random: &mut StdRng) -> Result<(), TestError> {
        // TODO: does not implemented "VirusCheckingFS" yet, so this test is not applicable
        // let temp_dir = tempfile::Builder::new().prefix("virusChecker").tempdir()?;
        // let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;
        //
        // // This test applies only to FSDirectory
        // if !dir.is_fs_directory() {
        //     return Ok(());
        // }
        //
        // let file_name: String;
        // loop {
        //     // create a random filename (segment file name style), so it cannot hit windows problem with
        //     // special filenames ("con", "com1",...):
        //     let candidate = IndexFileNames::segment_file_name(
        //         &TestUtil::random_simple_string_with_length(random, 1, 6),
        //         &TestUtil::random_simple_string(random),
        //         "test",
        //     );
        //
        //     {
        //         let out = dir.create_output(&candidate, IOContext::default_io_context()?)?;
        //         out.get_file_pointer(); // Just to mimic some usage
        //     }
        //     dir.delete_file(&candidate)?;
        //     if !dir.get_pending_deletions()?.is_empty() {
        //         // If the file couldn't be deleted due to "virus checker"
        //         file_name = candidate;
        //         break;
        //     }
        // }
        //
        // // Ensure `list_all` does not include the file
        // let files: HashSet<String> = dir.list_all()?.into_iter().collect();
        // assert!(!files.contains(&file_name));
        //
        // // Ensure `file_length` claims it's deleted
        // assert!(matches!(
        //     dir.file_length(&file_name),
        //     Err(LuceneError::IoWithPath { .. })
        // ));
        //
        // // Ensure `rename` fails
        // assert!(matches!(
        //     dir.rename(&file_name, "file2"),
        //     Err(LuceneError::IoWithPath { .. })
        // ));
        //
        // // Ensure `delete_file` fails
        // assert!(matches!(
        //     dir.delete_file(&file_name),
        //     Err(LuceneError::IoWithPath { .. })
        // ));
        //
        // // Ensure we cannot open it for reading
        // assert!(matches!(
        //     dir.open_input(&file_name, IOContext::default_io_context()?),
        //     Err(LuceneError::IoWithPath { .. })
        // ));

        Ok(())
    }
    fn test_list_all_is_sorted(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("test_list_all_is_sorted")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let count = random.gen_range(20..10000);
        let mut names = HashSet::new();

        let mut names_len = names.len();
        while names_len < count {
            let name = IndexFileNames::segment_file_name(
                &TestUtil::random_simple_string_with_length(random, 1, 6),
                &TestUtil::random_simple_string(random),
                "test",
            );

            if random.gen_range(0..5) == 1 {
                // Create a temporary output
                {
                    let output =
                        dir.create_temp_output(&name, "foo", IOContext::default_io_context()?)?;
                    let output_name = output.get_name().to_string();
                    names.insert(output_name);
                }
            } else if !names.contains(name.as_str()) {
                // Create a normal output
                {
                    let output = dir.create_output(&name, IOContext::default_io_context()?)?;
                    let output_name = output.get_name().to_string();
                    names.insert(output_name);
                }
            }
            names_len = names.len();
        }

        let actual: Vec<String> = dir.list_all()?;
        let mut expected = actual.clone();
        expected.sort();

        assert_eq!(expected, actual);

        Ok(())
    }
    fn test_data_types(&self) -> Result<(), TestError> {
        let mut values: [i64; 4] = [43, 12345, 123456, 1234567890];
        let temp_dir = tempfile::Builder::new()
            .prefix("test_data_types")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        {
            let mut out = dir.create_output("test", IOContext::default_io_context()?)?;
            out.write_byte(43u8)?;
            out.write_short(12345i16)?;
            out.write_int(1234567890i32)?;
            let values_len = values.len() as u32;
            out.write_group_vints(&mut values, values_len)?;
            out.write_long(1234567890123456789i64)?;
        }

        let mut restored = [0i64; 4];
        {
            let mut input = dir.open_input("test", IOContext::default_io_context()?)?;
            assert_eq!(43, DataInput::read_byte(&mut input)? as i32);
            assert_eq!(12345, DataInput::read_short(&mut input)? as i32);
            assert_eq!(1234567890, DataInput::read_int(&mut input)?);
            let restored_len = restored.len();
            GroupVIntUtil::read_group_vints(&mut input, &mut restored, restored_len as i32)?;
            assert_eq!(values, restored);
            assert_eq!(1234567890123456789, DataInput::read_long(&mut input)?);
        }

        Ok(())
    }
    fn test_group_vint_overflow(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir = tempfile::Builder::new()
            .prefix("testGroupVIntOverflow")
            .tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let size = 32;
        let mut values = vec![0i64; size];
        let mut restore = vec![0i64; size];
        values[0] = 1i64 << 31; // values[0] = 2147483648 as long, but as int it is -2147483648

        for i in 0..size {
            if random.gen_bool(0.5) {
                values[i] = values[0];
            }
        }

        // a smaller limit value covers the default implementation of read_group_vints,
        // and a bigger limit value covers the faster implementation.
        let values_len = values.len();
        let limit = random.gen_range(1..size);
        {
            let mut out = dir.create_output("test", IOContext::default_io_context()?)?;
            out.write_group_vints(&mut values[..values_len], limit as u32)?;
        }
        {
            let mut input = dir.open_input("test", IOContext::default_io_context()?)?;
            GroupVIntUtil::read_group_vints(&mut input, &mut restore, limit as i32)?;
            for i in 0..limit {
                assert_eq!(values[i], restore[i]);
            }
        }

        values[0] = (0xFFFFFFFF_u64 + 1) as i64;
        {
            let file_path = temp_dir.into_path();
            std::fs::remove_file(file_path.join("test"))?;
            let mut out = dir.create_output("test", IOContext::default_io_context()?)?;
            let result = out.write_group_vints(&mut values[..values_len], 4);
            assert!(matches!(result, Err(LuceneError::IntegerOverflow(_))));
        }

        Ok(())
    }
    fn test_group_vint(&self, random: &mut StdRng) -> Result<(), TestError> {
        let temp_dir1 = tempfile::Builder::new()
            .prefix("testGroupVInt1")
            .tempdir()?;
        let temp_dir2 = tempfile::Builder::new()
            .prefix("testGroupVInt2")
            .tempdir()?;
        let mut dir1 = self.get_directory(temp_dir1.path().to_path_buf())?;
        let mut dir2 = self.get_directory(temp_dir2.path().to_path_buf())?;

        // Test fallback to default implementation of readGroupVInt
        Self::do_test_group_vint(&mut dir1, &mut dir2, random, 5, 1, 6, 8)?;

        // Use more iterations to cover all bpv
        let iterations = random.gen_range(100..200); // Simulate `atLeast(100)`
        Self::do_test_group_vint(&mut dir1, &mut dir2, random, iterations, 1, 31, 128)?;

        // BaseChunkedDirectoryTestCase#testGroupVIntMultiBlocks covers multiple blocks
        // This part might be covered in another test or implementation
        Ok(())
    }
    fn do_test_group_vint(
        dir1: &mut impl Directory,
        dir2: &mut impl Directory,
        random: &mut StdRng,
        iterations: usize,
        min_bpv: usize,
        max_bpv: usize,
        max_num_values: usize,
    ) -> Result<(), TestError> {
        let mut values = vec![0i64; max_num_values];
        let mut num_values_array = vec![0usize; iterations];

        // Create output files
        {
            let mut group_vint_out =
                dir1.create_output("group-varint", IOContext::default_io_context()?)?;
            let mut vint_out = dir2.create_output("vint", IOContext::default_io_context()?)?;

            // Encode
            for num_values in num_values_array.iter_mut().take(iterations) {
                let bpv = random.gen_range(min_bpv..=max_bpv);
                *num_values = random.gen_range(1..=max_num_values);

                for value in values.iter_mut().take(*num_values) {
                    let upper = PackedInts::max_value(bpv as u32) as i32;
                    *value = if upper == 0 {
                        0
                    } else {
                        random.gen_range(0..=upper) as i64
                    };
                    vint_out.write_vint(*value as i32)?;
                }

                group_vint_out.write_group_vints(&mut values, *num_values as u32)?;
            }
        }

        // Decode
        {
            let mut group_vint_in =
                dir1.open_input("group-varint", IOContext::default_io_context()?)?;
            let mut vint_in = dir2.open_input("vint", IOContext::default_io_context()?)?;
            for &num_values in num_values_array.iter().take(iterations) {
                // 读取组 VInts
                GroupVIntUtil::read_group_vints(
                    &mut group_vint_in,
                    &mut values,
                    num_values as i32,
                )?;

                // 遍历 values 并比较
                for (j, &expected_value) in values.iter().take(num_values).enumerate() {
                    let vint_value = vint_in.read_vint()?;
                    assert_eq!(
                        vint_value as i64, expected_value,
                        "Mismatch at index {}: expected {}, got {}",
                        j, expected_value, vint_value
                    );
                }
            }
        }
        dir1.delete_file("group-varint")?;
        dir2.delete_file("vint")?;

        Ok(())
    }
    fn test_prefetch(&self, random: &mut StdRng) -> Result<(), TestError> {
        let start_offset = 0;
        let temp_dir = tempfile::Builder::new().prefix("test_prefetch").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let total_length = start_offset + random.gen_range(16384..=65536);
        // let mut arr = vec![0u8; total_length];
        let mut arr = vec![0u8; total_length];
        random.fill_bytes(&mut arr[..]);

        {
            let mut out = dir.create_output("temp.bin", IOContext::default_io_context()?)?;
            out.write_bytes_with_len(&arr, total_length as u32)?;
        }

        let mut temp = vec![0u8; 2048];

        let orig = dir.open_input("temp.bin", IOContext::default_io_context()?)?;
        let mut input = orig.clone();

        for _ in 0..10_000 {
            let offset = random.gen_range(0..(IndexInput::length(&input) as usize - 1)) as u64;

            if random.gen_bool(0.5) {
                let prefetch_length = random.gen_range(1..=(IndexInput::length(&input) - offset));
                input.prefetch(offset, prefetch_length)?;
            }

            input.seek(offset)?;
            assert_eq!(offset, input.get_file_pointer());

            match random.gen_range(3..100) {
                0 => {
                    let read_byte = DataInput::read_byte(&mut input)?;
                    assert_eq!(arr[start_offset + offset as usize], read_byte);
                }
                1 => {
                    if (IndexInput::length(&input) - offset) >= 8 {
                        let expected = i64::from_le_bytes(
                            arr[start_offset + offset as usize..start_offset + offset as usize + 8]
                                .try_into()
                                .unwrap(),
                        );
                        let read_long = DataInput::read_long(&mut input)?;
                        assert_eq!(expected, read_long);
                    }
                }
                _ => {
                    let read_length = random.gen_range(
                        1..=temp
                            .len()
                            .min((IndexInput::length(&input) - offset) as usize),
                    );
                    DataInput::read_bytes(
                        &mut input,
                        &mut temp[..read_length],
                        0,
                        read_length as u32,
                    )?;
                    assert_eq!(
                        &arr[start_offset + offset as usize
                            ..start_offset + offset as usize + read_length],
                        &temp[..read_length]
                    );
                }
            }
        }
        Ok(())
    }

    fn test_prefetch_on_slice(&self, random: &mut StdRng) -> Result<(), TestError> {
        let start_offset = random.gen_range(1..1024);
        let temp_dir = tempfile::Builder::new().prefix("test_prefetch").tempdir()?;
        let mut dir = self.get_directory(temp_dir.path().to_path_buf())?;

        let total_length = start_offset + random.gen_range(16384..=65536);
        // let mut arr = vec![0u8; total_length];
        let mut arr = vec![0u8; total_length];
        random.fill_bytes(&mut arr[..]);

        {
            let mut out = dir.create_output("temp.bin", IOContext::default_io_context()?)?;
            out.write_bytes_with_len(&arr, total_length as u32)?;
        }

        let mut temp = vec![0u8; 2048];

        let orig = dir.open_input("temp.bin", IOContext::default_io_context()?)?;
        let mut input = orig.slice(
            "slice",
            start_offset as u64,
            total_length as u64 - start_offset as u64,
        )?;

        for _ in 0..10_000 {
            let offset = random.gen_range(0..(IndexInput::length(&input) as usize - 1)) as u64;

            if random.gen_bool(0.5) {
                let prefetch_length = random.gen_range(1..=(IndexInput::length(&input) - offset));
                input.prefetch(offset, prefetch_length)?;
            }

            input.seek(offset)?;
            assert_eq!(offset, input.get_file_pointer());

            match random.gen_range(3..100) {
                0 => {
                    let read_byte = DataInput::read_byte(&mut input)?;
                    assert_eq!(arr[start_offset + offset as usize], read_byte);
                }
                1 => {
                    if (IndexInput::length(&input) - offset) >= 8 {
                        let expected = i64::from_le_bytes(
                            arr[start_offset + offset as usize..start_offset + offset as usize + 8]
                                .try_into()
                                .unwrap(),
                        );
                        let read_long = DataInput::read_long(&mut input)?;
                        assert_eq!(expected, read_long);
                    }
                }
                _ => {
                    let read_length = random.gen_range(
                        1..=temp
                            .len()
                            .min((IndexInput::length(&input) - offset) as usize),
                    );
                    DataInput::read_bytes(
                        &mut input,
                        &mut temp[..read_length],
                        0,
                        read_length as u32,
                    )?;
                    assert_eq!(
                        &arr[start_offset + offset as usize
                            ..start_offset + offset as usize + read_length],
                        &temp[..read_length]
                    );
                }
            }
        }
        Ok(())
    }
}

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
use crate::util::lucene_test_case::{new_directory, new_io_context, slow_file_exists};
use crate::util::test_error::TestError;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::store::check_sum_index_input::ChecksumIndexInput;
use rlucene::store::directory::Directory;
use rlucene::store::DataInput;
use rlucene::store::IndexInput;
use rlucene::store::{DataOutput, IOContext};
use rlucene::util::error::data_io_error_enum::DataIOError;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tempfile::Builder;

pub trait BaseDirectoryTestCase {
    fn get_directory(&self, path: PathBuf) -> Result<impl Directory, TestError>;

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
            input.read_bytes(&mut bytes2, 0, bytes2_len as u32)?;
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
            input.read_bytes(&mut bytes2, 0, num_bytes as u32)?;
            assert_eq!(input.length(), num_bytes as u64);
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
            Err(DataIOError::IoWithPath {
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
            assert_eq!(1, input.length());
            assert_eq!(128, input.read_byte()?);
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
            assert_eq!(2, input.length());
            assert_eq!(-20, input.read_short()?);
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
            assert_eq!(4, input.length());
            assert_eq!(-500, input.read_int()?);
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
            assert_eq!(8, input.length());
            assert_eq!(-5000, input.read_long()?);
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
            assert_eq!(24, input.length());

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
            assert_eq!(25, input.length());
            assert_eq!(2, input.read_byte()?);
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
            assert!(matches!(result, Err(DataIOError::Eof(_))));
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
            assert_eq!(12, input.length());
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
            assert_eq!(12 + padding, input.length());
            for _ in 0..padding {
                assert_eq!(2, input.read_byte()?);
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
        assert!(matches!(result, Err(DataIOError::Eof(_))));

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
            assert_eq!(12, input.length());
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
        assert_eq!(12 + padding as u64, input.length());
        for _ in 0..padding {
            assert_eq!(2, input.read_byte()?);
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
        assert!(matches!(result, Err(DataIOError::Eof(_))));

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
            assert_eq!(7, input.length());
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
            assert_eq!(2, input.length());
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
            assert_eq!(9, input.length());
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
            assert_eq!(input.length(), input.get_file_pointer());
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
            assert_eq!(input.length(), input.get_file_pointer());
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

            assert_eq!(input.length(), input.get_file_pointer());
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

            assert_eq!(input.length(), input.get_file_pointer());
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
            IndexInput::skip_bytes(&mut input,num_bytes as u64)?;
            let actual_checksum = input.get_checksum();
            assert_eq!(expected_checksum as u64, actual_checksum);
        }

        Ok(())
    }
}

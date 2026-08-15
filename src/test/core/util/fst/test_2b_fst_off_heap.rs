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

// Similar to Test2BFST but builds and reads the FST off-heap and can be run with a small heap.
//
// Run something like this:
// cargo test test_2b_fst_off_heap::test -- --ignored --nocapture

use std::sync::Arc;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use crate::core::index::BytesRef;
use crate::core::store::directory::Directory;
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::store::{IOContext, IndexInput};
use crate::core::util::IOUtils;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::core::util::fst_impl::fst::{FST, InputType};
use crate::core::util::fst_impl::fst_compiler::{Builder, DataOutputEnum};
use crate::core::util::fst_impl::ints_ref_fst_enum::IntsRefFSTEnum;
use crate::core::util::fst_impl::no_outputs::NoOutputs;
use crate::core::util::fst_impl::off_heap_fst_store::OffHeapFSTStore;
use crate::core::util::fst_impl::outputs::Outputs;
use crate::core::util::fst_impl::positive_int_outputs::PositiveIntOutputs;
use crate::core::util::fst_impl::util::Util;
use crate::core::util::ints_ref::IntsRef;
use crate::test_framework::core::util::lucene_test_case::{create_temp_dir_with_prefix, random};

#[allow(dead_code)] // for quick search
struct Test2BFSTOffHeap;

const LIMIT: i64 = 3 * 1024 * 1024 * 1024;

#[test]
#[ignore = "takes about 4.5 hours"]
fn test() -> Result<()> {
  let mut ints = vec![0; 7];
  let mut input = IntsRef::from_slice(ints.clone(), 0, ints.len());
  let seed = random().random::<u64>();

  let temp_dir = create_temp_dir_with_prefix("2BFSTOffHeap")?;
  let dir = MMapDirectory::new(temp_dir.path().to_path_buf())?;

  // Build FST with NoOutputs and stop when nodeCount > 2.2B.
  {
    println!("\nTEST: ~2.2B nodes; output=NO_OUTPUTS");
    let outputs = NoOutputs::get_singleton().clone();
    let no_output = outputs.get_no_output();
    let index_output = dir.create_output("fst", &IOContext::default_io_context()?)?;
    let mut builder = Builder::new(InputType::Byte1, outputs.clone());
    builder.data_output(DataOutputEnum::FromDir(index_output));
    let mut fst_compiler = builder.build()?;

    let mut count = 0;
    let mut r = StdRng::seed_from_u64(seed);
    let mut ints2 = vec![0; 200];
    let mut input2 = IntsRef::from_slice(ints2.clone(), 0, ints2.len());
    let mut start_time = Instant::now();
    loop {
      // println!("add: {input} -> {output}");
      for value in &mut ints2[10..] {
        *value = r.random_range(0..256);
      }
      input2.ints.clone_from(&ints2);
      fst_compiler.add(&input2, no_output.clone())?;
      count += 1;
      if count % 100_000 == 0 {
        println!(
          "{}: {} RAM bytes used; {} FST bytes; {} nodes; took {} seconds",
          count,
          fst_compiler.fst_ram_bytes_used()?,
          fst_compiler.fst_size_in_bytes(),
          fst_compiler.get_node_count(),
          start_time.elapsed().as_secs()
        );
      }
      if fst_compiler.get_node_count() > i32::MAX as i64 + 100 * 1024 * 1024 {
        break;
      }
      next_input(&mut r, &mut ints2);
    }

    let metadata = fst_compiler.compile()?.unwrap();
    fst_compiler.data_output.close()?;
    let index_input = Arc::new(dir.open_input("fst", &IOContext::default_io_context()?)?);
    let offset = index_input.get_file_pointer()?;
    let num_bytes = metadata.num_bytes as usize;
    let mut fst = FST::from_fst_reader(
      metadata,
      OffHeapFSTStore::new(index_input.clone(), offset, num_bytes),
    )
    .unwrap();

    let body_result = (|| -> Result<()> {
      for _verify in 0..2 {
        println!(
          "\nTEST: now verify [fst size={}; nodeCount={}; arcCount={}]",
          fst.num_bytes(),
          fst_compiler.get_node_count(),
          fst_compiler.get_arc_count()
        );
        ints2.fill(0);
        input2.ints.clone_from(&ints2);
        r = StdRng::seed_from_u64(seed);
        start_time = Instant::now();
        for i in 0..count {
          if i % 1_000_000 == 0 {
            println!("{}...: took {} seconds", i, start_time.elapsed().as_secs());
          }
          for value in &mut ints2[10..] {
            *value = r.random_range(0..256);
          }
          input2.ints.clone_from(&ints2);
          assert_eq!(Some(no_output.clone()), Util::get_from_ints(&fst, &input2)?);
          next_input(&mut r, &mut ints2);
        }

        println!("\nTEST: enum all input/outputs");
        let mut fst_enum = IntsRefFSTEnum::new(fst)?;
        ints2.fill(0);
        r = StdRng::seed_from_u64(seed);
        let mut upto = 0;
        while let Some(pair) = fst_enum.next_value()? {
          for value in &mut ints2[10..] {
            *value = r.random_range(0..256);
          }
          input2.ints.clone_from(&ints2);
          assert_eq!(input2, pair.input);
          assert_eq!(no_output, pair.output);
          upto += 1;
          next_input(&mut r, &mut ints2);
        }
        assert_eq!(count, upto);
        fst = fst_enum.base.fst;
      }
      Ok(())
    })();
    let body_result = IOUtils::use_or_suppress_result(body_result, index_input.close());
    let delete_result = dir.delete_file("fst");
    delete_result?;
    body_result?;
  }

  // Build FST with ByteSequenceOutputs and stop when FST size = 3 GB.
  {
    println!("\nTEST: 3 GB size; outputs=bytes");
    let index_output = dir.create_output("fst", &IOContext::default_io_context()?)?;
    let outputs = ByteSequenceOutputs::get_singleton().clone();
    let mut builder = Builder::new(InputType::Byte1, outputs.clone());
    builder.data_output(DataOutputEnum::FromDir(index_output));
    let mut fst_compiler = builder.build()?;

    let mut output_bytes = vec![0; 20];
    ints.fill(0);
    input.ints.clone_from(&ints);
    let mut count = 0;
    let mut r = StdRng::seed_from_u64(seed);
    loop {
      r.fill(&mut output_bytes[..]);
      // println!("add: {input} -> {output}");
      let output = BytesRef::from_bytes(Arc::new(output_bytes.clone()));
      fst_compiler.add(&input, output)?;
      count += 1;
      if count % 10_000 == 0 {
        let size = fst_compiler.fst_size_in_bytes();
        if count % 1_000_000 == 0 {
          println!("{count}...: {size} bytes");
        }
        if size > LIMIT {
          break;
        }
      }
      next_input(&mut r, &mut ints);
      input.ints.clone_from(&ints);
    }

    let metadata = fst_compiler.compile()?.unwrap();
    fst_compiler.data_output.close()?;
    let index_input = Arc::new(dir.open_input("fst", &IOContext::default_io_context()?)?);
    let offset = index_input.get_file_pointer()?;
    let num_bytes = metadata.num_bytes as usize;
    let mut fst = FST::from_fst_reader(
      metadata,
      OffHeapFSTStore::new(index_input.clone(), offset, num_bytes),
    )
    .unwrap();

    let body_result = (|| -> Result<()> {
      for _verify in 0..2 {
        println!(
          "\nTEST: now verify [fst size={}; nodeCount={}; arcCount={}]",
          fst.num_bytes(),
          fst_compiler.get_node_count(),
          fst_compiler.get_arc_count()
        );
        r = StdRng::seed_from_u64(seed);
        ints.fill(0);
        input.ints.clone_from(&ints);
        let start_time = Instant::now();
        for i in 0..count {
          if i % 1_000_000 == 0 {
            println!("{}...: took {} seconds", i, start_time.elapsed().as_secs());
          }
          r.fill(&mut output_bytes[..]);
          let output = BytesRef::from_bytes(Arc::new(output_bytes.clone()));
          assert_eq!(Some(output), Util::get_from_ints(&fst, &input)?);
          next_input(&mut r, &mut ints);
          input.ints.clone_from(&ints);
        }

        println!("\nTEST: enum all input/outputs");
        let mut fst_enum = IntsRefFSTEnum::new(fst)?;
        ints.fill(0);
        input.ints.clone_from(&ints);
        r = StdRng::seed_from_u64(seed);
        let mut upto = 0;
        while let Some(pair) = fst_enum.next_value()? {
          assert_eq!(input, pair.input);
          r.fill(&mut output_bytes[..]);
          let output = BytesRef::from_bytes(Arc::new(output_bytes.clone()));
          assert_eq!(output, pair.output);
          upto += 1;
          next_input(&mut r, &mut ints);
          input.ints.clone_from(&ints);
        }
        assert_eq!(count, upto);
        fst = fst_enum.base.fst;
      }
      Ok(())
    })();
    let body_result = IOUtils::use_or_suppress_result(body_result, index_input.close());
    let delete_result = dir.delete_file("fst");
    delete_result?;
    body_result?;
  }

  // Build FST with PositiveIntOutputs and stop when FST size = 3 GB.
  {
    let index_output = dir.create_output("fst", &IOContext::default_io_context()?)?;
    println!("\nTEST: 3 GB size; outputs=long");
    let outputs = PositiveIntOutputs::get_singleton().clone();
    let mut builder = Builder::new(InputType::Byte1, outputs.clone());
    builder.data_output(DataOutputEnum::FromDir(index_output));
    let mut fst_compiler = builder.build()?;

    let mut output = 1_i64;
    ints.fill(0);
    input.ints.clone_from(&ints);
    let mut count = 0;
    let mut r = StdRng::seed_from_u64(seed);
    loop {
      // println!("add: {input} -> {output}");
      fst_compiler.add(&input, Arc::new(output))?;
      output += 1 + r.random_range(0..10);
      count += 1;
      if count % 10_000 == 0 {
        let size = fst_compiler.fst_size_in_bytes();
        if count % 1_000_000 == 0 {
          println!("{count}...: {size} bytes");
        }
        if size > LIMIT {
          break;
        }
      }
      next_input(&mut r, &mut ints);
      input.ints.clone_from(&ints);
    }

    let metadata = fst_compiler.compile()?.unwrap();
    fst_compiler.data_output.close()?;
    let index_input = Arc::new(dir.open_input("fst", &IOContext::default_io_context()?)?);
    let offset = index_input.get_file_pointer()?;
    let num_bytes = metadata.num_bytes as usize;
    let mut fst = FST::from_fst_reader(
      metadata,
      OffHeapFSTStore::new(index_input.clone(), offset, num_bytes),
    )
    .unwrap();

    let body_result = (|| -> Result<()> {
      for _verify in 0..2 {
        println!(
          "\nTEST: now verify [fst size={}; nodeCount={}; arcCount={}]",
          fst.num_bytes(),
          fst_compiler.get_node_count(),
          fst_compiler.get_arc_count()
        );
        ints.fill(0);
        input.ints.clone_from(&ints);
        output = 1;
        r = StdRng::seed_from_u64(seed);
        let start_time = Instant::now();
        for i in 0..count {
          if i % 1_000_000 == 0 {
            println!("{}...: took {} seconds", i, start_time.elapsed().as_secs());
          }
          assert_eq!(Some(Arc::new(output)), Util::get_from_ints(&fst, &input)?);
          output += 1 + r.random_range(0..10);
          next_input(&mut r, &mut ints);
          input.ints.clone_from(&ints);
        }

        println!("\nTEST: enum all input/outputs");
        let mut fst_enum = IntsRefFSTEnum::new(fst)?;
        ints.fill(0);
        input.ints.clone_from(&ints);
        r = StdRng::seed_from_u64(seed);
        let mut upto = 0;
        output = 1;
        while let Some(pair) = fst_enum.next_value()? {
          assert_eq!(input, pair.input);
          assert_eq!(output, *pair.output);
          output += 1 + r.random_range(0..10);
          upto += 1;
          next_input(&mut r, &mut ints);
          input.ints.clone_from(&ints);
        }
        assert_eq!(count, upto);
        fst = fst_enum.base.fst;
      }
      Ok(())
    })();
    let body_result = IOUtils::use_or_suppress_result(body_result, index_input.close());
    let delete_result = dir.delete_file("fst");
    delete_result?;
    body_result?;
  }
  dir.close()
}

fn next_input(r: &mut StdRng, ints: &mut [i32]) {
  let mut down_to = 6_i32;
  while down_to >= 0 {
    // Must add random amounts (and not just 1) because
    // otherwise FST outsmarts us and remains tiny.
    ints[down_to as usize] += 1 + r.random_range(0..10);
    if ints[down_to as usize] < 256 {
      break;
    } else {
      ints[down_to as usize] = 0;
      down_to -= 1;
    }
  }
}

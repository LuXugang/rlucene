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
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ptr;
use std::rc::Rc;

use rand::Rng;

use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::test::util::lucene_test_case::at_least;
use crate::test::util::test_util::TestUtil;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::util::fst_impl::dummy::dummy_fst_reader::DummyFSTReader;
use crate::util::fst_impl::fst::{fst_util, Arc, FSTMetadata, InputType, FST};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::ints_ref_fst_enum::IntsRefFSTEnum;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::ints_ref::IntsRef;
use crate::util::ints_ref_builder::IntsRefBuilder;

pub struct FSTTester<D, T, R, O, S>
where
    D: Directory,
    T: OutputsBound,
    R: Rng,
    O: Outputs<T>,
    S: FSTTesterBase,
{
    pub random: R,
    pub pairs: Vec<InputOutput<T>>,
    pub input_mode: i32,
    pub outputs: O,
    pub dir: Rc<RefCell<D>>,

    pub node_count: i64,
    pub arc_count: i64,
    pub sub: Option<S>,
}
impl<D, T, R, O, S> FSTTester<D, T, R, O, S>
where
    D: Directory,
    T: OutputsBound,
    R: Rng,
    O: Outputs<T>,
    S: FSTTesterBase,
{
    // runs the term, returning the output, or null if term
    // isn't accepted.  if prefixLength is non-null it must be
    // length 1 int array; prefixLength[0] is set to the length
    // of the term prefix that matches
    pub fn run<F>(
        fst: &mut FST<T, O, F>,
        term: &IntsRef<Vec<i32>>,
        mut prefix_length: Option<&mut [i32]>,
    ) -> Result<Option<T>>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        assert!(prefix_length.is_none() || prefix_length.as_ref().unwrap().len() == 1);
        let mut arc = Arc::default();
        fst.get_first_arc(&mut arc);
        let mut output = fst.outputs.get_no_output();
        let mut reader = fst.get_bytes_reader()?;

        for i in 0..=term.length {
            let label = if i == term.length {
                fst_util::END_LABEL
            } else {
                term.ints[term.offset + i]
            };

            let find = fst.find_target_arc(label, &arc.clone(), &mut arc, &mut reader)?;
            if find.is_none() {
                if prefix_length.is_some() {
                    prefix_length.as_mut().unwrap()[0] = i as i32;
                    return Ok(Some(output));
                } else {
                    return Ok(None);
                }
            }

            output = fst.outputs.add(&output, &arc.output());
        }
        if prefix_length.is_some() {
            prefix_length.as_mut().unwrap()[0] = term.length as i32;
        }

        Ok(Some(output))
    }
    pub fn random_accepted_word<F>(
        fst: &mut FST<T, O, F>,
        in_builder: &mut IntsRefBuilder<Vec<i32>>,
        random: &mut R,
    ) -> Result<T>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
        R: Rng,
    {
        let mut arc = Arc::default();
        fst.get_first_arc(&mut arc);
        let mut arcs = Vec::new();
        in_builder.clear();
        let mut output = fst.outputs.get_no_output();
        let mut reader = fst.get_bytes_reader()?;

        loop {
            fst.read_first_target_arc(&arc.clone(), &mut arc, &mut reader)?;
            let mut new_arc = Arc::default();
            new_arc.copy_from(&arc);
            arcs.push(new_arc);
            while !arc.is_last() {
                fst.read_next_arc(&mut arc, &mut reader)?;
                let mut new_arc = Arc::default();
                new_arc.copy_from(&arc);
                arcs.push(new_arc);
            }
            let idx = random.random_range(0..arcs.len());
            arc = arcs[idx].clone();
            arcs.clear();
            output = fst.outputs.add(&output, &arc.output());

            if arc.label() == fst_util::END_LABEL {
                break;
            }
            in_builder.append(arc.label());
        }

        Ok(output)
    }
    pub fn verify_unpruned<F>(
        &self,
        input_mode: i32,
        fst: Option<FST<T, O, F>>,
        pairs: &[InputOutput<T>],
        outputs: &O,
        random: &mut R,
    ) -> Result<()>
    where
        T: OutputsBound + PartialEq + std::fmt::Debug,
        O: Outputs<T>,
        F: FstReader,
    {
        if pairs.is_empty() {
            assert!(fst.is_none(), "FST should be None for empty input");
            return Ok(());
        }

        let mut fst_enum = IntsRefFSTEnum::new(fst.unwrap())?;

        for pair in pairs {
            let term = &pair.input;
            let output = FSTTester::<D, T, R, O, S>::run(
                &mut fst_enum.base.as_mut().unwrap().fst,
                term,
                None,
            )?;
            assert!(
                output.is_some(),
                "term {} is not accepted",
                fst_tester_util::input_to_string(input_mode, term, true)?
            );
            assert!(self.outputs_equal(&pair.output, output.as_ref().unwrap()));

            let t = fst_enum.next()?;
            assert!(t.is_some(), "expected more terms");
            let t = t.unwrap();
            assert_eq!(
                &*t.input.borrow(),
                term,
                "expected input={} but got {}",
                fst_tester_util::input_to_string_with_term(input_mode, term,)?,
                fst_tester_util::input_to_string_with_term(input_mode, &t.input.borrow(),)?
            );
            assert_eq!(
                &t.output,
                &pair.output,
                "output mismatch at input={}",
                fst_tester_util::input_to_string(input_mode, term, true)?
            );
        }

        assert!(fst_enum.next()?.is_none(), "expected no more terms at end");
        let mut terms_map: HashMap<IntsRef<Vec<i32>>, T> = HashMap::new();
        for pair in pairs {
            terms_map.insert(pair.input.clone(), pair.output.clone());
        }

        if cfg!(feature = "test_log_verbose") {
            println!("TEST: verify random accepted terms");
        }

        let mut scratch = IntsRefBuilder::default();
        let num = at_least(random, 500);
        for _ in 0..num {
            let output = FSTTester::<D, T, R, O, S>::random_accepted_word(
                &mut fst_enum.base.as_mut().unwrap().fst,
                &mut scratch,
                random,
            )?;
            let key = scratch.get();
            let expected = terms_map.get(&key).expect(&format!(
                "accepted word {} is not valid",
                fst_tester_util::input_to_string(input_mode, &key, true)?
            ));
            assert!(
                self.outputs_equal(expected, &output),
                "mismatched output for {}",
                fst_tester_util::input_to_string(input_mode, &key, true)?
            );
        }

        #[cfg(debug_assertions)]
        {
            println!("TEST: verify seek");
        }

        // let mut fst_enum = IntsRefFSTEnum::new(fst)?;
        // let num_seek = at_least(random, 100);

        Ok(())
    }
    fn outputs_equal(&self, a: &T, b: &T) -> bool
    where
        T: OutputsBound,
    {
        if self.sub.is_some() {
            self.sub.as_ref().unwrap().outputs_equal_impl(a, b)
        } else {
            ptr::eq(a, b)
        }
    }
}
pub trait FSTTesterBase {
    fn outputs_equal_impl<T>(&self, a: &T, b: &T) -> bool
    where
        T: OutputsBound;
}
pub mod fst_tester_util {
    use rand::Rng;

    use crate::index::BytesRef;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::ints_ref::IntsRef;
    use crate::util::ints_ref_builder::IntsRefBuilder;
    use crate::util::unicode_util::UnicodeUtil;

    pub fn input_to_string_with_term(input_mode: i32, term: &IntsRef<Vec<i32>>) -> Result<String> {
        input_to_string(input_mode, term, true)
    }

    pub fn input_to_string(
        input_mode: i32,
        term: &IntsRef<Vec<i32>>,
        is_valid_unicode: bool,
    ) -> Result<String> {
        if !is_valid_unicode {
            Ok(term.to_string())
        } else if input_mode == 0 {
            // utf8
            let br = get_bytes_ref(term);
            return Ok(format!("{} {:?}", br.utf8_to_string()?, term));
        } else {
            let s = UnicodeUtil::new_string(&term.ints, term.offset, term.length)?;
            Ok(format!("{} {:?}", s, term))
        }
    }

    pub fn get_bytes_ref(ir: &IntsRef<Vec<i32>>) -> BytesRef<Vec<u8>> {
        let len = ir.length;
        let mut bytes = vec![0u8; len];

        for i in 0..len {
            let x = ir.ints[ir.offset + i];
            assert!((0..=255).contains(&x), "x={} out of range", x);
            bytes[i] = x as u8;
        }
        BytesRef {
            bytes,
            offset: 0,
            length: len,
        }
    }
    pub fn get_random_string<R: Rng>(random: &mut R) -> String {
        if random.random_bool(0.5) {
            TestUtil::random_realistic_unicode_string(random)
        } else {
            simple_random_string(random)
        }
    }
    pub fn simple_random_string<R: Rng>(rng: &mut R) -> String {
        let end = rng.random_range(0..11);
        if end == 10 {
            // allow 0 length
            return String::new();
        }

        let mut buffer = String::with_capacity(end);
        for _ in 0..end {
            let c = rng.random_range(97..=102) as u8 as char; // 'a' to 'f'
            buffer.push(c);
        }

        buffer
    }
    pub fn to_ints_ref_from_string(s: &str, input_mode: i32) -> IntsRef<Vec<i32>> {
        let mut ir = IntsRefBuilder::default();
        to_ints_ref_from_string_with_builder(s, input_mode, &mut ir)
    }

    pub fn to_ints_ref_from_string_with_builder(
        s: &str,
        input_mode: i32,
        ir: &mut IntsRefBuilder<Vec<i32>>,
    ) -> IntsRef<Vec<i32>> {
        if input_mode == 0 {
            // utf8
            let br = BytesRef::from_string(s);
            to_ints_ref(&br, ir)
        } else {
            // utf32
            to_ints_ref_utf32(s, ir)
        }
    }

    pub fn to_ints_ref_utf32(s: &str, ir: &mut IntsRefBuilder<Vec<i32>>) -> IntsRef<Vec<i32>> {
        ir.clear();
        for c in s.chars() {
            ir.append(c as i32);
        }
        ir.get().clone()
    }

    pub fn to_ints_ref_from_bytes(
        br: &BytesRef<Vec<u8>>,
        ir: &mut IntsRefBuilder<Vec<i32>>,
    ) -> IntsRef<Vec<i32>> {
        ir.clear();
        ir.grow_no_copy(br.length);
        for i in 0..br.length {
            let byte = br.bytes[br.offset + i];
            ir.append(byte as i32);
        }
        ir.get_owner()
    }
    pub fn to_ints_ref(
        br: &BytesRef<Vec<u8>>,
        ir: &mut IntsRefBuilder<Vec<i32>>,
    ) -> IntsRef<Vec<i32>> {
        ir.grow_no_copy(br.length);
        ir.clear();
        for i in 0..br.length {
            ir.append(br.bytes[br.offset + i] as i32);
        }
        ir.get_owner()
    }
}
#[derive(Debug, Clone)]
pub struct InputOutput<T>
where
    T: OutputsBound,
{
    pub input: IntsRef<Vec<i32>>,
    pub output: T,
}

impl<T> InputOutput<T>
where
    T: OutputsBound,
{
    pub fn new(input: IntsRef<Vec<i32>>, output: T) -> Self {
        Self { input, output }
    }
}
impl<T: PartialEq> PartialEq for InputOutput<T>
where
    T: OutputsBound,
{
    fn eq(&self, other: &Self) -> bool {
        self.input == other.input
    }
}

impl<T: Eq> Eq for InputOutput<T> where T: OutputsBound {}

impl<T: Ord> PartialOrd<Self> for InputOutput<T>
where
    T: OutputsBound,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for InputOutput<T>
where
    T: OutputsBound,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.input.cmp(&other.input)
    }
}

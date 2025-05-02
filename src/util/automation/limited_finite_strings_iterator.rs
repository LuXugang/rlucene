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
use std::borrow::Cow;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::finite_strings_iterator::{
    FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::ints_ref::IntsRef;

#[derive(Debug)]
pub struct LimitedFiniteStringsIterator<'a> {
    limit: i32,
    count: i32,
    base: FiniteStringsIterator<'a>,
}
impl<'a> LimitedFiniteStringsIterator<'a> {
    pub fn new(automaton: &'a Automaton, limit: i32) -> Result<Self> {
        if limit != -1 && limit <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "limit must be -1 (which means no limit), or > 0; got: {}",
                limit
            )));
        }

        Ok(Self {
            limit: if limit > 0 { limit } else { i32::MAX },
            count: 0,
            base: FiniteStringsIterator::new(automaton),
        })
    }

    /// Number of iterated finite strings so far
    pub fn size(&self) -> i32 {
        self.count
    }
}
impl FiniteStringsIteratorBase for LimitedFiniteStringsIterator<'_> {
    fn next(&mut self) -> Result<Option<Cow<IntsRef<Vec<i32>>>>> {
        if self.count >= self.limit {
            return Ok(None);
        }

        if let Some(result) = self.base.next()? {
            self.count += 1;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::test::util::automaton::automaton_test_util::AutomatonTestUtil;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::automation::automata::Automata;
    use crate::util::automation::finite_strings_iterator::tests::get_finite_strings;
    use crate::util::automation::limited_finite_strings_iterator::LimitedFiniteStringsIterator;
    use crate::util::automation::operations::Operations;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::fst_impl::util::Util;
    use crate::util::ints_ref_builder::IntsRefBuilder;
    #[allow(dead_code)] // for quick search
    struct TestLimitedFiniteStringsIterator;
    #[test]
    fn test_random_finite_strings() -> Result<()> {
        let mut random = random();
        // Just makes sure we can run on any random finite
        // automaton:
        let iters = at_least(&mut random, 1000);
        for _ in 0..iters {
            let limit = TestUtil::next_int(&mut random, 1, 1000);
            let a = AutomatonTestUtil::random_automaton(&mut random)?;
            let mut v = LimitedFiniteStringsIterator::new(&a, limit).unwrap();
            // Must pass a limit because the random automaton
            // can accept MANY strings:
            let result = get_finite_strings(&mut v);
            // NOTE: cannot do this, because the method is not
            // guaranteed to detect cycles when you have a limit
            // assertTrue(AutomatonTestUtil.isFinite(a));
            if result.is_err() {
                // TODO: 没能验证这个assert 需要等待RegExp实现
                assert!(!AutomatonTestUtil::is_finite(&a)?);
            }
        }

        Ok(())
    }

    #[test]
    fn test_invalid_limit_negative() -> Result<()> {
        let mut random = random();
        let a = AutomatonTestUtil::random_automaton(&mut random)?;

        let err = LimitedFiniteStringsIterator::new(&a, -7);
        assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
        assert!(err.unwrap_err().to_string().contains("limit must be -1"));
        Ok(())
    }

    #[test]
    fn test_invalid_limit_null() -> Result<()> {
        let mut random = random();
        let a = AutomatonTestUtil::random_automaton(&mut random)?;

        let err = LimitedFiniteStringsIterator::new(&a, 0);
        assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
        assert!(err.unwrap_err().to_string().contains("limit must be -1"));
        Ok(())
    }

    #[test]
    fn test_singleton() -> Result<()> {
        let a = Automata::make_string("foobar")?;
        let mut iterator = LimitedFiniteStringsIterator::new(&a, 1)?;
        let actual = get_finite_strings(&mut iterator)?;
        assert_eq!(1, actual.len());

        let mut scratch = IntsRefBuilder::new();
        Util::get_utf32_with_slice("foobar", 0, 6, &mut scratch);
        assert!(actual.contains(scratch.get()));

        Ok(())
    }

    #[test]
    fn test_limit() -> Result<()> {
        let a = Operations::union(
            &Automata::make_string("foo")?,
            &Automata::make_string("bar")?,
        )?;

        // Test without limit
        let mut without_limit = LimitedFiniteStringsIterator::new(&a, -1)?;
        let actual1 = get_finite_strings(&mut without_limit)?;
        assert_eq!(2, actual1.len());

        // Test with limit
        let mut with_limit = LimitedFiniteStringsIterator::new(&a, 1)?;
        let actual2 = get_finite_strings(&mut with_limit)?;
        assert_eq!(1, actual2.len());

        Ok(())
    }

    #[test]
    fn test_size() -> Result<()> {
        let a = Operations::union(
            &Automata::make_string("foo")?,
            &Automata::make_string("bar")?,
        )?;

        let mut iterator = LimitedFiniteStringsIterator::new(&a, -1)?;
        let actual = get_finite_strings(&mut iterator)?;
        assert_eq!(2, actual.len());
        assert_eq!(2, iterator.size());

        Ok(())
    }
}

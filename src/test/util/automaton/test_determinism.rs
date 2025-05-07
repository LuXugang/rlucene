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
/// Not completely thorough, but tries to test determinism correctness somewhat
/// randomly.
#[allow(dead_code)] // for quick search
pub struct TestDeterminism;

#[cfg(test)]
mod tests {
    use crate::test::util::automaton::automaton_test_util::AutomatonTestUtil;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::automation::operations::Operations;
    use crate::util::error::lucene_error::Result;
    /// test a bunch of random regular expressions
    fn test_regexps() -> Result<()> {
        // TODO: RegExp not Implement
        Ok(())
    }
    /// test against a simple, unoptimized det
    #[test]
    fn test_against_simple() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 200);

        for _ in 0..num {
            let a0 = AutomatonTestUtil::random_automaton(&mut random)?;
            let a = AutomatonTestUtil::determinize_simple(&a0)?;
            let b = Operations::determinize(&a, usize::MAX)?;
            assert!(AutomatonTestUtil::same_language(&a, &b)?);
        }

        Ok(())
    }
}

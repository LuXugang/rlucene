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
#[allow(dead_code)]
struct TestDeterminizeLexicon;
#[cfg(test)]
mod tests {
    use rand::prelude::SliceRandom;
    use rand::rngs::StdRng;

    use crate::test::util::automaton::automaton_test_util::AutomatonTestUtil;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::automation::automata::Automata;
    use crate::util::automation::automaton::Automaton;
    use crate::util::automation::byte_run_automaton::ByteRunAutomaton;
    use crate::util::automation::byte_runnable::ByteRunnable;
    use crate::util::automation::operations::Operations;
    use crate::util::error::lucene_error::Result;
    #[test]
    fn test_determinize_lexicon() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 1);

        for _ in 0..num {
            let mut automata = Vec::with_capacity(5000);
            let mut terms = Vec::with_capacity(5000);

            for _ in 0..5000 {
                let s = TestUtil::random_unicode_string(&mut random);
                let a = Automata::make_string(&s)?;
                automata.push(a);
                terms.push(s);
            }

            assert_lexicon(&mut random, &terms, &mut automata)?;
        }
        Ok(())
    }

    fn assert_lexicon(
        random: &mut StdRng,
        terms: &[String],
        automata: &mut [Automaton],
    ) -> Result<()> {
        let mut automata = automata.to_vec();
        automata.shuffle(random);
        let lex = Operations::union_list(&automata.iter().collect::<Vec<_>>())?;
        let lex = Operations::determinize(&lex, 1_000_000)?;
        assert!(AutomatonTestUtil::is_finite(&lex)?);

        for s in terms {
            assert!(Operations::run_str(&lex, s));
        }
        if cfg!(feature = "nightly") {
            let lex_byte = ByteRunAutomaton::new(lex.into_owned())?;
            for s in terms {
                let bytes = s.as_bytes();
                assert!(lex_byte.run(bytes, 0, bytes.len()));
            }
        }
        Ok(())
    }
}

/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
/// Not thorough, but tries to test determinism correctness somewhat randomly,
/// by determinizing a huge random lexicon.
#[allow(dead_code)]
struct TestDeterminizeLexicon;
#[cfg(test)]
mod tests {
    use rand::prelude::SliceRandom;
    use rand::Rng;

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

    fn assert_lexicon<R: Rng + ?Sized>(
        random: &mut R,
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

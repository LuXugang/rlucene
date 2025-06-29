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
/// Not completely thorough, but tries to test determinism correctness somewhat
/// randomly.
#[allow(dead_code)] // for quick search
pub struct TestDeterminism;

#[cfg(test)]
mod tests {
    use crate::test::util::automaton::automaton_test_util::AutomatonTestUtil;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::automation::automata::Automata;
    use crate::util::automation::automaton::Automaton;
    use crate::util::automation::operations::Operations;
    use crate::util::automation::reg_exp::RegExp;
    use crate::util::error::lucene_error::Result;
    /// test a bunch of random regular expressions
    #[test]
    fn test_regexps() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 500);
        for _ in 0..num {
            let pattern = AutomatonTestUtil::random_regexp(&mut random)?;
            let re = RegExp::parse(&pattern, RegExp::NONE, 0)?;
            let a = re.to_automaton()?;
            assert_automaton(&a)?;
        }
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
    pub fn assert_automaton(a: &Automaton) -> Result<()> {
        let v = Operations::remove_dead_states(a)?;
        let a = Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

        // complement(complement(a)) == a
        let equivalent = {
            let tmp = Operations::complement(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
            Operations::complement(&tmp, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        };
        assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

        // a union a == a
        let union = Operations::union(&a, &a)?;
        let reduced = Operations::remove_dead_states(&union)?;
        let equivalent =
            Operations::determinize(&reduced, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
        assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

        // a intersect a == a
        let inter = Operations::intersection(&a, &a)?;
        let reduced = Operations::remove_dead_states(&inter)?;
        let equivalent =
            Operations::determinize(&reduced, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
        assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

        // a - a == empty
        let empty = Operations::minus(&a, &a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
        assert!(Operations::is_empty(&empty));

        // if a doesn't accept empty string: optional(a) - ε == a
        if !Operations::run_str(&a, "") {
            let optional = Operations::optional(&a)?;
            let epsilon = Automata::make_empty_string()?;
            let equivalent = Operations::minus(
                &optional,
                &epsilon,
                Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
            )?;
            assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);
        }

        Ok(())
    }
}

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
use crate::core::analysis::char_array_map::CharArrayMap;
use std::fmt::Display;

/// A simple class that stores Strings as char[]'s in a hash table.
pub struct CharArraySet {
    map: CharArrayMap<()>,
}
impl CharArraySet {
    pub fn new(ignore: bool) -> Self {
        CharArraySet {
            map: CharArrayMap::new(ignore),
        }
    }
    pub fn add_all<I, S>(&mut self, iter: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for s in iter {
            self.add_str(s.as_ref());
        }
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }
    pub fn contains_key(&self, key: &[char], off: i32, len: i32) -> bool {
        self.map.contains_key(key, off, len)
    }
    pub fn contains_key_str(&self, key: &str) -> bool {
        self.map.contains_key_str(key)
    }
    pub fn contains_key_any<T>(&self, key: &T) -> bool
    where
        T: Display,
    {
        self.map.contains_key_any(key)
    }

    pub fn add(&mut self, key: &[char]) -> Option<()> {
        self.map.put(key, ())
    }
    pub fn add_str(&mut self, key: &str) -> Option<()> {
        self.map.put_str(key, ())
    }
    pub fn add_any<T>(&mut self, key: &T) -> Option<()>
    where
        T: Display,
    {
        self.map.put_any(key, ())
    }
    pub fn get(&self, key: &[char]) -> Option<&()> {
        self.map.get(key)
    }
    pub fn get_str(&self, key: &str) -> Option<&()> {
        self.map.get_str(key)
    }
    pub fn get_any<T>(&self, key: &T) -> Option<&()>
    where
        T: Display,
    {
        self.map.get_any(key)
    }
    pub fn size(&self) -> usize {
        self.map.size()
    }
}
#[cfg(test)]
mod tests {
    use crate::core::analysis::char_array_set::CharArraySet;

    #[allow(dead_code)]
    struct TestCharArraySet;
    static TEST_STOP_WORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
        "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
        "these", "they", "this", "to", "was", "will", "with",
    ];
    fn test_rehash() {
        // not required in Rust Lucene
    }

    #[test]
    fn test_non_zero_offset() {
        let words = ["Hello", "World", "this", "is", "a", "test"];
        let findme: Vec<char> = "xthisy".chars().collect();
        let mut set = CharArraySet::new(true);
        set.add_all(words);
        assert!(set.contains_key(&findme, 1, 4));
        assert!(set.contains_key_str("this"));
        let unmodifiable = set; // same to Java unmodifiable

        assert!(unmodifiable.contains_key(&findme, 1, 4));
        assert!(unmodifiable.contains_key_str("this"));
    }

    #[test]
    fn test_object_contains() {
        let mut set = CharArraySet::new(true);
        let val = 1;
        set.add_any(&val);
        assert!(set.contains_key_any(&val));
        assert!(set.contains_key_str("1"));
        let chars: Vec<char> = vec!['1'];
        assert!(set.contains_key(chars.as_slice(), 0, 1));
        let unmodifiable = set;

        assert!(unmodifiable.contains_key_any(&val));
        assert!(unmodifiable.contains_key_str("1"));
        assert!(unmodifiable.contains_key(&chars, 0, 1));
    }
    #[test]
    fn test_clear() {
        let mut set = CharArraySet::new(true);
        set.add_all(TEST_STOP_WORDS);
        assert_eq!(TEST_STOP_WORDS.len(), set.size(), "Not all words added");
        set.clear();
        assert_eq!(0, set.size(), "not empty after clear");
        for w in TEST_STOP_WORDS {
            assert!(!set.contains_key_str(w));
        }
        set.add_all(TEST_STOP_WORDS);
        assert_eq!(
            TEST_STOP_WORDS.len(),
            set.size(),
            "Not all words added after re-adding"
        );
        for w in TEST_STOP_WORDS {
            assert!(set.contains_key_str(w));
        }
    }

    #[test]
    fn test_modify_on_unmodifiable() {
        // TODO
    }
    #[test]
    fn test_supplementary_chars() {
        let missing = "Term {term} is missing in the set";
        let false_pos = "Term {term} is in the set but shouldn't";
        let upper_arr = ["Abc\u{1041C}", "\u{1041C}\u{1041C}CDE", "A\u{1041C}B"];
        let lower_arr = ["abc\u{10444}", "\u{10444}\u{10444}cde", "a\u{10444}b"];

        let mut set = CharArraySet::new(true);
        set.add_all(TEST_STOP_WORDS);
        for u in upper_arr {
            set.add_str(u);
        }
        for i in 0..upper_arr.len() {
            assert!(
                set.contains_key_str(upper_arr[i]),
                "{}",
                missing.replace("{term}", upper_arr[i])
            );
            assert!(
                set.contains_key_str(lower_arr[i]),
                "{}",
                missing.replace("{term}", lower_arr[i])
            );
        }

        let mut set = CharArraySet::new(false);
        set.add_all(TEST_STOP_WORDS);
        for u in &upper_arr {
            set.add_str(u);
        }
        for i in 0..upper_arr.len() {
            assert!(
                set.contains_key_str(upper_arr[i]),
                "{}",
                missing.replace("{term}", upper_arr[i])
            );
            assert!(
                !set.contains_key_str(lower_arr[i]),
                "{}",
                false_pos.replace("{term}", lower_arr[i])
            );
        }
    }
    #[test]
    fn test_single_high_surrogate() {
        // this test is not required in Rust Lucene
    }
}

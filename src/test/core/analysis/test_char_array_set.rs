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
use crate::core::analysis::char_array_set::CharArraySet;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)]
struct TestCharArraySet;
static TEST_STOP_WORDS: &[&str] = &[
  "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it",
  "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these", "they",
  "this", "to", "was", "will", "with",
];
#[test]
fn test_rehash() -> Result<()> {
  let mut cas = CharArraySet::new(true);
  for stop_word in TEST_STOP_WORDS {
    cas.add_str(stop_word);
  }
  assert_eq!(TEST_STOP_WORDS.len(), cas.size());
  for test_stop_word in TEST_STOP_WORDS {
    assert!(cas.contains_key_str(test_stop_word));
  }
  Ok(())
}

#[test]
fn test_non_zero_offset() -> Result<()> {
  let words = ["Hello", "World", "this", "is", "a", "test"];
  let findme: Vec<char> = "xthisy".chars().collect();
  let mut set = CharArraySet::new(true);
  set.add_all(words);
  assert!(set.contains_key(&findme, 1, 4));
  assert!(set.contains_key_str("this"));

  // TODO: Retest these lookups through an unmodifiable view after CharArraySet::unmodifiable_set
  // is migrated.
  Ok(())
}

#[test]
fn test_object_contains() -> Result<()> {
  let mut set = CharArraySet::new(true);
  let val = 1;
  set.add_any(&val);
  assert!(set.contains_key_any(&val));
  assert!(set.contains_key_str("1"));
  let chars: Vec<char> = vec!['1'];
  assert!(set.contains_key(chars.as_slice(), 0, 1));

  // TODO: Retest these lookups through an unmodifiable view after CharArraySet::unmodifiable_set
  // is migrated.
  Ok(())
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
fn test_modify_on_unmodifiable() -> Result<()> {
  // TODO: CharArraySet::unmodifiable_set and its unsupported-operation behavior have not been
  // migrated.
  Ok(())
}

#[test]
fn test_unmodifiable_set() -> Result<()> {
  // TODO: CharArraySet::unmodifiable_set and Java's null argument behavior have not been migrated.
  Ok(())
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
#[ignore = "Java-only: Rust strings cannot contain isolated UTF-16 surrogate code units"]
fn test_single_high_surrogate() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_copy_char_array_set_bw_compat() {
  let mut set_ignore_case = CharArraySet::new(true);
  let mut set_case_sensitive = CharArraySet::new(false);

  let stopwords_upper: Vec<String> = TEST_STOP_WORDS
    .iter()
    .map(|stop_word| stop_word.to_uppercase())
    .collect();
  set_ignore_case.add_all(TEST_STOP_WORDS);
  set_ignore_case.add_any(&1);
  set_case_sensitive.add_all(TEST_STOP_WORDS);
  set_case_sensitive.add_any(&1);

  let mut copy = set_ignore_case.clone();
  let copy_case_sens = set_case_sensitive.clone();

  assert_eq!(set_ignore_case.size(), copy.size());
  assert_eq!(set_case_sensitive.size(), copy.size());

  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    stopwords_upper
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy_case_sens.contains_key_str(stop_word))
  );
  for stop_word in &stopwords_upper {
    assert!(!copy_case_sens.contains_key_str(stop_word));
  }
  // test adding terms to the copy
  let new_words: Vec<String> = TEST_STOP_WORDS
    .iter()
    .map(|stop_word| format!("{stop_word}_1"))
    .collect();
  copy.add_all(&new_words);

  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    stopwords_upper
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    new_words
      .iter()
      .all(|new_word| copy.contains_key_str(new_word))
  );
  // new added terms are not in the source set
  for new_word in &new_words {
    assert!(!set_ignore_case.contains_key_str(new_word));
    assert!(!set_case_sensitive.contains_key_str(new_word));
  }
}

/// Tests copying a `CharArraySet` source.
#[test]
fn test_copy_char_array_set() {
  let mut set_ignore_case = CharArraySet::new(true);
  let mut set_case_sensitive = CharArraySet::new(false);

  let stopwords_upper: Vec<String> = TEST_STOP_WORDS
    .iter()
    .map(|stop_word| stop_word.to_uppercase())
    .collect();
  set_ignore_case.add_all(TEST_STOP_WORDS);
  set_ignore_case.add_any(&1);
  set_case_sensitive.add_all(TEST_STOP_WORDS);
  set_case_sensitive.add_any(&1);

  let mut copy = set_ignore_case.clone();
  let copy_case_sens = set_case_sensitive.clone();

  assert_eq!(set_ignore_case.size(), copy.size());
  assert_eq!(set_case_sensitive.size(), copy.size());

  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    stopwords_upper
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy_case_sens.contains_key_str(stop_word))
  );
  for stop_word in &stopwords_upper {
    assert!(!copy_case_sens.contains_key_str(stop_word));
  }
  // test adding terms to the copy
  let new_words: Vec<String> = TEST_STOP_WORDS
    .iter()
    .map(|stop_word| format!("{stop_word}_1"))
    .collect();
  copy.add_all(&new_words);

  assert!(
    TEST_STOP_WORDS
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    stopwords_upper
      .iter()
      .all(|stop_word| copy.contains_key_str(stop_word))
  );
  assert!(
    new_words
      .iter()
      .all(|new_word| copy.contains_key_str(new_word))
  );
  // new added terms are not in the source set
  for new_word in &new_words {
    assert!(!set_ignore_case.contains_key_str(new_word));
    assert!(!set_case_sensitive.contains_key_str(new_word));
  }
}

/// Tests copying a JDK `Set` source.
#[test]
#[ignore = "Java-only: Rust has no JDK Set implementation or Java collection-copy overload"]
fn test_copy_jdk_set() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

/// Tests the special case of copying `CharArraySet::EMPTY_SET`.
#[test]
#[ignore = "Java-only: Rust exposes an owned empty value instead of Java's shared EMPTY_SET singleton"]
fn test_copy_empty_set() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

/// Smoke-tests the static empty set.
#[test]
fn test_empty_set() {
  let empty_set = CharArraySet::empty_set();
  assert_eq!(0, empty_set.size());

  for stop_word in TEST_STOP_WORDS {
    assert!(!empty_set.contains_key_str(stop_word));
  }
  assert!(!empty_set.contains_key_str("foo"));
  assert!(!empty_set.contains_key_any(&"foo"));
  let foo: Vec<char> = "foo".chars().collect();
  assert!(!empty_set.contains_key(&foo, 0, 3));
}

/// Tests null handling.
#[test]
#[ignore = "Java-only: Rust references cannot represent Java null arguments"]
fn test_contains_with_null() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_to_string() {
  let mut set = CharArraySet::new(false);
  set.add_str("test");
  assert_eq!("[test]", set.to_string());
  set.add_str("test2");
  assert!(set.to_string().contains(", "));
}

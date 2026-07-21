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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::standard::standard_analyzer::{
  DEFAULT_MAX_TOKEN_LENGTH, StandardAnalyzer,
};
use crate::core::analysis::standard::standard_tokenizer::StandardTokenizer;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::close::Closeable;
use crate::test_framework::core::analysis::base_token_stream_test_case::{
  assert_analyzes_to6, assert_analyzes_to7, assert_analyzes_to9, assert_token_stream_contents12,
  check_one_term,
};
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;

use crate::core::util::error::lucene_error::Result;
use rand::RngExt;

#[allow(dead_code)]
struct TestStandardAnalyzer;

fn set_up() -> StandardAnalyzer {
  // TODO IMPORTANT 应该支持跟多的 attribute factory
  StandardAnalyzer::new()
}

// LUCENE-5897: slow tokenization of strings of the form
// (\p{WB:ExtendNumLet}[\p{WB:Format}\p{WB:Extend}]*)+
#[test]
fn test_large_partially_matching_token() -> Result<()> {
  // TODO: get these lists of chars matching a property from ICU4J
  // http://www.unicode.org/Public/6.3.0/ucd/auxiliary/WordBreakProperty.txt
  let word_break_extend_num_let_chars: Vec<char> =
    "_\u{203f}\u{2040}\u{2054}\u{fe33}\u{fe34}\u{fe4d}\u{fe4e}\u{fe4f}\u{ff3f}"
      .chars()
      .collect();

  // http://www.unicode.org/Public/6.3.0/ucd/auxiliary/WordBreakProperty.txt
  let word_break_format_chars = [
    0xAD, 0x600, 0x61C, 0x6DD, 0x70F, 0x180E, 0x200E, 0x202A, 0x2060, 0x2066, 0xFEFF, 0xFFF9,
    0x110BD, 0x1D173, 0xE0001, 0xE0020,
  ];

  // http://www.unicode.org/Public/6.3.0/ucd/auxiliary/WordBreakProperty.txt
  let word_break_extend_chars = [
    0x300, 0x483, 0x591, 0x5bf, 0x5c1, 0x5c4, 0x5c7, 0x610, 0x64b, 0x670, 0x6d6, 0x6df, 0x6e7,
    0x6ea, 0x711, 0x730, 0x7a6, 0x7eb, 0x816, 0x81b, 0x825, 0x829, 0x859, 0x8e4, 0x900, 0x93a,
    0x93e, 0x951, 0x962, 0x981, 0x9bc, 0x9be, 0x9c7, 0x9cb, 0x9d7, 0x9e2, 0xa01, 0xa3c, 0xa3e,
    0xa47, 0xa4b, 0xa51, 0xa70, 0xa75, 0xa81, 0xabc, 0xabe, 0xac7, 0xacb, 0xae2, 0xb01, 0xb3c,
    0xb3e, 0xb47, 0xb4b, 0xb56, 0xb62, 0xb82, 0xbbe, 0xbc6, 0xbca, 0xbd7, 0xc01, 0xc3e, 0xc46,
    0xc4a, 0xc55, 0xc62, 0xc82, 0xcbc, 0xcbe, 0xcc6, 0xcca, 0xcd5, 0xce2, 0xd02, 0xd3e, 0xd46,
    0xd4a, 0xd57, 0xd62, 0xd82, 0xdca, 0xdcf, 0xdd6, 0xdd8, 0xdf2, 0xe31, 0xe34, 0xe47, 0xeb1,
    0xeb4, 0xebb, 0xec8, 0xf18, 0xf35, 0xf37, 0xf39, 0xf3e, 0xf71, 0xf86, 0xf8d, 0xf99, 0xfc6,
    0x102b, 0x1056, 0x105e, 0x1062, 0x1067, 0x1071, 0x1082, 0x108f, 0x109a, 0x135d, 0x1712, 0x1732,
    0x1752, 0x1772, 0x17b4, 0x17dd, 0x180b, 0x18a9, 0x1920, 0x1930, 0x19b0, 0x19c8, 0x1a17, 0x1a55,
    0x1a60, 0x1a7f, 0x1b00, 0x1b34, 0x1b6b, 0x1b80, 0x1ba1, 0x1be6, 0x1c24, 0x1cd0, 0x1cd4, 0x1ced,
    0x1cf2, 0x1dc0, 0x1dfc, 0x200c, 0x20d0, 0x2cef, 0x2d7f, 0x2de0, 0x302a, 0x3099, 0xa66f, 0xa674,
    0xa69f, 0xa6f0, 0xa802, 0xa806, 0xa80b, 0xa823, 0xa880, 0xa8b4, 0xa8e0, 0xa926, 0xa947, 0xa980,
    0xa9b3, 0xaa29, 0xaa43, 0xaa4c, 0xaa7b, 0xaab0, 0xaab2, 0xaab7, 0xaabe, 0xaac1, 0xaaeb, 0xaaf5,
    0xabe3, 0xabec, 0xfb1e, 0xfe00, 0xfe20, 0xff9e, 0x101fd, 0x10a01, 0x10a05, 0x10a0c, 0x10a38,
    0x10a3f, 0x11000, 0x11001, 0x11038, 0x11080, 0x11082, 0x110b0, 0x110b3, 0x110b7, 0x110b9,
    0x11100, 0x11127, 0x1112c, 0x11180, 0x11182, 0x111b3, 0x111b6, 0x111bf, 0x116ab, 0x116ac,
    0x116b0, 0x116b6, 0x16f51, 0x16f8f, 0x1d165, 0x1d167, 0x1d16d, 0x1d17b, 0x1d185, 0x1d1aa,
    0x1d242, 0xe0100,
  ];

  let mut random = random();
  let mut builder = String::new();
  let num_chars = TestUtil::next_int(&mut random, 100 * 1024, 1024 * 1024) as usize;
  let mut i = 0;
  while i < num_chars {
    let ch = word_break_extend_num_let_chars
      [random.random_range(0..word_break_extend_num_let_chars.len())];
    builder.push(ch);
    i += ch.len_utf16();
    if random.random_bool(0.5) {
      let num_format_extend_chars = TestUtil::next_int(&mut random, 1, 8);
      for _ in 0..num_format_extend_chars {
        let code_point = if random.random_bool(0.5) {
          word_break_format_chars[random.random_range(0..word_break_format_chars.len())]
        } else {
          word_break_extend_chars[random.random_range(0..word_break_extend_chars.len())]
        };
        let ch = char::from_u32(code_point).unwrap();
        builder.push(ch);
        i += ch.len_utf16();
      }
    }
  }

  let mut tokenizer = StandardTokenizer::new();
  tokenizer.set_reader(builder.as_str().into())?;
  tokenizer.reset()?;
  while tokenizer.increment_token()? {}
  tokenizer.end()?;
  tokenizer.close()?;

  let new_buffer_size = TestUtil::next_int(&mut random, 200, 8192) as usize;
  tokenizer.set_max_token_length(new_buffer_size)?;
  tokenizer.set_reader(builder.into())?;
  tokenizer.reset()?;
  while tokenizer.increment_token()? {}
  tokenizer.end()?;
  tokenizer.close()
}

#[test]
fn test_huge_doc() -> Result<()> {
  let mut input = " ".repeat(4094);
  input.push_str("testing 1234");
  let mut tokenizer = StandardTokenizer::new();
  tokenizer.set_reader(input.into())?;
  assert_token_stream_contents12(&mut tokenizer, &["testing", "1234"])
}

#[test]
fn test_armenian() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "Վիքիպեդիայի 13 միլիոն հոդվածները (4,600` հայերեն վիքիպեդիայում) գրվել են կամավորների կողմից ու համարյա բոլոր հոդվածները կարող է խմբագրել ցանկաց մարդ ով կարող է բացել Վիքիպեդիայի կայքը։",
    &[
      "վիքիպեդիայի",
      "13",
      "միլիոն",
      "հոդվածները",
      "4,600",
      "հայերեն",
      "վիքիպեդիայում",
      "գրվել",
      "են",
      "կամավորների",
      "կողմից",
      "ու",
      "համարյա",
      "բոլոր",
      "հոդվածները",
      "կարող",
      "է",
      "խմբագրել",
      "ցանկաց",
      "մարդ",
      "ով",
      "կարող",
      "է",
      "բացել",
      "վիքիպեդիայի",
      "կայքը",
    ],
  )?;

  Ok(())
}

#[test]
fn test_amharic() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "ዊኪፔድያ የባለ ብዙ ቋንቋ የተሟላ ትክክለኛና ነጻ መዝገበ ዕውቀት (ኢንሳይክሎፒዲያ) ነው። ማንኛውም",
    &[
      "ዊኪፔድያ",
      "የባለ",
      "ብዙ",
      "ቋንቋ",
      "የተሟላ",
      "ትክክለኛና",
      "ነጻ",
      "መዝገበ",
      "ዕውቀት",
      "ኢንሳይክሎፒዲያ",
      "ነው",
      "ማንኛውም",
    ],
  )?;

  Ok(())
}
#[test]
fn test_arabic() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "الفيلم الوثائقي الأول عن ويكيبيديا يسمى \"الحقيقة بالأرقام: قصة ويكيبيديا\" (بالإنجليزية: Truth in Numbers: The Wikipedia Story)، سيتم إطلاقه في 2008.",
    &[
      "الفيلم",
      "الوثائقي",
      "الأول",
      "عن",
      "ويكيبيديا",
      "يسمى",
      "الحقيقة",
      "بالأرقام",
      "قصة",
      "ويكيبيديا",
      "بالإنجليزية",
      "truth",
      "in",
      "numbers",
      "the",
      "wikipedia",
      "story",
      "سيتم",
      "إطلاقه",
      "في",
      "2008",
    ],
  )?;

  Ok(())
}

#[test]
fn test_aramaic() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "ܘܝܩܝܦܕܝܐ (ܐܢܓܠܝܐ: Wikipedia) ܗܘ ܐܝܢܣܩܠܘܦܕܝܐ ܚܐܪܬܐ ܕܐܢܛܪܢܛ ܒܠܫܢ̈ܐ ܣܓܝܐ̈ܐ܂ ܫܡܗ ܐܬܐ ܡܢ ܡ̈ܠܬܐ ܕ\"ܘܝܩܝ\" ܘ\"ܐܝܢܣܩܠܘܦܕܝܐ\"܀",
    &[
      "ܘܝܩܝܦܕܝܐ",
      "ܐܢܓܠܝܐ",
      "wikipedia",
      "ܗܘ",
      "ܐܝܢܣܩܠܘܦܕܝܐ",
      "ܚܐܪܬܐ",
      "ܕܐܢܛܪܢܛ",
      "ܒܠܫܢ̈ܐ",
      "ܣܓܝܐ̈ܐ",
      "ܫܡܗ",
      "ܐܬܐ",
      "ܡܢ",
      "ܡ̈ܠܬܐ",
      "ܕ",
      "ܘܝܩܝ",
      "ܘ",
      "ܐܝܢܣܩܠܘܦܕܝܐ",
    ],
  )?;

  Ok(())
}

#[test]
fn test_bengali() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "এই বিশ্বকোষ পরিচালনা করে উইকিমিডিয়া ফাউন্ডেশন (একটি অলাভজনক সংস্থা)। উইকিপিডিয়ার শুরু ১৫ জানুয়ারি, ২০০১ সালে। এখন পর্যন্ত ২০০টিরও বেশী ভাষায় উইকিপিডিয়া রয়েছে।",
    &[
      "এই",
      "বিশ্বকোষ",
      "পরিচালনা",
      "করে",
      "উইকিমিডিয়া",
      "ফাউন্ডেশন",
      "একটি",
      "অলাভজনক",
      "সংস্থা",
      "উইকিপিডিয়ার",
      "শুরু",
      "১৫",
      "জানুয়ারি",
      "২০০১",
      "সালে",
      "এখন",
      "পর্যন্ত",
      "২০০টিরও",
      "বেশী",
      "ভাষায়",
      "উইকিপিডিয়া",
      "রয়েছে",
    ],
  )?;

  Ok(())
}
#[test]
fn test_farsi() -> Result<()> {
  let a = set_up();
  let mut random = random();
  let input =
    "ویکی پدیای انگلیسی در تاریخ ۲۵ دی ۱۳۷۹ به صورت مکملی برای دانشنامهٔ تخصصی نوپدیا نوشته شد.";
  let expected = [
    "ویکی",
    "پدیای",
    "انگلیسی",
    "در",
    "تاریخ",
    "۲۵",
    "دی",
    "۱۳۷۹",
    "به",
    "صورت",
    "مکملی",
    "برای",
    "دانشنامهٔ",
    "تخصصی",
    "نوپدیا",
    "نوشته",
    "شد",
  ];
  assert_analyzes_to6(&mut random, &a, input, &expected)
}
#[test]
fn test_greek() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "Γράφεται σε συνεργασία από εθελοντές με το λογισμικό wiki, κάτι που σημαίνει ότι άρθρα μπορεί να προστεθούν ή να αλλάξουν από τον καθένα.",
    &[
      "γράφεται",
      "σε",
      "συνεργασία",
      "από",
      "εθελοντές",
      "με",
      "το",
      "λογισμικό",
      "wiki",
      "κάτι",
      "που",
      "σημαίνει",
      "ότι",
      "άρθρα",
      "μπορεί",
      "να",
      "προστεθούν",
      "ή",
      "να",
      "αλλάξουν",
      "από",
      "τον",
      "καθένα",
    ],
  )?;

  Ok(())
}
#[test]
fn test_thai() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "การที่ได้ต้องแสดงว่างานดี. แล้วเธอจะไปไหน? ๑๒๓๔",
    &["การที่ได้ต้องแสดงว่างานดี", "แล้วเธอจะไปไหน", "๑๒๓๔"],
  )?;

  Ok(())
}

#[test]
fn test_lao() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "ສາທາລະນະລັດ ປະຊາທິປະໄຕ ປະຊາຊົນລາວ",
    &["ສາທາລະນະລັດ", "ປະຊາທິປະໄຕ", "ປະຊາຊົນລາວ"],
  )?;

  Ok(())
}

#[test]
fn test_tibetan() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "སྣོན་མཛོད་དང་ལས་འདིས་བོད་ཡིག་མི་ཉམས་གོང་འཕེལ་དུ་གཏོང་བར་ཧ་ཅང་དགེ་མཚན་མཆིས་སོ། །",
    &[
      "སྣོན",
      "མཛོད",
      "དང",
      "ལས",
      "འདིས",
      "བོད",
      "ཡིག",
      "མི",
      "ཉམས",
      "གོང",
      "འཕེལ",
      "དུ",
      "གཏོང",
      "བར",
      "ཧ",
      "ཅང",
      "དགེ",
      "མཚན",
      "མཆིས",
      "སོ",
    ],
  )?;

  Ok(())
}
#[test]
fn test_chinese() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "我是中国人。 １２３４ Ｔｅｓｔｓ ",
    &["我", "是", "中", "国", "人", "１２３４", "ｔｅｓｔｓ"],
  )?;

  Ok(())
}

#[test]
fn test_empty() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "", &[])?;
  assert_analyzes_to6(&mut random, &a, ".", &[])?;
  assert_analyzes_to6(&mut random, &a, " ", &[])?;

  Ok(())
}

#[test]
fn test_lucene1545() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "moͤchte", &["moͤchte"])?;

  Ok(())
}

#[test]
fn test_alphanumeric_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "B2B", &["b2b"])?;
  assert_analyzes_to6(&mut random, &a, "2B", &["2b"])?;

  Ok(())
}

#[test]
fn test_delimiters_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "some-dashed-phrase",
    &["some", "dashed", "phrase"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &a,
    "dogs,chase,cats",
    &["dogs", "chase", "cats"],
  )?;
  assert_analyzes_to6(&mut random, &a, "ac/dc", &["ac", "dc"])?;

  Ok(())
}

#[test]
fn test_apostrophes_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "O'Reilly", &["o'reilly"])?;
  assert_analyzes_to6(&mut random, &a, "you're", &["you're"])?;
  assert_analyzes_to6(&mut random, &a, "she's", &["she's"])?;
  assert_analyzes_to6(&mut random, &a, "Jim's", &["jim's"])?;
  assert_analyzes_to6(&mut random, &a, "don't", &["don't"])?;
  assert_analyzes_to6(&mut random, &a, "O'Reilly's", &["o'reilly's"])?;

  Ok(())
}
#[test]
fn test_numeric_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "21.35", &["21.35"])?;
  assert_analyzes_to6(&mut random, &a, "R2D2 C3PO", &["r2d2", "c3po"])?;
  assert_analyzes_to6(&mut random, &a, "216.239.63.104", &["216.239.63.104"])?;
  assert_analyzes_to6(&mut random, &a, "216.239.63.104", &["216.239.63.104"])?;
  Ok(())
}
#[test]
fn test_text_with_numbers_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "David has 5000 bones",
    &["david", "has", "5000", "bones"],
  )?;

  Ok(())
}
#[test]
fn test_various_text_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "C embedded developers wanted",
    &["c", "embedded", "developers", "wanted"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &a,
    "foo bar FOO BAR",
    &["foo", "bar", "foo", "bar"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &a,
    "foo      bar .  FOO <> BAR",
    &["foo", "bar", "foo", "bar"],
  )?;
  assert_analyzes_to6(&mut random, &a, "\"QUOTED\" word", &["quoted", "word"])?;

  Ok(())
}
#[test]
fn test_korean_sa() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(
    &mut random,
    &a,
    "안녕하세요 한글입니다",
    &["안녕하세요", "한글입니다"],
  )?;

  Ok(())
}

#[test]
fn test_offsets() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to9(
    &mut random,
    &a,
    "David has 5000 bones",
    &["david", "has", "5000", "bones"],
    Some(&[0, 6, 10, 15]),
    Some(&[5, 9, 14, 20]),
  )?;

  Ok(())
}

#[test]
fn test_types() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "David has 5000 bones",
    &["david", "has", "5000", "bones"],
    Some(&["<ALPHANUM>", "<ALPHANUM>", "<NUM>", "<ALPHANUM>"]),
  )?;

  Ok(())
}
#[test]
fn test_unicode_word_breaks() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}
#[test]
fn test_supplementary() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "𩬅艱鍟䇹愯瀛",
    &["𩬅", "艱", "鍟", "䇹", "愯", "瀛"],
    Some(&[
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
    ]),
  )?;

  Ok(())
}
#[test]
fn test_korean() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "훈민정음",
    &["훈민정음"],
    Some(&["<HANGUL>"]),
  )?;

  Ok(())
}

#[test]
fn test_japanese() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "仮名遣い カタカナ",
    &["仮", "名", "遣", "い", "カタカナ"],
    Some(&[
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<IDEOGRAPHIC>",
      "<HIRAGANA>",
      "<KATAKANA>",
    ]),
  )?;

  Ok(())
}
#[test]
fn test_combining_marks() -> Result<()> {
  let a = set_up();
  let mut random = random();

  check_one_term(&mut random, &a, "ざ", "ざ")?;
  check_one_term(&mut random, &a, "ザ", "ザ")?;
  check_one_term(&mut random, &a, "壹゙", "壹゙")?;
  check_one_term(&mut random, &a, "아゙", "아゙")?;

  Ok(())
}

#[test]
fn test_mid() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to6(&mut random, &a, "A:B", &["a:b"])?;
  assert_analyzes_to6(&mut random, &a, "A::B", &["a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "1.2", &["1.2"])?;
  assert_analyzes_to6(&mut random, &a, "A.B", &["a.b"])?;
  assert_analyzes_to6(&mut random, &a, "1..2", &["1", "2"])?;
  assert_analyzes_to6(&mut random, &a, "A..B", &["a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "1,2", &["1,2"])?;
  assert_analyzes_to6(&mut random, &a, "1,,2", &["1", "2"])?;

  assert_analyzes_to6(&mut random, &a, "A.:B", &["a", "b"])?;
  assert_analyzes_to6(&mut random, &a, "A:.B", &["a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "1,.2", &["1", "2"])?;
  assert_analyzes_to6(&mut random, &a, "1.,2", &["1", "2"])?;

  assert_analyzes_to6(&mut random, &a, "A:B_A:B", &["a:b_a:b"])?;
  assert_analyzes_to6(&mut random, &a, "A:B_A::B", &["a:b_a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "1.2_1.2", &["1.2_1.2"])?;
  assert_analyzes_to6(&mut random, &a, "A.B_A.B", &["a.b_a.b"])?;
  assert_analyzes_to6(&mut random, &a, "1.2_1..2", &["1.2_1", "2"])?;
  assert_analyzes_to6(&mut random, &a, "A.B_A..B", &["a.b_a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "1,2_1,2", &["1,2_1,2"])?;
  assert_analyzes_to6(&mut random, &a, "1,2_1,,2", &["1,2_1", "2"])?;

  assert_analyzes_to6(&mut random, &a, "C_A.:B", &["c_a", "b"])?;
  assert_analyzes_to6(&mut random, &a, "C_A:.B", &["c_a", "b"])?;

  assert_analyzes_to6(&mut random, &a, "3_1,.2", &["3_1", "2"])?;
  assert_analyzes_to6(&mut random, &a, "3_1.,2", &["3_1", "2"])?;

  Ok(())
}
#[test]
fn test_emoji() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "💩 💩💩",
    &["💩", "💩", "💩"],
    Some(&["<EMOJI>", "<EMOJI>", "<EMOJI>"]),
  )?;

  Ok(())
}
#[test]
fn test_emoji_sequence() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(&mut random, &a, "👩‍❤️‍👩", &["👩‍❤️‍👩"], Some(&["<EMOJI>"]))?;

  Ok(())
}

#[test]
fn test_emoji_sequence_with_modifier() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(&mut random, &a, "👨🏼‍⚕️", &["👨🏼‍⚕️"], Some(&["<EMOJI>"]))?;

  Ok(())
}

#[test]
fn test_emoji_regional_indicator() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "🇺🇸🇺🇸",
    &["🇺🇸", "🇺🇸"],
    Some(&["<EMOJI>", "<EMOJI>"]),
  )?;

  Ok(())
}
#[test]
fn test_emoji_variation_sequence() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(&mut random, &a, "#️⃣", &["#️⃣"], Some(&["<EMOJI>"]))?;
  assert_analyzes_to7(&mut random, &a, "3️⃣", &["3️⃣"], Some(&["<EMOJI>"]))?;

  assert_analyzes_to7(&mut random, &a, "#\u{FE0E}", &[], Some(&[]))?;
  assert_analyzes_to7(
    &mut random,
    &a,
    "3\u{FE0E}",
    &["3\u{FE0E}"],
    Some(&["<NUM>"]),
  )?;
  assert_analyzes_to7(
    &mut random,
    &a,
    "\u{2B55}\u{FE0E}",
    &["\u{2B55}"],
    Some(&["<EMOJI>"]),
  )?;
  assert_analyzes_to7(
    &mut random,
    &a,
    "\u{2B55}\u{FE0E}\u{200D}\u{2B55}\u{FE0E}",
    &["\u{2B55}", "\u{200D}\u{2B55}"],
    Some(&["<EMOJI>", "<EMOJI>"]),
  )?;

  Ok(())
}
#[test]
fn test_emoji_tag_sequence() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(&mut random, &a, "🏴", &["🏴"], Some(&["<EMOJI>"]))?;

  Ok(())
}

#[test]
fn test_emoji_tokenization() -> Result<()> {
  let a = set_up();
  let mut random = random();

  assert_analyzes_to7(
    &mut random,
    &a,
    "poo💩poo",
    &["poo", "💩", "poo"],
    Some(&["<ALPHANUM>", "<EMOJI>", "<ALPHANUM>"]),
  )?;
  assert_analyzes_to7(
    &mut random,
    &a,
    "💩便便💩",
    &["💩", "便", "便", "💩"],
    Some(&["<EMOJI>", "<IDEOGRAPHIC>", "<IDEOGRAPHIC>", "<EMOJI>"]),
  )?;

  Ok(())
}
#[test]
fn test_unicode_emoji_tests() -> Result<()> {
  // TODO: EmojiTokenizationTestUnicode_12_1 from Lucene's test framework has not been migrated.
  Ok(())
}

#[test]
fn test_random_strings() -> Result<()> {
  // TODO: BaseTokenStreamTestCase::check_random_data is not implemented yet.
  Ok(())
}

#[test]
fn test_random_huge_strings() -> Result<()> {
  // TODO: BaseTokenStreamTestCase::check_random_data is not implemented yet.
  Ok(())
}

#[test]
fn test_random_huge_strings_graph_after() -> Result<()> {
  // TODO: BaseTokenStreamTestCase::check_random_data and MockGraphTokenFilter are not implemented
  // for this analyzer path yet.
  Ok(())
}

#[test]
fn test_normalize() -> Result<()> {
  let analyzer = StandardAnalyzer::new();
  let normalized = analyzer.normalize("dummy", "\"\\À3[]()! Cz@")?;
  assert_eq!("\"\\à3[]()! cz@", normalized.utf8_to_string()?);
  Ok(())
}

#[test]
fn test_max_token_length_default() -> Result<()> {
  let analyzer = StandardAnalyzer::new();

  // exact max length:
  let b_string = "b".repeat(DEFAULT_MAX_TOKEN_LENGTH);
  // first bString is exact max default length; next one is 1 too long
  let input = format!("x {b_string} {b_string}b");
  let expected = ["x", b_string.as_str(), b_string.as_str(), "b"];
  let mut random = random();
  assert_analyzes_to6(&mut random, &analyzer, &input, &expected)
}

#[test]
fn test_max_token_length_non_default() -> Result<()> {
  let mut analyzer = StandardAnalyzer::new();
  analyzer.set_max_token_length(5)?;
  let mut random = random();
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "ab cd toolong xy z",
    &["ab", "cd", "toolo", "ng", "xy", "z"],
  )
}

#[test]
fn test_split_surrogate_pair_with_spoon_feed_reader() -> Result<()> {
  // TODO: The Java spoon-feed Reader has not been migrated, and Rust String cannot split a UTF-16
  // surrogate pair between Reader calls.
  Ok(())
}

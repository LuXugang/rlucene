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
use crate::core::analysis::standard::standard_analyzer::StandardAnalyzer;
use crate::test::core::analysis::base_token_stream_test_case::{
  assert_analyzes_to6, assert_analyzes_to7, assert_analyzes_to9, check_one_term,
};

use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

#[allow(dead_code)]
struct TestStandardAnalyzer;

fn set_up() -> StandardAnalyzer {
  // TODO IMPORTANT 应该支持跟多的 attribute factory
  StandardAnalyzer::new()
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
  // TODO IMPORTANT
  Ok(())
}

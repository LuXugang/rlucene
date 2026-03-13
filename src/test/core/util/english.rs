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
pub struct English;

impl English {
    pub fn long_to_english(i: i64) -> String {
        let mut result = String::new();
        Self::long_to_english_into(i, &mut result);
        result
    }

    pub fn long_to_english_into(mut i: i64, result: &mut String) {
        if i == 0 {
            result.push_str("zero");
            return;
        }
        if i < 0 {
            result.push_str("minus ");
            i = -i;
        }
        if i >= 1_000_000_000_000_000_000 {
            Self::long_to_english_into(i / 1_000_000_000_000_000_000, result);
            result.push_str("quintillion, ");
            i %= 1_000_000_000_000_000_000;
        }
        if i >= 1_000_000_000_000_000 {
            Self::long_to_english_into(i / 1_000_000_000_000_000, result);
            result.push_str("quadrillion, ");
            i %= 1_000_000_000_000_000;
        }
        if i >= 1_000_000_000_000 {
            Self::long_to_english_into(i / 1_000_000_000_000, result);
            result.push_str("trillion, ");
            i %= 1_000_000_000_000;
        }
        if i >= 1_000_000_000 {
            Self::long_to_english_into(i / 1_000_000_000, result);
            result.push_str("billion, ");
            i %= 1_000_000_000;
        }
        if i >= 1_000_000 {
            Self::long_to_english_into(i / 1_000_000, result);
            result.push_str("million, ");
            i %= 1_000_000;
        }
        if i >= 1_000 {
            Self::long_to_english_into(i / 1_000, result);
            result.push_str("thousand, ");
            i %= 1_000;
        }
        if i >= 100 {
            Self::long_to_english_into(i / 100, result);
            result.push_str("hundred ");
            i %= 100;
        }
        if i >= 20 {
            match (i as i32) / 10 {
                9 => result.push_str("ninety"),
                8 => result.push_str("eighty"),
                7 => result.push_str("seventy"),
                6 => result.push_str("sixty"),
                5 => result.push_str("fifty"),
                4 => result.push_str("forty"),
                3 => result.push_str("thirty"),
                2 => result.push_str("twenty"),
                _ => {},
            }
            i %= 10;
            if i == 0 {
                result.push(' ');
            } else {
                result.push('-');
            }
        }
        match i as i32 {
            19 => result.push_str("nineteen "),
            18 => result.push_str("eighteen "),
            17 => result.push_str("seventeen "),
            16 => result.push_str("sixteen "),
            15 => result.push_str("fifteen "),
            14 => result.push_str("fourteen "),
            13 => result.push_str("thirteen "),
            12 => result.push_str("twelve "),
            11 => result.push_str("eleven "),
            10 => result.push_str("ten "),
            9 => result.push_str("nine "),
            8 => result.push_str("eight "),
            7 => result.push_str("seven "),
            6 => result.push_str("six "),
            5 => result.push_str("five "),
            4 => result.push_str("four "),
            3 => result.push_str("three "),
            2 => result.push_str("two "),
            1 => result.push_str("one "),
            0 => {},
            _ => {},
        }
    }

    pub fn int_to_english(i: i32) -> String {
        let mut result = String::new();
        Self::long_to_english_into(i as i64, &mut result);
        result
    }

    pub fn int_to_english_into(i: i32, result: &mut String) {
        Self::long_to_english_into(i as i64, result);
    }
}

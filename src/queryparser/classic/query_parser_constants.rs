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

pub trait QueryParserConstants {}

/** End of File. */
pub const EOF: i32 = 0;
/** RegularExpression Id. */
pub const _NUM_CHAR: i32 = 1;
/** RegularExpression Id. */
pub const _ESCAPED_CHAR: i32 = 2;
/** RegularExpression Id. */
pub const _TERM_START_CHAR: i32 = 3;
/** RegularExpression Id. */
pub const _TERM_CHAR: i32 = 4;
/** RegularExpression Id. */
pub const _WHITESPACE: i32 = 5;
/** RegularExpression Id. */
pub const _QUOTED_CHAR: i32 = 6;
/** RegularExpression Id. */
pub const AND: i32 = 8;
/** RegularExpression Id. */
pub const OR: i32 = 9;
/** RegularExpression Id. */
pub const NOT: i32 = 10;
/** RegularExpression Id. */
pub const PLUS: i32 = 11;
/** RegularExpression Id. */
pub const MINUS: i32 = 12;
/** RegularExpression Id. */
pub const BAREOPER: i32 = 13;
/** RegularExpression Id. */
pub const LPAREN: i32 = 14;
/** RegularExpression Id. */
pub const RPAREN: i32 = 15;
/** RegularExpression Id. */
pub const COLON: i32 = 16;
/** RegularExpression Id. */
pub const STAR: i32 = 17;
/** RegularExpression Id. */
pub const CARAT: i32 = 18;
/** RegularExpression Id. */
pub const QUOTED: i32 = 19;
/** RegularExpression Id. */
pub const TERM: i32 = 20;
/** RegularExpression Id. */
pub const FUZZY_SLOP: i32 = 21;
/** RegularExpression Id. */
pub const PREFIXTERM: i32 = 22;
/** RegularExpression Id. */
pub const WILDTERM: i32 = 23;
/** RegularExpression Id. */
pub const REGEXPTERM: i32 = 24;
/** RegularExpression Id. */
pub const RANGEIN_START: i32 = 25;
/** RegularExpression Id. */
pub const RANGEEX_START: i32 = 26;
/** RegularExpression Id. */
pub const NUMBER: i32 = 27;
/** RegularExpression Id. */
pub const RANGE_TO: i32 = 28;
/** RegularExpression Id. */
pub const RANGEIN_END: i32 = 29;
/** RegularExpression Id. */
pub const RANGEEX_END: i32 = 30;
/** RegularExpression Id. */
pub const RANGE_QUOTED: i32 = 31;
/** RegularExpression Id. */
pub const RANGE_GOOP: i32 = 32;

/** Lexical state. */
pub const BOOST: i32 = 0;
/** Lexical state. */
pub const RANGE: i32 = 1;
/** Lexical state. */
pub const DEFAULT: i32 = 2;

/** Literal token values. */
pub const TOKEN_IMAGE: [&str; 33] = [
  "<EOF>",
  "<_NUM_CHAR>",
  "<_ESCAPED_CHAR>",
  "<_TERM_START_CHAR>",
  "<_TERM_CHAR>",
  "<_WHITESPACE>",
  "<_QUOTED_CHAR>",
  "<token of kind 7>",
  "<AND>",
  "<OR>",
  "<NOT>",
  "\"+\"",
  "\"-\"",
  "<BAREOPER>",
  "\"(\"",
  "\")\"",
  "\":\"",
  "\"*\"",
  "\"^\"",
  "<QUOTED>",
  "<TERM>",
  "<FUZZY_SLOP>",
  "<PREFIXTERM>",
  "<WILDTERM>",
  "<REGEXPTERM>",
  "\"[\"",
  "\"{\"",
  "<NUMBER>",
  "\"TO\"",
  "\"]\"",
  "\"}\"",
  "<RANGE_QUOTED>",
  "<RANGE_GOOP>",
];

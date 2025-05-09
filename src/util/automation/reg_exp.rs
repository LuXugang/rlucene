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
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::util::automation::automata::Automata;
use crate::util::automation::automaton::Automaton;
use crate::util::automation::automaton_provider::{AutomatonProvider, EmptyAutomatonProvider};
use crate::util::automation::operations::Operations;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
/// Regular Expression extension to [`Automaton`].
///
/// Regular expressions are built from the following abstract syntax:
///
/// ```text
/// regexp         ::= unionexp
///
/// unionexp       ::= interexp '|' unionexp          (union)
///                 |  interexp
///
/// interexp       ::= concatexp '&' interexp         (intersection) [OPTIONAL]
///                 |  concatexp
///
/// concatexp      ::= repeatexp concatexp            (concatenation)
///                 |  repeatexp
///
/// repeatexp      ::= repeatexp '?'                  (zero or one occurrence)
///                 |  repeatexp '*'                  (zero or more occurrences)
///                 |  repeatexp '+'                  (one or more occurrences)
///                 |  repeatexp '{n}'                (n occurrences)
///                 |  repeatexp '{n,}'               (n or more occurrences)
///                 |  repeatexp '{n,m}'              (n to m occurrences, inclusive)
///                 |  complexp
///
/// complexp       ::= charclassexp
///                 |  simpleexp
///
/// charclassexp   ::= '[' charclasses ']'            (character class)
///                 |  '[^' charclasses ']'           (negated character class)
///                 |  simpleexp
///
/// charclasses    ::= charclass charclasses
///                 |  charclass
///
/// charclass      ::= charexp '-' charexp            (character range, inclusive)
///                 |  charexp
///
/// simpleexp      ::= charexp
///                 |  '.'                            (any single character)
///                 |  '#'                            (empty language) [OPTIONAL]
///                 |  '@'                            (any string) [OPTIONAL]
///                 |  "\"" <Unicode string> "\""     (a string)
///                 |  "()"                           (the empty string)
///                 |  '(' unionexp ')'               (precedence override)
///                 |  '<' identifier '>'             (named automaton) [OPTIONAL]
///                 |  '<n-m>'                        (numerical interval) [OPTIONAL]
///
/// charexp        ::= <Unicode character>            (a single non-reserved character)
///                 |  \d                             (a digit [0-9])
///                 |  \D                             (a non-digit [^0-9])
///                 |  \s                             (whitespace [ \t\n\r])
///                 |  \S                             (non-whitespace)
///                 |  \w                             (a word character [a-zA-Z_0-9])
///                 |  \W                             (a non-word character [^\w])
///                 |  \\<Unicode character>          (an escaped character)
/// ```
///
/// Productions marked [OPTIONAL] are only allowed if specified by the syntax
/// flags passed to the [`RegExp`] constructor.
///
/// Reserved characters used in the enabled syntax must be escaped with
/// backslash (`\`) or double-quotes (`"..."`). This escaping is also required
/// inside character classes.
///
/// Be aware that dash (`-`) has a special meaning in `charclass` expressions.
///
/// An identifier is a string not containing right angle bracket (`>`) or dash
/// (`-`).
///
/// Numerical intervals are specified by non-negative decimal integers and
/// include both end points. If `n` and `m` have the same number of digits, then
/// the conforming strings must have that length (i.e., prefixed by zeroes).
pub struct RegExp {
    // ----- Immutable parsed state -----
    /// The type of expression
    pub kind: RegExpKind,
    /// Child expressions held by a container type expression
    pub exp1: Option<Box<RegExp>>,
    pub exp2: Option<Box<RegExp>>,
    /// String expression
    pub s: String,
    /// Character expression
    pub c: i32,
    /// Limits for repeatable type expressions
    pub min: i32,
    pub max: i32,
    pub digits: i32,
    pub from: i32,
    pub to: i32,
    // ----- Parser variables -----
    pub original_string: String,
    pub flags: i32,
    pub pos: usize,
}

impl RegExp {
    // ----- Syntax flags (<= 0xff) -----
    pub const INTERSECTION: i32 = 0x0001;
    pub const EMPTY: i32 = 0x0004;
    pub const ANYSTRING: i32 = 0x0008;
    pub const AUTOMATON: i32 = 0x0010;
    pub const INTERVAL: i32 = 0x0020;
    pub const ALL: i32 = 0xff;
    pub const NONE: i32 = 0x0000;
    // ----- Matching flags (> 0xff <= 0xffff) -----
    pub const ASCII_CASE_INSENSITIVE: i32 = 0x0100;
    // ----- Deprecated flags (> 0xffff) -----
    #[deprecated(note = "This flag will be removed in Lucene 11")]
    pub const DEPRECATED_COMPLEMENT: i32 = 0x10000;
    /// Equivalent to `RegExp(s)` → `RegExp::parse(s, ALL, 0)`
    pub fn from_str(s: &str) -> Result<Self> {
        Self::parse(s, Self::ALL, 0)
    }

    /// Equivalent to `RegExp(s, syntax_flags)`
    pub fn from_str_with_flags(s: &str, syntax_flags: i32) -> Result<Self> {
        Self::parse(s, syntax_flags, 0)
    }
    pub fn parse(s: &str, syntax_flags: i32, match_flags: i32) -> Result<Self> {
        if (syntax_flags & !Self::DEPRECATED_COMPLEMENT) > Self::ALL {
            return Err(LuceneError::illegal_argument("Illegal syntax flag"));
        }
        if match_flags > 0 && match_flags <= Self::ALL {
            return Err(LuceneError::illegal_argument("Illegal match flag"));
        }

        let flags = syntax_flags | match_flags;

        let mut parser = RegExp {
            kind: RegExpKind::Empty,
            exp1: None,
            exp2: None,
            s: String::new(),
            c: 0,
            min: 0,
            max: 0,
            digits: 0,
            from: 0,
            to: 0,
            original_string: s.to_string(),
            flags,
            pos: 0,
        };

        let mut e = if s.is_empty() {
            RegExp::make_string(flags, "")
        } else {
            let e = parser.parse_union_exp()?;
            if parser.pos < parser.original_string.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "end-of-string expected at position {}",
                    parser.pos
                )));
            }
            e
        };
        e.original_string = s.to_string();
        e.flags = flags;
        e.pos = parser.pos;

        Ok(e)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        flags: i32,
        kind: RegExpKind,
        exp1: Option<Box<RegExp>>,
        exp2: Option<Box<RegExp>>,
        s: &str,
        c: i32,
        min: i32,
        max: i32,
        digits: i32,
        from: i32,
        to: i32,
    ) -> Self {
        RegExp {
            original_string: String::new(),
            kind,
            flags,
            exp1,
            exp2,
            s: s.to_string(),
            c,
            min,
            max,
            digits,
            from,
            to,
            pos: 0,
        }
    }
    // Simplified construction of container nodes
    fn new_container_node(
        flags: i32,
        kind: RegExpKind,
        exp1: Option<RegExp>,
        exp2: Option<RegExp>,
    ) -> Self {
        RegExp {
            kind,
            exp1: exp1.map(Box::new),
            exp2: exp2.map(Box::new),
            s: String::new(),
            c: 0,
            min: 0,
            max: 0,
            digits: 0,
            from: 0,
            to: 0,
            original_string: String::new(),
            flags,
            pos: 0,
        }
    }

    // Simplified construction of repeating nodes
    fn new_repeating_node(flags: i32, kind: RegExpKind, exp: RegExp, min: i32, max: i32) -> Self {
        RegExp {
            kind,
            exp1: Some(Box::new(exp)),
            exp2: None,
            s: String::new(),
            c: 0,
            min,
            max,
            digits: 0,
            from: 0,
            to: 0,
            original_string: String::new(),
            flags,
            pos: 0,
        }
    }
    // Simplified construction of leaf nodes
    fn new_leaf_node(
        flags: i32,
        kind: RegExpKind,
        s: &str,
        c: i32,
        min: i32,
        max: i32,
        digits: i32,
        from: i32,
        to: i32,
    ) -> Self {
        RegExp {
            kind,
            exp1: None,
            exp2: None,
            s: s.to_string(),
            c,
            min,
            max,
            digits,
            from,
            to,
            original_string: String::new(),
            flags,
            pos: 0,
        }
    }
    /// Constructs a new [`Automaton`] from this [`RegExp`].
    /// Same as calling `to_automaton_with_map` (with an empty automaton map).
    pub fn to_automaton(&self) -> Result<Automaton> {
        self.to_automaton_impl(&HashMap::new(), &EmptyAutomatonProvider)
    }
    /// Constructs a new [`Automaton`] from this [`RegExp`].
    ///
    /// Parameters:
    /// - `automata`: A map from automaton identifiers to [`Automaton`]
    ///   instances.
    ///
    /// Errors:
    /// - Returns an error if this regular expression uses a named identifier
    ///   that does not exist in the automaton map.
    pub fn to_automaton_with_map(
        &self,
        automata: &HashMap<String, Automaton>,
    ) -> Result<Automaton> {
        self.to_automaton_impl(automata, &EmptyAutomatonProvider)
    }
    /// Constructs a new [`Automaton`] from this [`RegExp`].
    ///
    /// Parameters:
    /// - `automaton_provider`: Provider of automata for named identifiers
    ///
    /// Errors:
    /// - Returns an error if this regular expression uses a named identifier
    ///   that is not available from the automaton provider.
    pub fn to_automaton_with_provider(
        &self,
        provider: &impl AutomatonProvider,
    ) -> Result<Automaton> {
        self.to_automaton_impl(&HashMap::new(), provider)
    }
    fn to_automaton_impl(
        &self,
        automata: &HashMap<String, Automaton>,
        provider: &impl AutomatonProvider,
    ) -> Result<Automaton> {
        use RegExpKind::*;
        let a = match self.kind {
            PreClass => self
                .expand_predefined()?
                .to_automaton_impl(automata, provider)?,

            Union => {
                let mut list = Vec::new();
                if let Some(e1) = &self.exp1 {
                    e1.find_leaves(Union, &mut list, automata, provider)?;
                }
                if let Some(e2) = &self.exp2 {
                    e2.find_leaves(Union, &mut list, automata, provider)?;
                }
                let refs: Vec<&crate::util::automation::automaton::Automaton> =
                    list.iter().collect();
                Operations::union_list(&refs)?
            },

            Concatenation => {
                let mut list = Vec::new();
                if let Some(e1) = &self.exp1 {
                    e1.find_leaves(Concatenation, &mut list, automata, provider)?;
                }
                if let Some(e2) = &self.exp2 {
                    e2.find_leaves(Concatenation, &mut list, automata, provider)?;
                }
                Operations::concatenate_with_list(&list.iter().collect::<Vec<_>>())?
            },

            Intersection => {
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                let a2 = self
                    .exp2
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;

                match Operations::intersection(&a1, &a2)? {
                    Cow::Borrowed(v) => {
                        if std::ptr::eq(v, &a1) {
                            a1
                        } else {
                            a2
                        }
                    },
                    Cow::Owned(o) => o,
                }
            },

            Optional => {
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                match Operations::optional(&a1)? {
                    Cow::Borrowed(_) => a1,
                    Cow::Owned(o) => o,
                }
            },

            Repeat => {
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                match Operations::repeat(&a1)? {
                    Cow::Borrowed(_) => a1,
                    Cow::Owned(o) => o,
                }
            },

            RepeatMin => {
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                match Operations::repeat_count(&a1, self.min)? {
                    Cow::Borrowed(_) => a1,
                    Cow::Owned(o) => o,
                }
            },

            RepeatMinMax => {
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                Operations::repeat_min_max(&a1, self.min, self.max)?
            },

            Complement => {
                // we don't support arbitrary complement, just "negated character class"
                // this is just a list of characters (e.g. "a") or ranges (e.g. "b-d")
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                Operations::complement(&a1, i32::MAX as usize)?
            },

            DeprecatedComplement => {
                // to ease transitions for users only, support arbitrary complement
                // but bounded by DEFAULT_DETERMINIZE_WORK_LIMIT: must not be configurable.
                let a1 = self
                    .exp1
                    .as_ref()
                    .unwrap()
                    .to_automaton_impl(automata, provider)?;
                Operations::complement(&a1, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
            },

            Char => {
                if self.check(Self::ASCII_CASE_INSENSITIVE) {
                    Self::to_case_insensitive_char(self.c)?
                } else {
                    Automata::make_char(self.c)?
                }
            },

            CharRange => Automata::make_char_range(self.from, self.to)?,
            AnyChar => Automata::make_any_char()?,
            Empty => Automata::make_empty()?,
            String => {
                if self.check(Self::ASCII_CASE_INSENSITIVE) {
                    self.to_case_insensitive_string()?
                } else {
                    Automata::make_string(&self.s)?
                }
            },
            AnyString => Automata::make_any_string()?,

            Automaton => {
                if let Some(a) = automata.get(&self.s) {
                    // TODO: Data Copy here, but currently only used in Test,
                    a.clone()
                } else {
                    provider.get_automaton(&self.s)?
                }
            },

            Interval => Automata::make_decimal_interval(self.min, self.max, self.digits)?,
        };

        Ok(a)
    }
    fn to_case_insensitive_char(codepoint: i32) -> Result<Automaton> {
        let case1 = Automata::make_char(codepoint)?;

        if codepoint > 128 {
            return Ok(case1);
        }

        let alt_case = if (codepoint as u8 as char).is_ascii_lowercase() {
            (codepoint as u8 as char).to_ascii_uppercase() as i32
        } else {
            (codepoint as u8 as char).to_ascii_lowercase() as i32
        };

        if alt_case != codepoint {
            let case2 = Automata::make_char(alt_case)?;
            Operations::union(&case1, &case2)
        } else {
            Ok(case1)
        }
    }
    fn to_case_insensitive_string(&self) -> Result<Automaton> {
        let list: Result<Vec<Automaton>> = self
            .s
            .chars()
            .map(|ch| Self::to_case_insensitive_char(ch as i32))
            .collect();

        let automata = list?;
        let refs: Vec<&Automaton> = automata.iter().collect();
        Operations::concatenate_with_list(&refs)
    }

    fn find_leaves(
        &self,
        kind: RegExpKind,
        list: &mut Vec<Automaton>,
        automata: &HashMap<String, Automaton>,
        provider: &impl AutomatonProvider,
    ) -> Result<()> {
        if self.kind == kind {
            if let Some(e1) = &self.exp1 {
                e1.find_leaves(kind, list, automata, provider)?;
            }
            if let Some(e2) = &self.exp2 {
                e2.find_leaves(kind, list, automata, provider)?;
            }
        } else {
            list.push(self.to_automaton_impl(automata, provider)?)
        }
        Ok(())
    }
    /// The string that was used to construct the regex. Compare to toString.
    pub fn get_original_string(&self) -> &str {
        &self.original_string
    }
    pub fn to_string_builder(&self, b: &mut String) {
        use RegExpKind::*;

        match self.kind {
            Union => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push('|');
                self.exp2.as_ref().unwrap().to_string_builder(b);
                b.push(')');
            },
            Concatenation => {
                self.exp1.as_ref().unwrap().to_string_builder(b);
                self.exp2.as_ref().unwrap().to_string_builder(b);
            },
            Intersection => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push('&');
                self.exp2.as_ref().unwrap().to_string_builder(b);
                b.push(')');
            },
            Optional => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push_str(")?");
            },
            Repeat => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push_str(")*");
            },
            RepeatMin => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push_str("){");
                b.push_str(&self.min.to_string());
                b.push_str(",}");
            },
            RepeatMinMax => {
                b.push('(');
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push_str("){");
                b.push_str(&self.min.to_string());
                b.push(',');
                b.push_str(&self.max.to_string());
                b.push('}');
            },
            Complement | DeprecatedComplement => {
                b.push_str("~(");
                self.exp1.as_ref().unwrap().to_string_builder(b);
                b.push(')');
            },
            Char => {
                if let Some(ch) = std::char::from_u32(self.c as u32) {
                    b.push('\\');
                    b.push(ch);
                }
            },
            CharRange => {
                let from_ch = std::char::from_u32(self.from as u32).unwrap_or('?');
                let to_ch = std::char::from_u32(self.to as u32).unwrap_or('?');
                b.push_str("[\\");
                b.push(from_ch);
                b.push('-');
                b.push('\\');
                b.push(to_ch);
                b.push(']');
            },
            AnyChar => {
                b.push('.');
            },
            Empty => {
                b.push('#');
            },
            String => {
                b.push('"');
                b.push_str(&self.s);
                b.push('"');
            },
            AnyString => {
                b.push('@');
            },
            Automaton => {
                b.push('<');
                b.push_str(&self.s);
                b.push('>');
            },
            Interval => {
                let s1 = self.min.to_string();
                let s2 = self.max.to_string();
                b.push('<');
                if self.digits > 0 {
                    for _ in s1.len()..self.digits as usize {
                        b.push('0');
                    }
                }
                b.push_str(&s1);
                b.push('-');
                if self.digits > 0 {
                    for _ in s2.len()..self.digits as usize {
                        b.push('0');
                    }
                }
                b.push_str(&s2);
                b.push('>');
            },
            PreClass => {
                if let Some(ch) = std::char::from_u32(self.from as u32) {
                    b.push('\\');
                    b.push(ch);
                }
            },
        }
    }
    /// Like to string, but more verbose (shows the higherchy more clearly).
    pub fn to_string_tree(&self) -> String {
        let mut b = String::new();
        self.to_string_tree_with_string(&mut b, "");
        b
    }
    pub(crate) fn to_string_tree_with_string(&self, b: &mut String, indent: &str) {
        use RegExpKind::*;

        let newline = "\n";
        let indent_more = format!("{}  ", indent);

        match self.kind {
            // binary
            Union | Concatenation | Intersection => {
                b.push_str(indent);
                b.push_str(&format!("{:?}{}", self.kind, newline));
                if let Some(e1) = &self.exp1 {
                    e1.to_string_tree_with_string(b, &indent_more);
                }
                if let Some(e2) = &self.exp2 {
                    e2.to_string_tree_with_string(b, &indent_more);
                }
            },

            // unary
            Optional | Repeat | Complement | DeprecatedComplement => {
                b.push_str(indent);
                b.push_str(&format!("{:?}{}", self.kind, newline));
                if let Some(e1) = &self.exp1 {
                    e1.to_string_tree_with_string(b, &indent_more);
                }
            },

            RepeatMin => {
                b.push_str(indent);
                b.push_str(&format!("{:?} min={}{}", self.kind, self.min, newline));
                if let Some(e1) = &self.exp1 {
                    e1.to_string_tree_with_string(b, &indent_more);
                }
            },

            RepeatMinMax => {
                b.push_str(indent);
                b.push_str(&format!(
                    "{:?} min={} max={}{}",
                    self.kind, self.min, self.max, newline
                ));
                if let Some(e1) = &self.exp1 {
                    e1.to_string_tree_with_string(b, &indent_more);
                }
            },

            Char => {
                b.push_str(indent);
                if let Some(ch) = std::char::from_u32(self.c as u32) {
                    b.push_str(&format!("{:?} char={}{}", self.kind, ch, newline));
                } else {
                    b.push_str(&format!("{:?} char=?{}", self.kind, newline));
                }
            },

            PreClass => {
                b.push_str(indent);
                if let Some(ch) = std::char::from_u32(self.from as u32) {
                    b.push_str(&format!("{:?} class=\\{}{}", self.kind, ch, newline));
                } else {
                    b.push_str(&format!("{:?} class=\\?{}", self.kind, newline));
                }
            },

            CharRange => {
                b.push_str(indent);
                let from_ch = std::char::from_u32(self.from as u32).unwrap_or('?');
                let to_ch = std::char::from_u32(self.to as u32).unwrap_or('?');
                b.push_str(&format!(
                    "{:?} from={} to={}{}",
                    self.kind, from_ch, to_ch, newline
                ));
            },

            String => {
                b.push_str(indent);
                b.push_str(&format!("{:?} string={}{}", self.kind, self.s, newline));
            },

            Interval => {
                b.push_str(indent);
                b.push_str(&format!("{:?} <", self.kind));
                let s1 = self.min.to_string();
                let s2 = self.max.to_string();
                if self.digits > 0 {
                    for _ in s1.len()..self.digits as usize {
                        b.push('0');
                    }
                }
                b.push_str(&s1);
                b.push('-');
                if self.digits > 0 {
                    for _ in s2.len()..self.digits as usize {
                        b.push('0');
                    }
                }
                b.push_str(&s2);
                b.push_str(&format!(">{}", newline));
            },

            AnyChar | AnyString | Empty | Automaton => {
                b.push_str(indent);
                b.push_str(&format!("{:?}{}", self.kind, newline));
            },
        }
    }
    /// Returns set of automaton identifiers that occur in this regular
    /// expression.
    pub fn get_identifiers_set(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        self.get_identifiers(&mut set);
        set
    }
    pub(crate) fn get_identifiers(&self, set: &mut HashSet<String>) {
        use RegExpKind::*;
        match self.kind {
            Union | Concatenation | Intersection => {
                if let Some(ref e1) = self.exp1 {
                    e1.get_identifiers(set);
                }
                if let Some(ref e2) = self.exp2 {
                    e2.get_identifiers(set);
                }
            },
            Optional | Repeat | RepeatMin | RepeatMinMax | Complement | DeprecatedComplement => {
                if let Some(ref e1) = self.exp1 {
                    e1.get_identifiers(set);
                }
            },
            Automaton => {
                set.insert(self.s.clone());
            },
            AnyChar | AnyString | Char | CharRange | Empty | Interval | PreClass | String => {
                // No-op
            },
        }
    }
    fn make_union(flags: i32, exp1: RegExp, exp2: RegExp) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Union, Some(exp1), Some(exp2))
    }
    fn make_concatenation(flags: i32, mut exp1: RegExp, mut exp2: RegExp) -> Self {
        let is_str_or_char = |e: &RegExp| matches!(e.kind, RegExpKind::Char | RegExpKind::String);
        if is_str_or_char(&exp1) && is_str_or_char(&exp2) {
            return RegExp::make_string_concat(flags, &exp1, &exp2);
        }

        if exp1.kind == RegExpKind::Concatenation {
            if let Some(e2) = &exp1.exp2 {
                if is_str_or_char(e2) && is_str_or_char(&exp2) {
                    let rexp1 = *exp1.exp1.take().unwrap();
                    let rexp2 = RegExp::make_string_concat(flags, e2, &exp2);
                    return RegExp::new_container_node(
                        flags,
                        RegExpKind::Concatenation,
                        Some(rexp1),
                        Some(rexp2),
                    );
                }
            }
        } else if exp2.kind == RegExpKind::Concatenation {
            if let Some(e1) = &exp2.exp1 {
                if is_str_or_char(&exp1) && is_str_or_char(e1) {
                    let rexp1 = RegExp::make_string_concat(flags, &exp1, e1);
                    let rexp2 = *exp2.exp2.take().unwrap();
                    return RegExp::new_container_node(
                        flags,
                        RegExpKind::Concatenation,
                        Some(rexp1),
                        Some(rexp2),
                    );
                }
            }
        }

        RegExp::new_container_node(flags, RegExpKind::Concatenation, Some(exp1), Some(exp2))
    }
    fn make_string_concat(flags: i32, exp1: &RegExp, exp2: &RegExp) -> Self {
        let mut b = String::new();

        match exp1.kind {
            RegExpKind::String => b.push_str(&exp1.s),
            RegExpKind::Char => {
                if let Some(ch) = std::char::from_u32(exp1.c as u32) {
                    b.push(ch);
                }
            },
            _ => {},
        }

        match exp2.kind {
            RegExpKind::String => b.push_str(&exp2.s),
            RegExpKind::Char => {
                if let Some(ch) = std::char::from_u32(exp2.c as u32) {
                    b.push(ch);
                }
            },
            _ => {},
        }
        RegExp::make_string(flags, &b)
    }
    fn make_intersection(flags: i32, exp1: RegExp, exp2: RegExp) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Intersection, Some(exp1), Some(exp2))
    }

    fn make_optional(flags: i32, exp: RegExp) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Optional, Some(exp), None)
    }

    fn make_repeat(flags: i32, exp: RegExp) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Repeat, Some(exp), None)
    }

    fn make_repeat_min(flags: i32, exp: RegExp, min: i32) -> Self {
        RegExp::new_repeating_node(flags, RegExpKind::RepeatMin, exp, min, 0)
    }

    fn make_repeat_minmax(flags: i32, exp: RegExp, min: i32, max: i32) -> Self {
        RegExp::new_repeating_node(flags, RegExpKind::RepeatMinMax, exp, min, max)
    }

    fn make_complement(flags: i32, exp: RegExp) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Complement, Some(exp), None)
    }
    /// Creates a node that will compute the complement of an arbitrary
    /// expression.
    ///
    /// @deprecated Will be removed in Lucene 11
    #[deprecated(note = "Will be removed in Lucene 11")]
    fn make_deprecated_complement(flags: i32, exp: RegExp) -> RegExp {
        RegExp::new_container_node(flags, RegExpKind::DeprecatedComplement, Some(exp), None)
    }

    fn make_char(flags: i32, c: i32) -> Self {
        RegExp::new_leaf_node(flags, RegExpKind::Char, "", c, 0, 0, 0, 0, 0)
    }

    fn make_char_range(flags: i32, from: i32, to: i32) -> Result<Self> {
        if from > to {
            return Err(LuceneError::illegal_argument(format!(
                "invalid range: from ({}) cannot be > to ({})",
                from, to
            )));
        }
        Ok(RegExp::new_leaf_node(
            flags,
            RegExpKind::CharRange,
            "",
            0,
            0,
            0,
            0,
            from,
            to,
        ))
    }

    fn make_any_char(flags: i32) -> Self {
        RegExp::new_container_node(flags, RegExpKind::AnyChar, None, None)
    }

    fn make_empty(flags: i32) -> Self {
        RegExp::new_container_node(flags, RegExpKind::Empty, None, None)
    }

    fn make_string(flags: i32, s: &str) -> Self {
        RegExp::new_leaf_node(flags, RegExpKind::String, s, 0, 0, 0, 0, 0, 0)
    }

    fn make_any_string(flags: i32) -> Self {
        RegExp::new_container_node(flags, RegExpKind::AnyString, None, None)
    }
    fn make_automaton(flags: i32, s: &str) -> Self {
        RegExp::new_leaf_node(flags, RegExpKind::Automaton, s, 0, 0, 0, 0, 0, 0)
    }

    fn make_interval(flags: i32, min: i32, max: i32, digits: i32) -> Self {
        RegExp::new_leaf_node(flags, RegExpKind::Interval, "", 0, min, max, digits, 0, 0)
    }

    fn peek(&self, s: &str) -> bool {
        self.more() && s.contains(self.original_string[self.pos..].chars().next().unwrap())
    }

    fn match_char(&mut self, c: char) -> bool {
        if let Some(next_ch) = self.original_string[self.pos..].chars().next() {
            if next_ch == c {
                self.pos += next_ch.len_utf8();
                return true;
            }
        }
        false
    }
    fn more(&self) -> bool {
        self.pos < self.original_string.len()
    }
    fn next(&mut self) -> Result<i32> {
        if !self.more() {
            return Err(LuceneError::illegal_argument("unexpected end-of-string"));
        }
        let ch = self.original_string[self.pos..].chars().next().unwrap();
        self.pos += ch.len_utf8();
        Ok(ch as i32)
    }

    fn check(&self, flag: i32) -> bool {
        (self.flags & flag) != 0
    }
    pub(crate) fn parse_union_exp(&mut self) -> Result<RegExp> {
        let flags = self.flags;
        self.iterative_parse_exp(
            |p| p.parse_inter_exp(),
            |p| p.match_char('|'),
            &UnionGroup,
            flags,
        )
    }

    pub(crate) fn parse_inter_exp(&mut self) -> Result<RegExp> {
        let flags = self.flags;
        self.iterative_parse_exp(
            |p| p.parse_concat_exp(),
            |p| p.check(RegExp::INTERSECTION) && p.match_char('&'),
            &IntersectionGroup,
            flags,
        )
    }

    pub(crate) fn parse_concat_exp(&mut self) -> Result<RegExp> {
        let flags = self.flags;
        self.iterative_parse_exp(
            |p| p.parse_repeat_exp(),
            |p| p.more() && !p.peek(")|") && (!p.check(RegExp::INTERSECTION) || !p.peek("&")),
            &ConcatGroup,
            flags,
        )
    }
    fn iterative_parse_exp<G, S, R>(
        &mut self,
        mut gather: G,
        mut stop: S,
        reducer: &R,
        flags: i32,
    ) -> Result<RegExp>
    where
        G: FnMut(&mut Self) -> Result<RegExp>,
        S: FnMut(&mut Self) -> bool,
        R: MakeRegexGroup,
    {
        let mut result = gather(self)?;
        while stop(self) {
            let e = gather(self)?;
            result = reducer.get(flags, result, e);
        }
        Ok(result)
    }
    fn parse_repeat_exp(&mut self) -> Result<RegExp> {
        let mut e = self.parse_compl_exp()?;

        while self.peek("?*+{") {
            if self.match_char('?') {
                e = RegExp::make_optional(self.flags, e);
            } else if self.match_char('*') {
                e = RegExp::make_repeat(self.flags, e);
            } else if self.match_char('+') {
                e = RegExp::make_repeat_min(self.flags, e, 1);
            } else if self.match_char('{') {
                let start = self.pos;
                while self.peek("0123456789") {
                    self.next()?;
                }
                if start == self.pos {
                    return Err(LuceneError::illegal_argument(format!(
                        "integer expected at position {}",
                        self.pos
                    )));
                }
                let n_str = &self.original_string[start..self.pos];
                let n = n_str.parse::<i32>().map_err(|_| {
                    LuceneError::illegal_argument(format!(
                        "invalid number at position {}",
                        self.pos
                    ))
                })?;

                let mut m = -1;
                if self.match_char(',') {
                    let start = self.pos;
                    while self.peek("0123456789") {
                        self.next()?;
                    }
                    if start != self.pos {
                        let m_str = &self.original_string[start..self.pos];
                        m = m_str.parse::<i32>().map_err(|_| {
                            LuceneError::illegal_argument(format!(
                                "invalid number at position {}",
                                self.pos
                            ))
                        })?;
                    }
                } else {
                    m = n;
                }

                if !self.match_char('}') {
                    return Err(LuceneError::illegal_argument(format!(
                        "expected '}}' at position {}",
                        self.pos
                    )));
                }

                if m != -1 && n > m {
                    return Err(LuceneError::illegal_argument(format!(
                        "invalid repetition range (out of order): {}..{}",
                        n, m
                    )));
                }

                if m == -1 {
                    e = RegExp::make_repeat_min(self.flags, e, n);
                } else {
                    e = RegExp::make_repeat_minmax(self.flags, e, n, m);
                }
            }
        }

        Ok(e)
    }
    pub(crate) fn parse_compl_exp(&mut self) -> Result<RegExp> {
        if self.check(RegExp::DEPRECATED_COMPLEMENT) && self.match_char('~') {
            let sub = self.parse_compl_exp()?;
            Ok(RegExp::make_deprecated_complement(self.flags, sub))
        } else {
            self.parse_char_class_exp()
        }
    }
    pub(crate) fn parse_char_class_exp(&mut self) -> Result<RegExp> {
        if self.match_char('[') {
            let mut negate = false;
            if self.match_char('^') {
                negate = true;
            }
            let mut e = self.parse_char_classes()?;
            if negate {
                let any = RegExp::make_any_char(self.flags);
                let not_e = RegExp::make_complement(self.flags, e);
                e = RegExp::make_intersection(self.flags, any, not_e);
            }
            if !self.match_char(']') {
                return Err(LuceneError::illegal_argument(format!(
                    "expected ']' at position {}",
                    self.pos
                )));
            }
            Ok(e)
        } else {
            self.parse_simple_exp()
        }
    }
    pub(crate) fn parse_char_classes(&mut self) -> Result<RegExp> {
        let mut e = self.parse_char_class()?;
        while self.more() && !self.peek("]") {
            let next = self.parse_char_class()?;
            e = RegExp::make_union(self.flags, e, next);
        }
        Ok(e)
    }
    pub(crate) fn parse_char_class(&mut self) -> Result<RegExp> {
        if let Some(predefined) = self.match_predefined_character_class()? {
            return Ok(predefined);
        }

        let c1 = self.parse_char_exp()?;
        if self.match_char('-') {
            return RegExp::make_char_range(self.flags, c1, self.parse_char_exp()?);
        }

        Ok(RegExp::make_char(self.flags, c1))
    }
    fn expand_predefined(&self) -> Result<RegExp> {
        match std::char::from_u32(self.from as u32) {
            Some('d') => RegExp::from_str("[0-9]"),        // digit
            Some('D') => RegExp::from_str("[^0-9]"),       // non-digit
            Some('s') => RegExp::from_str("[ \t\n\r]"),    // whitespace
            Some('S') => RegExp::from_str("[^\\s]"),       // non-whitespace
            Some('w') => RegExp::from_str("[a-zA-Z_0-9]"), // word
            Some('W') => RegExp::from_str("[^\\w]"),       // non-word
            Some(ch) => Err(LuceneError::illegal_argument(format!(
                "invalid character class: \\{}",
                ch
            ))),
            None => Err(LuceneError::illegal_argument(
                "invalid unicode value in .from",
            )),
        }
    }
    pub(crate) fn match_predefined_character_class(&mut self) -> Result<Option<RegExp>> {
        // See https://docs.oracle.com/javase/tutorial/essential/regex/pre_char_classes.html
        if self.match_char('\\') {
            if self.peek("dDwWsS") {
                let cp = self.next()?;
                return Ok(Some(RegExp::new_leaf_node(
                    self.flags,
                    RegExpKind::PreClass,
                    "",
                    0,
                    0,
                    0,
                    0,
                    cp,
                    0,
                )));
            }

            if self.peek("\\") {
                let cp = self.next()?;
                return Ok(Some(RegExp::make_char(self.flags, cp)));
            }
            // From https://docs.oracle.com/javase/8/docs/api/java/util/regex/Pattern.html#bs
            // "It is an error to use a backslash prior to any alphabetic character that
            // does not denote an escaped
            // construct;"
            if self.peek("abcefghijklmnopqrtuvxyz") || self.peek("ABCEFGHIJKLMNOPQRTUVXYZ") {
                let cp = self.next()?;
                let ch = std::char::from_u32(cp as u32).unwrap_or('?');
                return Err(LuceneError::illegal_argument(format!(
                    "invalid character class \\{}",
                    ch
                )));
            }
        }

        Ok(None)
    }
    pub(crate) fn parse_simple_exp(&mut self) -> Result<RegExp> {
        if self.match_char('.') {
            Ok(RegExp::make_any_char(self.flags))
        } else if self.check(RegExp::EMPTY) && self.match_char('#') {
            return Ok(RegExp::make_empty(self.flags));
        } else if self.check(RegExp::ANYSTRING) && self.match_char('@') {
            return Ok(RegExp::make_any_string(self.flags));
        } else if self.match_char('"') {
            let start = self.pos;
            while self.more() && !self.peek("\"") {
                self.next()?;
            }
            if !self.match_char('"') {
                return Err(LuceneError::illegal_argument(format!(
                    "expected '\"' at position {}",
                    self.pos
                )));
            }
            let s = self.original_string[start..(self.pos - 1)].to_string();
            return Ok(RegExp::make_string(self.flags, &s));
        } else if self.match_char('(') {
            if self.match_char(')') {
                return Ok(RegExp::make_string(self.flags, ""));
            }
            let e = self.parse_union_exp()?;
            if !self.match_char(')') {
                return Err(LuceneError::illegal_argument(format!(
                    "expected ')' at position {}",
                    self.pos
                )));
            }
            return Ok(e);
        } else if (self.check(RegExp::AUTOMATON) || self.check(RegExp::INTERVAL))
            && self.match_char('<')
        {
            let start = self.pos;
            while self.more() && !self.peek(">") {
                self.next()?;
            }
            if !self.match_char('>') {
                return Err(LuceneError::illegal_argument(format!(
                    "expected '>' at position {}",
                    self.pos
                )));
            }
            let s = self.original_string[start..(self.pos - 1)].to_string();
            if let Some(i) = s.find('-') {
                if !self.check(RegExp::INTERVAL) {
                    return Err(LuceneError::illegal_argument(format!(
                        "illegal identifier at position {}",
                        self.pos - 1
                    )));
                }
                if i == 0 || i == s.len() - 1 || i != s.rfind('-').unwrap() {
                    return Err(LuceneError::illegal_argument(format!(
                        "interval syntax error at position {}",
                        self.pos - 1
                    )));
                }
                // parse interval
                let smin = &s[0..i];
                let smax = &s[i + 1..];
                let imin = smin.parse::<i32>().map_err(|_| {
                    LuceneError::illegal_argument(format!(
                        "interval syntax error at position {}",
                        self.pos - 1
                    ))
                })?;
                let imax = smax.parse::<i32>().map_err(|_| {
                    LuceneError::illegal_argument(format!(
                        "interval syntax error at position {}",
                        self.pos - 1
                    ))
                })?;
                let digits = if smin.len() == smax.len() {
                    smin.len() as i32
                } else {
                    0
                };
                let (min, max) = if imin <= imax {
                    (imin, imax)
                } else {
                    (imax, imin)
                };
                return Ok(RegExp::make_interval(self.flags, min, max, digits));
            } else {
                if !self.check(RegExp::AUTOMATON) {
                    return Err(LuceneError::illegal_argument(format!(
                        "interval syntax error at position {}",
                        self.pos - 1
                    )));
                }
                return Ok(RegExp::make_automaton(self.flags, &s));
            }
        } else {
            if let Some(predefined) = self.match_predefined_character_class()? {
                return Ok(predefined);
            }
            let ch = self.parse_char_exp()?;
            return Ok(RegExp::make_char(self.flags, ch));
        }
    }
    fn parse_char_exp(&mut self) -> Result<i32> {
        self.match_char('\\');
        self.next()
    }
}
impl fmt::Display for RegExp {
    /// Constructs string from parsed regular expression.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        self.to_string_builder(&mut s);
        write!(f, "{}", s)
    }
}
/// The type of expression represented by a RegExp node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegExpKind {
    /// The union of two expressions
    Union,
    /// A sequence of two expressions
    Concatenation,
    /// The intersection of two expressions
    Intersection,
    /// An optional expression
    Optional,
    /// An expression that repeats
    Repeat,
    /// An expression that repeats a minimum number of times
    RepeatMin,
    /// An expression that repeats a minimum and maximum number of times
    RepeatMinMax,
    /// The complement of a character class
    Complement,
    /// A Character
    Char,
    /// A Character range
    CharRange,
    /// Any Character allowed
    AnyChar,
    /// An empty expression
    Empty,
    /// A string expression
    String,
    /// Any string allowed
    AnyString,
    /// An Automaton expression
    Automaton,
    /// An Interval expression
    Interval,
    /// An expression for a pre-defined class e.g. \w
    PreClass,
    /// The complement of an expression (deprecated)
    #[deprecated(note = "Will be removed in Lucene 11")]
    DeprecatedComplement,
}
/// Custom functional interface for supplying methods with the signature:
/// `RegExp(int int1, RegExp exp1, RegExp exp2)`
trait MakeRegexGroup {
    fn get(&self, int1: i32, exp1: RegExp, exp2: RegExp) -> RegExp;
}
struct UnionGroup;
impl MakeRegexGroup for UnionGroup {
    fn get(&self, flags: i32, e1: RegExp, e2: RegExp) -> RegExp {
        RegExp::make_union(flags, e1, e2)
    }
}

struct IntersectionGroup;
impl MakeRegexGroup for IntersectionGroup {
    fn get(&self, flags: i32, e1: RegExp, e2: RegExp) -> RegExp {
        RegExp::make_intersection(flags, e1, e2)
    }
}

struct ConcatGroup;
impl MakeRegexGroup for ConcatGroup {
    fn get(&self, flags: i32, e1: RegExp, e2: RegExp) -> RegExp {
        RegExp::make_concatenation(flags, e1, e2)
    }
}

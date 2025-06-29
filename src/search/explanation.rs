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
use std::fmt;
use std::hash::Hash;

use crate::util::number::Number;
/// Expert: Describes the score computation for document and query.
pub struct Explanation {
    pub matched: bool,
    pub value: Number,
    pub description: String,
    pub details: Vec<Explanation>,
}
impl Explanation {
    /// Internal constructor, equivalent to private constructor in Java
    fn new(matched: bool, value: Number, description: String, details: Vec<Explanation>) -> Self {
        Explanation {
            matched,
            value,
            description,
            details,
        }
    }
    /// Indicates whether or not this Explanation models a match.
    pub fn is_match(&self) -> bool {
        self.matched
    }
    /// The value assigned to this explanation node.
    pub fn get_value(&self) -> &Number {
        &self.value
    }
    /// A description of this explanation node.
    pub fn get_description(&self) -> &str {
        &self.description
    }

    fn get_summary(&self) -> String {
        format!("{} = {}", self.get_value(), self.get_description())
    }
    /// The sub-nodes of this explanation node.
    pub fn get_details(&self) -> &[Explanation] {
        &self.details
    }
    /// Render an explanation as text.
    fn to_string_with_depth(&self, depth: usize) -> String {
        let mut buffer = String::new();
        for _ in 0..depth {
            buffer.push_str("  ");
        }
        buffer.push_str(&self.get_summary());
        buffer.push('\n');

        for detail in &self.details {
            buffer.push_str(&detail.to_string_with_depth(depth + 1));
        }

        buffer
    }
}
impl PartialEq for Explanation {
    fn eq(&self, other: &Self) -> bool {
        self.matched == other.matched
            && self.value == other.value
            && self.description == other.description
            && self.details == other.details
    }
}
impl Hash for Explanation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.matched.hash(state);
        self.value.hash(state);
        self.description.hash(state);
        self.details.hash(state);
    }
}

impl Eq for Explanation {}
impl fmt::Display for Explanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_with_depth(0))
    }
}

pub mod explanation_util {
    use crate::search::explanation::Explanation;
    use crate::util::number::Number;
    /// Create a new explanation for a match.
    ///
    /// # Arguments
    ///
    /// * `value` - The contribution to the score of the document.
    /// * `description` - How `value` was computed.
    /// * `details` - Sub explanations that contributed to this explanation.
    pub fn match_(value: Number, description: String, details: Vec<Explanation>) -> Explanation {
        Explanation::new(true, value, description, details)
    }
    /// Create a new explanation for a document which does not match.
    pub fn no_match(description: String, details: Vec<Explanation>) -> Explanation {
        Explanation::new(false, Number::F32(0.0), description, details)
    }
}

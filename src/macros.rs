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
/// Creates a vector by converting each element with [`Into::into`].
///
/// The target element type must be known from a type annotation or the receiving API.
/// For Lucene fields, queries and sort fields, prefer [`fields!`], [`queries!`] and
/// [`sort_fields!`], which also infer the target type for empty lists.
///
/// ```
/// let values: Vec<String> = rlucene::into_vec!["first", String::from("second")];
/// assert_eq!(values, ["first", "second"]);
/// ```
#[macro_export]
macro_rules! into_vec {
  ($($value:expr),* $(,)?) => {
    ::std::vec![$(::std::convert::Into::into($value)),*]
  };
}

/// Creates a `Vec<Fields>` from different concrete field types without explicit conversions.
///
/// Each expression is evaluated once, in order, and moved into the vector. The empty
/// invocation `fields![]` also has the concrete type `Vec<Fields>`.
///
/// ```
/// use rlucene::core::document::{field::Store, int_field::IntField, text_field::TextField};
/// # use rlucene::core::util::error::lucene_error::Result;
/// # fn main() -> Result<()> {
/// let fields = rlucene::fields![
///     IntField::new("age", 18, Store::Yes)?,
///     TextField::from_string("body", "Rust Lucene", Store::No)?,
/// ];
/// # assert_eq!(fields.len(), 2);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! fields {
  ($($field:expr),* $(,)?) => {{
    let fields: ::std::vec::Vec<$crate::core::document::fields::Fields> = ::std::vec![
      $(::std::convert::Into::<$crate::core::document::fields::Fields>::into($field)),*
    ];
    fields
  }};
}

/// Creates a `Vec<Query>` from different query types using Lucene's [`IntoQuery`](crate::core::search::query::IntoQuery).
///
/// Each expression is evaluated once, in order. The empty invocation `queries![]`
/// also has the concrete type `Vec<Query>`.
///
/// ```
/// use rlucene::core::index::term::Term;
/// use rlucene::core::search::{match_all_docs_query::MatchAllDocsQuery, term_query::TermQuery};
/// let queries = rlucene::queries![
///     TermQuery::new(Term::from_text("id", "42")),
///     MatchAllDocsQuery::new(),
/// ];
/// assert_eq!(queries.len(), 2);
/// ```
#[macro_export]
macro_rules! queries {
  ($($query:expr),* $(,)?) => {{
    let queries: ::std::vec::Vec<$crate::core::search::query::Query> = ::std::vec![
      $($crate::core::search::query::IntoQuery::into_query($query)),*
    ];
    queries
  }};
}

/// Creates a `Vec<SortFieldEnum>` from different concrete sort field types.
///
/// Each expression is evaluated once, in order. The empty invocation `sort_fields![]`
/// has the concrete type `Vec<SortFieldEnum>`; [`Sort::with_fields`](crate::core::search::sort::Sort::with_fields)
/// still rejects an empty collection.
///
/// ```
/// use rlucene::core::search::sort::Sort;
/// use rlucene::core::search::sort_field::{SortField, SortFieldType};
/// use rlucene::core::search::sorted_numeric_sort_field::SortedNumericSortField;
/// # use rlucene::core::util::error::lucene_error::Result;
/// # fn main() -> Result<()> {
/// let sort = Sort::with_fields(rlucene::sort_fields![
///     SortedNumericSortField::new("age", SortFieldType::Int)?,
///     SortField::get_field_doc()?,
/// ])?;
/// # assert_eq!(sort.get_sort().len(), 2);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! sort_fields {
  ($($field:expr),* $(,)?) => {{
    let fields: ::std::vec::Vec<$crate::core::search::sort_field_enum::SortFieldEnum> = ::std::vec![
      $(::std::convert::Into::<$crate::core::search::sort_field_enum::SortFieldEnum>::into($field)),*
    ];
    fields
  }};
}

#[macro_export]
macro_rules! dummy_unreachable {
  () => {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  };
}

/// Extracts a value whose presence or success is guaranteed by a documented program invariant.
///
/// Production code must use this macro instead of calling `Option::expect` or `Result::expect`
/// directly. The reason must describe why failure is a programmer error rather than a recoverable
/// runtime condition.
macro_rules! expect_invariant {
  ($value:expr, $reason:literal $(,)?) => {{
    #[allow(clippy::expect_used)]
    let value = $value.expect(concat!("invariant violated: ", $reason));
    value
  }};
}

/// Panics for a documented, unrecoverable program invariant violation.
///
/// Production code must use this macro instead of calling `panic!` directly. The literal reason
/// must explain why the failure is a programmer error rather than a recoverable runtime condition.
/// The reason documents the lint exemption; the panic message is preserved without a prefix.
macro_rules! panic_invariant {
  ($reason:literal, $($message:tt)+) => {{
    #[allow(clippy::panic, reason = $reason)]
    {
      panic!($($message)+)
    }
  }};
}

macro_rules! unwrap_caught_result {
  ($result:expr) => {{
    match $result {
      Ok(result) => result,
      Err(payload) => std::panic::resume_unwind(payload),
    }
  }};
}

macro_rules! resume_caught_panic {
  ($result:expr) => {{
    if let Err(payload) = $result {
      std::panic::resume_unwind(payload);
    }
  }};
}

/// Declares a mutable slice backed by a local array or, for larger lengths, a vector.
///
/// Usage: `stack_or_heap_buffer!(name, element_type, length, stack_capacity, initial_value);`
/// The element type must be `Copy`; stack capacity is a compile-time element count,
/// not a byte budget. Length and initial value are each evaluated once.
///
/// Expands into declarations in the caller's scope so both backing buffers outlive the
/// slice. The fixed array can occupy stack space even when the heap branch is selected;
/// callers must account for nested calls and recursion when choosing its capacity.
macro_rules! stack_or_heap_buffer {
  ($name:ident, $element:ty, $length:expr, $capacity:expr, $initial:expr $(,)?) => {
    let buffer_length: usize = $length;
    let buffer_initial: $element = $initial;
    let mut stack_buffer: [$element; $capacity] = [buffer_initial; $capacity];
    let mut heap_buffer;
    let $name: &mut [$element] = if buffer_length <= stack_buffer.len() {
      &mut stack_buffer[..buffer_length]
    } else {
      heap_buffer = ::std::vec![buffer_initial; buffer_length];
      heap_buffer.as_mut_slice()
    };
  };
}

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

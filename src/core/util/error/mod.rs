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
pub mod lucene_error;
pub mod parse;

#[macro_export]
macro_rules! message_error {
  ($name:ident) => {
    #[derive(Debug)]
    pub struct $name {
      pub message: String,
      pub source: Option<Box<$crate::core::util::error::lucene_error::LuceneError>>,
    }

    impl $name {
      pub fn new(msg: impl Into<String>) -> Self {
        Self {
          message: msg.into(),
          source: None,
        }
      }
      pub fn add_suppressed(
        &mut self,
        source: $crate::core::util::error::lucene_error::LuceneError,
      ) {
        self.source = Some(Box::new(source));
      }

      pub fn get_suppressed(
        &self,
      ) -> Option<&$crate::core::util::error::lucene_error::LuceneError> {
        self.source.as_deref()
      }
    }

    impl From<String> for $name {
      fn from(msg: String) -> Self {
        Self::new(msg)
      }
    }

    impl From<&str> for $name {
      fn from(msg: &str) -> Self {
        Self::new(msg)
      }
    }
    impl std::fmt::Display for $name {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
      }
    }

    impl std::error::Error for $name {
      fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|e| e as &dyn std::error::Error)
      }
    }
  };
}
message_error!(AlreadyClosedError);
message_error!(ArrayIndexOutOfBoundsError);
message_error!(BufferAllocationError);
message_error!(CollectionTerminatedError);
message_error!(CorruptIndexError);
message_error!(Eof);
message_error!(FuzzyTermsError);
message_error!(IllegalArgumentError);
message_error!(IllegalStateError);
message_error!(IndexFormatTooNewError);
message_error!(IndexFormatTooOldError);
message_error!(IndexNotFound);
message_error!(LockAlreadyHeldError);
message_error!(LockHeldByOtherError);
message_error!(LockObtainFailedError);
message_error!(LockReleaseFailedError);
message_error!(MaxBytesLengthExceededError);
message_error!(MergeAbortedError);
message_error!(MergeError);
message_error!(NeedImplementedError);
message_error!(NoMoreTermsError);
message_error!(NoSuchElementError);
message_error!(NotImplementedError);
message_error!(NotSuchFileError);
message_error!(NumberFormatError);
message_error!(NumberOverflow);
message_error!(TimeExceededError);
message_error!(TooComplexToDeterminizeError);
message_error!(TooManyClausesError);
message_error!(TooManyNestedClausesError);
message_error!(UncheckedIOError);
message_error!(UnreachableError);
message_error!(UnsupportedOperationError);
message_error!(VersionError);

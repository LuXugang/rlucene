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
            pub source: Option<Box<$crate::util::error::lucene_error::LuceneError>>,
        }

        impl $name {
            pub fn new(msg: impl Into<String>) -> Self {
                Self {
                    message: msg.into(),
                    source: None,
                }
            }

            pub fn from_error(err: $crate::util::error::lucene_error::LuceneError) -> Self {
                Self {
                    message: err.to_string(),
                    source: Some(Box::new(err)),
                }
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

        impl From<crate::util::error::lucene_error::LuceneError> for $name {
            fn from(err: crate::util::error::lucene_error::LuceneError) -> Self {
                Self::from_error(err)
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
message_error!(NeedImplementedError);
message_error!(NotImplementedError);
message_error!(UnreachableError);
message_error!(NumberFormatError);
message_error!(IllegalArgumentError);
message_error!(IllegalStateError);
message_error!(Eof);
message_error!(NumberOverflow);
message_error!(CorruptIndexError);
message_error!(IndexFormatTooNewError);
message_error!(IndexFormatTooOldError);
message_error!(UnsupportedOperationError);
message_error!(NotFoundError);
message_error!(LockAlreadyHeldError);
message_error!(LockHeldByOtherError);
message_error!(ArrayIndexOutOfBoundsError);
message_error!(IndexNotFound);
message_error!(MaxBytesLengthExceededError);
message_error!(BufferAllocationError);
message_error!(MergeError);
message_error!(MergeAbortedError);
message_error!(AlreadyClosedError);
message_error!(VersionError);
message_error!(TooComplexToDeterminizeError);
message_error!(NoSuchElementError);

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
pub mod lucene_error;
pub mod parse;

#[macro_export]
macro_rules! message_error {
    ($name:ident) => {
        #[derive(Debug)]
        pub struct $name {
            pub message: String,
        }

        impl $name {
            pub fn new(msg: impl Into<String>) -> Self {
                Self {
                    message: msg.into(),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message)
            }
        }

        impl std::error::Error for $name {}
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

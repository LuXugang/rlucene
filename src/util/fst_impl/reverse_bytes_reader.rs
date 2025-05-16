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

use crate::store::DataInput;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::BytesReader;

#[macro_export]
macro_rules! define_reverse_bytes_reader {
    ($bytes_type:ty) => {
        /// Reads in reverse from a single byte array.
        pub struct ReverseBytesReader {
            bytes: $bytes_type,
            pos: i32,
        }

        #[allow(unused)]
        impl ReverseBytesReader {
            pub fn new(bytes: $bytes_type) -> Self {
                Self { bytes, pos: 0 }
            }
        }

        impl DataInput for ReverseBytesReader {
            fn read_byte(&mut self) -> Result<u8> {
                let b = self.bytes[self.pos as usize];
                self.pos -= 1;
                Ok(b)
            }

            fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
                let offset = offset as usize;
                for i in 0..len as usize {
                    b[offset + i] = self.bytes[self.pos as usize];
                    self.pos -= 1;
                }
                Ok(())
            }

            fn skip_bytes(&mut self, count: i64) -> Result<()> {
                self.pos -= count as i32;
                Ok(())
            }
        }

        impl BytesReader for ReverseBytesReader {
            fn get_position(&self) -> i64 {
                self.pos as i64
            }

            fn set_position(&mut self, pos: i64) {
                self.pos = pos as i32;
            }
        }

        impl std::fmt::Display for ReverseBytesReader {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "ReverseBytesReader")
            }
        }
    };
}
#[cfg(test)]
define_reverse_bytes_reader!(Rc<Vec<u8>>);

#[cfg(not(test))]
define_reverse_bytes_reader!(Vec<u8>);

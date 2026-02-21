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
use crate::core::index::BytesRef;
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::{HashCode, OutputIdentity};
use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

/// Represents the outputs for an FST, providing the basic algebra required for
/// building and traversing the FST.
///
/// Note that any operation that returns `NO_OUTPUT` must return the same
/// SINGLETON object from [`get_no_output`](Outputs::get_no_output).
///
/// # lucene.experimental
pub trait Outputs: Display + Clone + Default {
    // TODO: maybe change this API to allow for re-use of the
    // output instances -- this is an insane amount of garbage
    // (new object per byte/char/int) if eg used during
    // analysis

    type V: OutputsBound;

    /// Eg. `common("foobar", "food") -> "foo"`
    fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V;

    /// Eg. `subtract("foobar", "foo") -> "bar"`
    fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V;

    /// Eg. `add("foo", "bar") -> "foobar"`
    fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V;

    /// Encode an output value into a `Write` stream.
    fn write(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()>;

    /// Encode a final node output value into a `Write` stream.
    /// By default this just calls [`write`].
    fn write_final_output(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()> {
        self.write(output, out)
    }

    /// Decode an output value previously written with [`write`].
    fn read(&self, input: &mut impl DataInput) -> Result<Self::V>;

    /// Skip the output; defaults to just calling [`read`] and discarding the
    /// result.
    fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
        let _ = self.read(input)?;
        Ok(())
    }

    /// Decode an output value previously written with [`write_final_output`].
    /// By default this just calls [`read`].
    fn read_final_output(&self, input: &mut impl DataInput) -> Result<Self::V> {
        self.read(input)
    }

    /// Skip the output previously written with [`write_final_output`];
    /// defaults to just calling [`read_final_output`] and discarding the
    /// result.
    fn skip_final_output(&self, input: &mut impl DataInput) -> Result<()> {
        self.skip_output(input)?;
        Ok(())
    }

    /// NOTE: this output is compared with pointer equality (`==`), so you must
    /// ensure that all methods return the same SINGLETON object if it's
    /// really no output.
    fn get_no_output(&self) -> Self::V;

    fn output_to_string(&self, output: &Self::V) -> String;

    fn merge(&self, _first: &Self::V, _second: &Self::V) -> Result<Self::V> {
        Err(LuceneError::unsupported_operation(""))
    }

    /// Return memory usage for the provided output.
    ///
    /// See also: `Accountable`
    fn ram_bytes_used(&self, output: &Self::V) -> i64;
}

pub enum OutputsEnum {
    ByteSequence(ByteSequenceOutputs),
}

pub trait OutputsBound:
    Clone + PartialEq + Default + HashCode + Hash + Display + OutputIdentity
{
}
impl OutputsBound for Arc<i64> {}
impl OutputsBound for BytesRef<Arc<Vec<u8>>> {}
impl OutputsBound for IntsRef<Arc<Vec<i32>>> {}
// impl<T: Clone + PartialEq + Default + Hash + Display> OutputsBound for T {}

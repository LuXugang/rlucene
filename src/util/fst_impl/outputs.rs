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
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use std::fmt::Display;
use std::hash::Hash;

/// Represents the outputs for an FST, providing the basic algebra required for building and
/// traversing the FST.
///
/// Note that any operation that returns `NO_OUTPUT` must return the same SINGLETON object from
/// [`get_no_output`](Outputs::get_no_output).
///
/// # lucene.experimental
pub trait Outputs<T>: Display + Clone
where
    T: OutputsBound,
{
    // TODO: maybe change this API to allow for re-use of the
    // output instances -- this is an insane amount of garbage
    // (new object per byte/char/int) if eg used during
    // analysis

    /// Eg. `common("foobar", "food") -> "foo"`
    fn common(&self, output1: &T, output2: &T) -> T;

    /// Eg. `subtract("foobar", "foo") -> "bar"`
    fn subtract(&self, output: &T, inc: &T) -> T;

    /// Eg. `add("foo", "bar") -> "foobar"`
    fn add(&self, prefix: &T, output: &T) -> T;

    /// Encode an output value into a `Write` stream.
    fn write(&self, output: &T, out: &mut impl DataOutput) -> Result<()>;

    /// Encode a final node output value into a `Write` stream.
    /// By default this just calls [`write`].
    fn write_final_output(&self, output: &T, out: &mut impl DataOutput) -> Result<()> {
        self.write(output, out)
    }

    /// Decode an output value previously written with [`write`].
    fn read(&self, input: &mut impl DataInput) -> Result<T>;

    /// Skip the output; defaults to just calling [`read`] and discarding the result.
    fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
        let _ = self.read(input)?;
        Ok(())
    }

    /// Decode an output value previously written with [`write_final_output`].
    /// By default this just calls [`read`].
    fn read_final_output(&self, input: &mut impl DataInput) -> Result<T> {
        self.read(input)
    }

    /// Skip the output previously written with [`write_final_output`];
    /// defaults to just calling [`read_final_output`] and discarding the result.
    fn skip_final_output(&self, input: &mut impl DataInput) -> Result<()> {
        self.skip_output(input)?;
        Ok(())
    }

    /// NOTE: this output is compared with pointer equality (`==`), so you must ensure that
    /// all methods return the same SINGLETON object if it's really no output.
    fn get_no_output(&self) -> T;

    fn output_to_string(&self, output: &T) -> String;

    // TODO: maybe make valid(T output) public...? for asserts

    fn merge(&self, _first: &T, _second: &T) -> Result<T> {
        Err(LuceneError::unsupported_operation(""))
    }

    /// Return memory usage for the provided output.
    ///
    /// See also: `Accountable`
    fn ram_bytes_used(&self, output: &T) -> i64;
}

pub enum OutputsEnum {
    ByteSequence(ByteSequenceOutputs),
}

pub trait OutputsBound: Clone + PartialEq + Default + Hash + Display {}
impl<T: Clone + PartialEq + Default + Hash + Display> OutputsBound for T {}

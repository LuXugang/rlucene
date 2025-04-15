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
use crate::store::{ByteArrayDataOutput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::dummy::dummy_bytes_reader::{DummyBytesReader, InputType};
use crate::util::fst_impl::fst::{fst_util, FST};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::fst_impl::read_write_data_output::ReadWriteDataOutput;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

pub struct FSTCompiler<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    no_output: T,
    fst: FST<T, O, F>,
}
pub mod fst_compiler_util {
    use crate::util::error::lucene_error::Result;
    use crate::util::fst_impl::read_write_data_output::ReadWriteDataOutput;
    /// Maximum oversizing factor allowed for direct addressing.
    pub(crate) const DIRECT_ADDRESSING_MAX_OVERSIZING_FACTOR: f32 = 1.0;

    /// Minimum depth at which fixed-length arcs are considered for shallow nodes.
    ///
    /// See [`FSTCompiler::should_expand_node_with_fixed_length_arcs`].
    pub(crate) const FIXED_LENGTH_ARC_SHALLOW_DEPTH: i32 = 3;

    /// Minimum number of arcs required to consider fixed-length arcs at shallow depth.
    ///
    /// See [`FSTCompiler::should_expand_node_with_fixed_length_arcs`].
    pub(crate) const FIXED_LENGTH_ARC_SHALLOW_NUM_ARCS: i32 = 5;

    /// Minimum number of arcs required to consider fixed-length arcs at deep depth.
    ///
    /// See [`FSTCompiler::should_expand_node_with_fixed_length_arcs`].
    pub(crate) const FIXED_LENGTH_ARC_DEEP_NUM_ARCS: i32 = 10;

    /// Maximum oversizing factor allowed for direct addressing compared to binary search when
    /// expansion credits allow the oversizing. This factor prevents expansions that are obviously
    /// too costly even if there are sufficient credits.
    ///
    /// See [`FSTCompiler::should_expand_node_with_direct_addressing`].
    pub(super) const DIRECT_ADDRESSING_MAX_OVERSIZE_WITH_CREDIT_FACTOR: f32 = 1.66;
    pub fn get_on_heap_reader_writer(block_bits: i32) -> Result<ReadWriteDataOutput> {
        Ok(ReadWriteDataOutput::new(block_bits))
    }
}
impl<T, O, F> FSTCompiler<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    fn valid_output(&self, output: &T) -> bool {
        std::ptr::eq(output, &self.no_output) || *output != self.no_output
    }
}

/// This class is used for FST backed by non-FSTReader DataOutput. It does not allow getting the
/// reverse BytesReader nor writing to a DataOutput.
struct NullFSTReader;
#[allow(unused)]
impl Accountable for NullFSTReader {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
#[allow(unused)]
impl FstReader for NullFSTReader {
    type FstBytesReader = DummyBytesReader;

    fn get_reverse_bytes_reader(&mut self) -> Result<Self::FstBytesReader> {
        Err(LuceneError::unsupported_operation(
            "FST was not constructed with getOnHeapReaderWriter()".to_string(),
        ))
    }

    fn write_to(&mut self, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "FST was not constructed with getOnHeapReaderWriter()".to_string(),
        ))
    }
}
/// Fluent-style builder for constructing an [`FSTCompiler`].
///
/// Creates an FST/FSA builder with all possible tuning and construction tweaks.
/// Read parameter documentation carefully.
pub struct Builder<T, O>
where
    T: OutputsBound,
    O: Outputs<T>,
{
    input_type: InputType,
    outputs: O,
    suffix_ram_limit_mb: f64,
    allow_fixed_length_arcs: bool,
    data_output: Option<ReadWriteDataOutput>,
    direct_addressing_max_oversizing_factor: f32,
    version: i32,
    phantom: PhantomData<T>,
}
impl<T, O> Builder<T, O>
where
    T: OutputsBound,
    O: Outputs<T>,
{
    /// Creates a new [`Builder`] with the given input type and outputs.
    ///
    /// - `input_type`: The input type (transition labels). Can be any variant of [`InputType`].
    ///   Shorter types consume less memory. Strings (character sequences) are typically represented
    ///   using [`InputType::Byte4`] for full Unicode codepoints.
    ///
    /// - `outputs`: The output type for each input sequence. Applies only when building an FST.
    ///   For FSA, use [`NoOutputs::singleton()`](crate::util::fst_impl::no_outputs::NoOutputs::get_singleton) and [`NoOutputs::no_output()`](crate::util::fst_impl::no_outputs::NoOutputs::get_no_output) as the singleton output.
    pub fn new(input_type: InputType, outputs: O) -> Self {
        Self {
            input_type,
            outputs,
            suffix_ram_limit_mb: 32.0,
            allow_fixed_length_arcs: true,
            data_output: None,
            direct_addressing_max_oversizing_factor:
                fst_compiler_util::DIRECT_ADDRESSING_MAX_OVERSIZING_FACTOR,
            version: fst_util::VERSION_CURRENT,
            phantom: Default::default(),
        }
    }
    /// Sets the approximate maximum amount of RAM (in MB) to use for holding the suffix cache.
    ///
    /// This cache enables the FST to share common suffixes. Passing `f64::INFINITY` keeps all
    /// suffixes, resulting in an exactly minimal FST. The actual memory usage in that case will be
    /// bounded by the number of unique suffixes.
    ///
    /// If a smaller value is passed, the least recently used suffixes are discarded, reducing
    /// suffix sharing and producing a non-minimal FST. The larger the limit, the closer the result
    /// will be to the minimal FST, with diminishing returns.
    ///
    /// Pass `0.0` to disable suffix sharing entirely (may result in a substantially larger FST).
    ///
    /// Note: this is an approximate limit. The implementation uses hash tables to map suffixes and
    /// estimates overhead from unused slots.
    ///
    /// Default: `32.0`
    pub fn suffix_ram_limit_mb(mut self, mb: f64) -> Result<Self> {
        if mb < 0f64 {
            return Err(LuceneError::illegal_argument(format!(
                "suffix_ram_limit_mb must be >= 0; got: {}",
                mb
            )));
        }
        self.suffix_ram_limit_mb = mb;
        Ok(self)
    }

    /// Controls whether fixed-length arc optimization (binary search or direct addressing) is enabled.
    ///
    /// Disabling this makes the resulting FST smaller but slower to traverse.
    ///
    /// Default: `true`
    pub fn allow_fixed_length_arcs(mut self, allow: bool) -> Self {
        self.allow_fixed_length_arcs = allow;
        self
    }

    /// Set the [`DataOutput`] which is used for low-level writing of FST. If you want the FST to
    /// be immediately readable, you need to use [`fst_compiler_util::get_on_heap_reader_writer`].
    ///
    /// Otherwise you need to construct the corresponding [`DataInput`](crate::store::data_input::DataInput)
    /// and use the FST constructor to read it.
    ///
    /// # Arguments
    ///
    /// * `data_output` - the `DataOutput`
    ///
    /// # Returns
    ///
    /// This builder.
    ///
    /// # See also
    ///
    /// [`fst_compiler_util::get_on_heap_reader_writer`]
    pub fn data_output(mut self, data_output: ReadWriteDataOutput) -> Self {
        self.data_output = Some(data_output);
        self
    }
    /// Overrides the default maximum oversizing of fixed array allowed to enable direct
    /// addressing of arcs instead of binary search.
    ///
    /// Setting this factor to a negative value (e.g. `-1`) effectively disables direct addressing,
    /// only binary search nodes will be created.
    ///
    /// This factor does not determine whether to encode a node with a list of variable length
    /// arcs or with fixed length arcs. It only determines the effective encoding of a node that is
    /// already known to be encoded with fixed length arcs.
    ///
    ///
    /// Default = `1`.

    pub fn with_direct_addressing_max_oversizing_factor(mut self, factor: f32) -> Self {
        self.direct_addressing_max_oversizing_factor = factor;
        self
    }
    ///  Expert: Set the codec version.
    pub fn with_version(mut self, version: i32) -> Result<Self> {
        if (fst_util::VERSION_90..=fst_util::VERSION_CURRENT).contains(&version) {
            return Err(LuceneError::illegal_argument(format!(
                "Version must be in range [{} - {}]; got: {}",
                fst_util::VERSION_90,
                fst_util::VERSION_CURRENT,
                version
            )));
        }

        self.version = version;
        Ok(self)
    }
    /// Creates a new {@link FSTCompiler}
    pub fn build(mut self) -> Result<FSTCompiler<T, O, ReadWriteDataOutput>> {
        if self.data_output.is_none() {
            self.data_output = Some(fst_compiler_util::get_on_heap_reader_writer(15)?);
        }
        todo!()
    }
}
/// Expert: holds a pending (seen but not yet serialized) arc.
pub(crate) struct Arc<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    pub label: i32, // really an "unsigned" byte
    pub target: NodeEnum<T, O, F>,
    pub is_final: bool,
    pub output: T,
    pub next_final_output: T,
}
impl<T, O, F> Default for Arc<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    fn default() -> Self {
        Self {
            label: 0,
            target: NodeEnum::CompiledNode(CompiledNode::default()),
            is_final: false,
            output: T::default(),
            next_final_output: T::default(),
        }
    }
}

/// # NOTE:
/// Not many instances of Node or CompiledNode are in
/// memory while the FST is being built; it's only the
/// current "frontier":
pub(crate) trait Node {
    fn is_compiled(&self) -> bool;
}
pub(crate) enum NodeEnum<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    UnCompiledNode(UnCompiledNode<T, O, F>),
    CompiledNode(CompiledNode),
}
impl<T, O, F> Node for NodeEnum<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    fn is_compiled(&self) -> bool {
        match self {
            NodeEnum::UnCompiledNode(node) => node.is_compiled(),
            NodeEnum::CompiledNode(node) => node.is_compiled(),
        }
    }
}
#[derive(Default)]
pub(crate) struct CompiledNode {
    node: i64,
}
impl CompiledNode {
    pub(crate) fn new() -> Self {
        Self { node: 0 }
    }
}
impl Node for CompiledNode {
    fn is_compiled(&self) -> bool {
        true
    }
}
/// Expert: holds a pending (seen but not yet serialized) Node.
pub(crate) struct UnCompiledNode<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    pub owner: Rc<RefCell<FSTCompiler<T, O, F>>>,
    pub num_arcs: i32,
    pub arcs: Vec<Arc<T, O, F>>,
    // TODO: instead of recording is_final/output on the node,
    // maybe we should use -1 arc to mean "end" (like we do when reading the FST).
    // Would simplify much code here...
    pub output: T,
    pub is_final: bool,

    /// This node's depth, starting from the automaton root.
    pub depth: i32,
}
impl<T, O, F> UnCompiledNode<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    /// Creates a new uncompiled node.
    ///
    /// # Parameters
    /// - `depth`: The node's depth starting from the automaton root.
    ///   Needed for LUCENE-2934 (node expansion based on conditions other than the fanout size).
    pub(crate) fn new(owner: Rc<RefCell<FSTCompiler<T, O, F>>>, depth: i32) -> Self {
        let mut arcs = Vec::with_capacity(1);
        arcs.push(Arc::default());

        let output = owner.borrow().no_output.clone();

        Self {
            owner,
            num_arcs: 0,
            arcs,
            output,
            is_final: false,
            depth,
        }
    }

    pub(crate) fn is_compiled(&self) -> bool {
        false
    }

    pub(crate) fn clear(&mut self) {
        self.num_arcs = 0;
        self.is_final = false;
        self.output = self.owner.borrow().no_output.clone();
        // We don't clear the depth here because it never changes
        // for nodes on the frontier (even when reused).
    }

    pub(crate) fn get_last_output(&self, label_to_match: i32) -> T {
        debug_assert!(self.num_arcs > 0);
        debug_assert!(self.arcs[self.num_arcs as usize - 1].label == label_to_match);
        self.arcs[self.num_arcs as usize - 1].output.clone()
    }

    pub(crate) fn add_arc(&mut self, label: i32, target: NodeEnum<T, O, F>) -> Result<()> {
        debug_assert!(label >= 0);
        debug_assert!(
            self.num_arcs == 0 || label > self.arcs[self.num_arcs as usize - 1].label,
            "arc[numArcs-1].label={} new label={} numArcs={}",
            self.arcs[self.num_arcs as usize - 1].label,
            label,
            self.num_arcs
        );

        if self.num_arcs as usize == self.arcs.len() {
            ArrayUtil::grow(&mut self.arcs)?;
        }

        let arc = &mut self.arcs[self.num_arcs as usize];
        self.num_arcs += 1;
        arc.label = label;
        arc.target = target;
        arc.output = self.owner.borrow().no_output.clone();
        arc.next_final_output = arc.output.clone();
        arc.is_final = false;
        Ok(())
    }
    pub(crate) fn replace_last(
        &mut self,
        label_to_match: i32,
        target: NodeEnum<T, O, F>,
        next_final_output: T,
        is_final: bool,
    ) {
        debug_assert!(self.num_arcs > 0);
        let arc = &mut self.arcs[self.num_arcs as usize - 1];
        debug_assert_eq!(
            arc.label, label_to_match,
            "arc.label={} vs {}",
            arc.label, label_to_match
        );
        arc.target = target;
        arc.next_final_output = next_final_output;
        arc.is_final = is_final;
    }

    pub(crate) fn set_last_output(&mut self, label_to_match: i32, new_output: T) {
        debug_assert!(self.owner.borrow().valid_output(&new_output));
        debug_assert!(self.num_arcs > 0);
        let arc = &mut self.arcs[self.num_arcs as usize - 1];
        debug_assert_eq!(arc.label, label_to_match);
        arc.output = new_output;
    }

    /// Pushes an output prefix forward onto all arcs.
    pub(crate) fn prepend_output(&mut self, output_prefix: &T) {
        debug_assert!(self.owner.borrow().valid_output(output_prefix));
        let owner = self.owner.borrow();
        let outputs = owner.fst.outputs.borrow();

        for i in 0..self.num_arcs as usize {
            let new_output = outputs.add(output_prefix, &self.arcs[i].output);
            debug_assert!(self.owner.borrow().valid_output(&new_output));
            self.arcs[i].output = new_output;
        }

        if self.is_final {
            let new_output = outputs.add(output_prefix, &self.output);
            debug_assert!(self.owner.borrow().valid_output(&new_output));
            self.output = new_output;
        }
    }
}
impl<T, O, F> Node for UnCompiledNode<T, O, F>
where
    T: OutputsBound,
    F: FstReader,
    O: Outputs<T>,
{
    fn is_compiled(&self) -> bool {
        false
    }
}

/// Reusable buffer for building nodes with fixed length arcs (binary search or direct addressing).
pub(crate) struct FixedLengthArcsBuffer {
    bado: ByteArrayDataOutput,
}
impl FixedLengthArcsBuffer {
    pub(crate) fn new() -> Self {
        // Initial capacity is the max length required for the header of a node with fixed length arcs:
        // header(byte) + numArcs(vint) + numBytes(vint)
        let bytes = vec![0u8; 11];
        let bado = ByteArrayDataOutput::with_bytes(bytes);
        Self { bado }
    }
    /// Ensures the capacity of the internal byte array. Enlarges it if needed.
    pub(crate) fn ensure_capacity(&mut self, capacity: i32) -> Result<()> {
        if self.bado.bytes.len() < capacity as usize {
            ArrayUtil::grow_with_len(&mut self.bado.bytes, ArrayUtil::oversize(capacity, 1))?;
            self.bado.reset()?;
        }
        Ok(())
    }

    pub(crate) fn reset_position(&mut self) -> Result<()> {
        self.bado.reset()
    }

    pub(crate) fn write_byte(&mut self, b: u8) -> Result<()> {
        self.bado.write_byte(b)
    }

    pub(crate) fn write_vint(&mut self, i: i32) -> Result<()> {
        self.bado.write_vint(i)
    }

    pub(crate) fn get_position(&self) -> i32 {
        self.bado.get_position()
    }

    /// Gets the internal byte array.
    pub(crate) fn get_bytes(&mut self) -> &mut [u8] {
        &mut self.bado.bytes
    }
}

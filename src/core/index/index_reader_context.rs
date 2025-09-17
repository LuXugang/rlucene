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
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_reader::{IndexReader, IndexReaderEnum};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;

/// A struct like class that represents a hierarchical relationship between IndexReader instances.
#[allow(private_bounds)]
pub trait IndexReaderContext: IndexReaderContextSealed {
    type IndexReader: IndexReader;
    /// Returns the [`IndexReader`], this context represents.
    fn reader(&self) -> &Self::IndexReader;
    /// Returns the context's leaves if this context is a top-level context.
    ///
    /// For convenience, if this is a [`LeafReaderContext`] this returns itself as the only leaf,
    /// and it will never return a `None`.
    ///
    /// # Error
    ///
    /// Error with `UnsupportedOperationException` if this is not a top-level context.
    /// [`IndexReaderContext::children`]
    fn leaves(&self) -> Result<&[LeafReaderContext]>;

    /// Returns the context's children iff this context is a composite context otherwise None.
    fn children(&self) -> Option<&[IndexReaderContextEnum]>;

    fn get_index_reader_context_base(&self) -> &IndexReaderContextBase;
    fn get_index_reader_context_base_mut(&mut self) -> &mut IndexReaderContextBase;
}

pub struct IndexReaderContextBase {
    /// The reader context for this reader's immediate parent, or `None` if none.
    pub parent: Option<CompositeReaderContext>,

    /// `true` if this context struct represents the top-level reader within the hierarchical context.
    pub is_top_level: bool,

    /// The doc base for this reader in the parent, `0` if parent is `None`.
    pub doc_base_in_parent: i32,

    /// The ord for this reader in the parent, `0` if parent is `None`.
    pub ord_in_parent: i32,
    // An object that uniquely identifies this context without referencing segments.
    /// In Rust we model it as an `Arc<()>` so that pointer equality can be used for identity.
    pub identity: Rc<()>,
}

impl IndexReaderContextBase {
    pub fn new(
        parent: Option<CompositeReaderContext>,
        ord_in_parent: i32,
        doc_base_in_parent: i32,
    ) -> Self {
        let is_top_level = parent.is_none();
        Self {
            parent,
            is_top_level,
            doc_base_in_parent,
            ord_in_parent,
            identity: Rc::new(()),
        }
    }

    pub fn id(&self) -> &Rc<()> {
        &self.identity
    }
}
// Similar to Java's sealed trait pattern
pub(crate) trait IndexReaderContextSealed {}

pub enum IndexReaderContextEnum {
    Composite(CompositeReaderContext),
    Leaf(LeafReaderContext),
}

impl IndexReaderContextSealed for IndexReaderContextEnum {}

impl IndexReaderContext for IndexReaderContextEnum {
    type IndexReader = IndexReaderEnum;

    fn reader(&self) -> &Self::IndexReader {
        todo!()
    }

    fn leaves(&self) -> Result<&[LeafReaderContext]> {
        todo!()
    }

    fn children(&self) -> Option<&[IndexReaderContextEnum]> {
        todo!()
    }

    fn get_index_reader_context_base(&self) -> &IndexReaderContextBase {
        todo!()
    }

    fn get_index_reader_context_base_mut(&mut self) -> &mut IndexReaderContextBase {
        todo!()
    }
}

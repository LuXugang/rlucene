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
use crate::index::buffered_updates::BufferedUpdates;
use crate::index::doc_values_type::DocValuesType;
use crate::index::doc_values_update::{DocValuesUpdate, DocValuesUpdateBase};
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::index::term::Term;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::LuceneError;
use crate::util::info_stream::InfoStreamEnum;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard};

pub struct DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    global_buffer_lock: RwLock<GlobalState<Q>>,
    generation: i64,
    next_seq_no: Arc<AtomicI64>,
    info_stream: Arc<Mutex<InfoStreamEnum>>,
    start_seq_no: i64,
    previous_max_seq_id: i64,
}
impl<Q> DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    pub fn new(info_stream: Arc<Mutex<InfoStreamEnum>>) -> Self {
        Self::with_params(info_stream, 0, 1, 0)
    }
}
impl<Q> DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    pub fn with_params(
        info_stream: Arc<Mutex<InfoStreamEnum>>,
        generation: i64,
        start_seq_no: i64,
        previous_max_seq_id: i64,
    ) -> Self {
        let tail = Arc::new(Node::new(NodeEnum::EmptyNode(EmptyNode::new())));
        let value = previous_max_seq_id;
        debug_assert!(
            value <= start_seq_no,
            "illegal max sequence ID: {} start was: {}",
            value,
            start_seq_no
        );
        let global_slice = GlobalState::new(tail, generation);

        Self {
            global_buffer_lock: RwLock::new(global_slice),
            generation,
            next_seq_no: Arc::new(AtomicI64::new(start_seq_no)),
            info_stream,
            start_seq_no,
            previous_max_seq_id,
        }
    }
    pub fn add_delete_query(&self, queries: Vec<Arc<Q>>) -> Result<i64, LuceneError> {
        let query_array_node = Node::new(NodeEnum::QueryNodeArray(QueryNodeArray::new(queries)));
        let seq_no = self.add_node(Arc::new(query_array_node))?;
        self.try_apply_global_slice()?;
        Ok(seq_no)
    }
    pub fn add_delete_term(&self, terms: Vec<Term>) -> Result<i64, LuceneError> {
        let node = Node::new(NodeEnum::TermNodeArray(TermNodeArray::new(terms)));
        let seq_no = self.add_node(Arc::new(node))?;
        self.try_apply_global_slice()?;
        Ok(seq_no)
    }
    pub fn add_doc_values_updates(
        &self,
        updates: Vec<DocValuesUpdate>,
    ) -> Result<i64, LuceneError> {
        let node = Node::new(NodeEnum::DocValuesUpdatesNode(DocValuesUpdatesNode::new(
            updates,
        )));
        let seq_no = self.add_node(Arc::new(node))?;
        self.try_apply_global_slice()?;
        Ok(seq_no)
    }
    pub fn new_node_for_term(term: Term) -> Node<Q> {
        Node::new(NodeEnum::TermNode(TermNode::new(term)))
    }

    pub fn new_node_for_query(query: Q) -> Node<Q> {
        Node::new(NodeEnum::QueryNode(QueryNode::new(Arc::new(query))))
    }

    pub fn new_node_for_doc_values(updates: &[DocValuesUpdate]) -> Node<Q> {
        Node::new(NodeEnum::DocValuesUpdatesNode(DocValuesUpdatesNode::new(
            updates.to_vec(),
        )))
    }
    pub fn add_with_slice(
        &self,
        delete_node: Arc<Node<Q>>,
        slice: &mut DeleteSlice<Q>,
    ) -> Result<i64, LuceneError> {
        let seq_no = self.add_node(delete_node.clone())?;
        // This is an update request where the term is the updated documents
        // delTerm. In that case we need to guarantee that this insert is atomic
        // with regards to the given delete slice. This means if two threads try to
        // update the same document with in turn the same delTerm one of them must
        // win. By taking the node we have created for our del term as the new tail
        // it is guaranteed that if another thread adds the same right after us we
        // will apply this delete next time we update our slice and one of the two
        // competing updates wins!
        slice.slice_tail = delete_node;
        debug_assert!(
            !Arc::ptr_eq(&slice.slice_head, &slice.slice_tail),
            "slice head and tail must differ after add"
        );
        // TODO doing this each time is not necessary maybe
        // we can do it just every n times or so?
        self.try_apply_global_slice()?;
        Ok(seq_no)
    }

    pub fn add_node(&self, new_node: Arc<Node<Q>>) -> Result<i64, LuceneError> {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;

        self.ensure_open(global_state.closed)?;
        {
            let mut tail_next_guard =
                global_state.tail.next.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock".to_string())
                })?;
            *tail_next_guard = Option::from(new_node.clone());
        }
        global_state.tail = new_node;

        Ok(self.get_next_sequence_number(global_state.max_seq_no))
    }

    pub(crate) fn any_changes(&self) -> Result<bool, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        self.any_changes_with_lock(&global_state)
    }
    pub(crate) fn any_changes_with_lock(
        &self,
        global_state: &RwLockWriteGuard<GlobalState<Q>>,
    ) -> Result<bool, LuceneError> {
        //  Check if all items in the global slice were applied,
        //  if the global slice is up-to-date,
        //  and if `global_buffered_updates` has changes.
        let tail_next = global_state
            .tail
            .next
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        Ok(global_state.global_buffered_updates.any()
            || !global_state.global_slice.is_empty()
            || !Arc::ptr_eq(&global_state.global_slice.slice_tail, &global_state.tail)
            || tail_next.is_some())
    }
    fn try_apply_global_slice(&self) -> Result<(), LuceneError> {
        match self.global_buffer_lock.try_write() {
            Ok(mut global_state) => {
                self.ensure_open(global_state.closed)?;
                // The global buffer must be locked, but we don't need to update them if
                //there is an update going on right now. It is sufficient to apply the
                //deletes that have been added after the current in-flight global slices
                //tail the next time we can get the lock!
                if self.update_slice_no_seq_no(&mut global_state) {
                    global_state.apply(BufferedUpdates::MAX_INT)?;
                }
            }
            Err(_) => {
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn freeze_global_buffer<D>(
        &mut self,
        caller_slice: Option<&mut DeleteSlice<Q>>,
    ) -> Result<Option<FrozenBufferedUpdates<D, Q>>, LuceneError>
    where
        D: Directory,
    {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        self.ensure_open(global_state.closed)?;
        // Here we freeze the global buffer so we need to lock it, apply all
        // deletes in the queue and reset the global slice to let the GC prune the
        // queue.
        // Take the current tail make this local any
        let current_tail = global_state.tail.clone();
        // Changes after this call are applied later
        // and not relevant here
        if let Some(slice) = caller_slice {
            // Update the callers slices so we are on the same page
            slice.slice_tail = current_tail.clone();
        }
        let result = self.freeze_global_buffer_internal(&mut global_state, current_tail)?;
        Ok(result)
    }
    /// This may freeze the global buffer unless the delete queue has already been closed.
    /// If the queue has been closed, this method will return `None`.
    fn maybe_freeze_global_buffer<D>(
        &mut self,
    ) -> Result<Option<FrozenBufferedUpdates<D, Q>>, LuceneError>
    where
        D: Directory,
    {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;

        if !global_state.closed {
            // Here we freeze the global buffer so we need to lock it,
            //  apply all deletes in the queue and reset the global slice
            // to let the GC prune the queue.
            let current_tail = global_state.tail.clone(); // Take the current tail and make this local
            self.freeze_global_buffer_internal(&mut global_state, current_tail)
        } else {
            debug_assert!(
                !self.any_changes_with_lock(&global_state)?,
                "We are closed but have changes"
            );
            Ok(None)
        }
    }

    fn freeze_global_buffer_internal<D>(
        &self,
        global_state: &mut RwLockWriteGuard<GlobalState<Q>>,
        current_tail: Arc<Node<Q>>,
    ) -> Result<Option<FrozenBufferedUpdates<D, Q>>, LuceneError>
    where
        D: Directory,
    {
        if !Arc::ptr_eq(&global_state.global_slice.slice_tail, &current_tail) {
            global_state.global_slice.slice_tail = current_tail;
            global_state.apply(BufferedUpdates::MAX_INT)?;
        }

        if global_state.global_buffered_updates.any() {
            let packet = FrozenBufferedUpdates::new(
                self.info_stream.clone(),
                &mut global_state.global_buffered_updates,
                None,
            )?;

            global_state.global_buffered_updates.clear()?;
            Ok(Some(packet))
        } else {
            Ok(None)
        }
    }
    pub fn new_slice(&self) -> Result<DeleteSlice<Q>, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
        Ok(DeleteSlice::new(global_state.tail.clone()))
    }
    pub fn update_slice(&self, slice: &mut DeleteSlice<Q>) -> Result<i64, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
        self.ensure_open(global_state.closed)?;
        let mut seq_no = self.get_next_sequence_number(global_state.max_seq_no);
        if !Arc::ptr_eq(&slice.slice_tail, &global_state.tail) {
            // new deletes arrived since we last checked
            slice.slice_tail = global_state.tail.clone();
            seq_no = -seq_no;
        }
        Ok(seq_no)
    }

    /// Just like updateSlice, but does not assign a sequence number.
    pub fn update_slice_no_seq_no(
        &self,
        global_state: &mut RwLockWriteGuard<GlobalState<Q>>,
    ) -> bool {
        if !Arc::ptr_eq(&global_state.global_slice.slice_tail, &global_state.tail) {
            // New deletes arrived since the last check
            global_state.global_slice.slice_tail = global_state.tail.clone();
            true
        } else {
            false
        }
    }

    fn ensure_open(&self, closed: bool) -> Result<(), LuceneError> {
        if closed {
            return Err(LuceneError::already_closed("already closed.".to_string()));
        }
        Ok(())
    }
    pub fn is_open(&self) -> Result<bool, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        Ok(!global_state.closed)
    }

    pub fn get_next_sequence_number(&self, max_seq_no: i64) -> i64 {
        let seq_no = self.next_seq_no.fetch_add(1, Ordering::SeqCst);
        debug_assert!(
            seq_no <= max_seq_no,
            "seq_no={} vs max_seq_no={}",
            seq_no,
            max_seq_no
        );
        seq_no
    }
    pub fn close(&mut self) -> Result<(), LuceneError> {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;

        if self.any_changes_with_lock(&global_state)? {
            return Err(LuceneError::illegal_state(
                "Can't close queue unless all changes are applied".to_string(),
            ));
        }
        global_state.closed = true;

        let seq_no = self.next_seq_no.load(Ordering::SeqCst);
        debug_assert!(
            seq_no <= global_state.max_seq_no,
            "maxSeqNo must be greater or equal to {} but was {}",
            seq_no,
            global_state.max_seq_no
        );
        self.next_seq_no
            .store(global_state.max_seq_no + 1, Ordering::SeqCst);
        Ok(())
    }
    #[cfg(feature = "test_only")]
    pub fn num_global_term_deletes(&self) -> Result<i32, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        Ok(global_state.global_buffered_updates.delete_terms.size())
    }
    pub fn clear(&self) -> Result<(), LuceneError> {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        global_state.global_slice.slice_head = global_state.tail.clone();
        global_state.global_slice.slice_tail = global_state.tail.clone();
        global_state.global_buffered_updates.clear()?;
        Ok(())
    }
    pub fn get_buffered_updates_terms_size(&self) -> Result<i32, LuceneError> {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;

        let current_tail = global_state.tail.clone();

        if !Arc::ptr_eq(&global_state.global_slice.slice_tail, &current_tail) {
            global_state.global_slice.slice_tail = current_tail;
            global_state.apply(BufferedUpdates::MAX_INT)?;
        }
        Ok(global_state.global_buffered_updates.delete_terms.size())
    }
    pub fn get_last_sequence_number(&self) -> i64 {
        self.next_seq_no.load(Ordering::SeqCst) - 1
    }
    /// Inserts a gap in the sequence numbers.
    /// This is used by IW during flush or commit to ensure any in-flight threads
    /// get sequence numbers inside the gap.
    pub fn skip_sequence_numbers(&self, jump: i64) {
        self.next_seq_no.fetch_add(jump, Ordering::SeqCst);
    }

    /// Returns the maximum completed sequence number for this queue.
    pub fn get_max_completed_seq_no(&self) -> i64 {
        let seq_no = self.next_seq_no.load(Ordering::SeqCst);

        if self.start_seq_no < seq_no {
            self.get_last_sequence_number()
        } else {
            // If we haven't advanced the seqNo, fall back to the previous queue
            let value = self.previous_max_seq_id;
            debug_assert!(
                value < self.start_seq_no,
                "illegal max sequence ID: {} start was: {}",
                value,
                self.start_seq_no
            );
            value
        }
    }

    /// Advances the queue to the next queue on flush. This carries over the generation
    /// to the next queue and sets the maximum sequence number based on the given `max_num_pending_ops`.
    /// This method can only be called once; subsequently, the returned queue should be used.
    ///
    /// # Arguments
    /// - `max_num_pending_ops`: The maximum number of possible concurrent operations that will execute
    ///   on this queue after it was advanced.
    ///
    /// # Returns
    /// A new `DocumentsWriterDeleteQueue` as the successor of this queue.
    pub fn advance_queue(
        &mut self,
        max_num_pending_ops: i64,
    ) -> Result<DocumentsWriterDeleteQueue<Q>, LuceneError> {
        let mut global_state = self
            .global_buffer_lock
            .write()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        if global_state.advanced {
            return Err(LuceneError::illegal_state(
                "queue was already advanced".to_string(),
            ));
        }
        global_state.advanced = true;

        let seq_no = self.get_last_sequence_number() + max_num_pending_ops + 1;

        global_state.max_seq_no = seq_no;

        // we use a static method to get this lambda since we previously introduced a memory leak since it
        // would
        // implicitly reference this.nextSeqNo which holds on to this del queue. see LUCENE-9478 for
        // reference
        let prev_max_seq_id = self.next_seq_no.load(Ordering::SeqCst) - 1;
        // Create a new queue with updated parameters
        Ok(DocumentsWriterDeleteQueue::with_params(
            self.info_stream.clone(),
            self.generation + 1,
            seq_no + 1,
            prev_max_seq_id,
        ))
    }

    /// Returns the maximum sequence number for this queue.
    /// This value will change once this queue is advanced.
    pub fn get_max_seq_no(&self) -> Result<i64, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        Ok(global_state.max_seq_no)
    }

    /// Returns `true` if the queue has been advanced.
    pub fn is_advanced(&self) -> Result<bool, LuceneError> {
        let global_state = self
            .global_buffer_lock
            .read()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        Ok(global_state.advanced)
    }
}
impl<Q> Drop for DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            // TODO:
        }
    }
}
impl<Q> Display for DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DWDP: [ generation: {} ]", self.generation)
    }
}
impl<Q> Accountable for DocumentsWriterDeleteQueue<Q>
where
    Q: Query,
{
    fn ram_bytes_used(&self) -> i64 {
        //TODO: memory calculation not implemented
        0
    }
}
struct GlobalState<Q>
where
    Q: Query,
{
    tail: Arc<Node<Q>>,
    global_slice: DeleteSlice<Q>,
    generation: i64,
    global_buffered_updates: BufferedUpdates<Q>,
    max_seq_no: i64,
    advanced: bool,
    closed: bool,
}
impl<Q> GlobalState<Q>
where
    Q: Query,
{
    fn new(tail: Arc<Node<Q>>, generation: i64) -> Self {
        Self {
            tail: tail.clone(),
            global_slice: DeleteSlice::new(tail),
            generation,
            global_buffered_updates: BufferedUpdates::new("global".to_string()),
            max_seq_no: i64::MAX,
            advanced: false,
            closed: false,
        }
    }
}
impl<Q> GlobalState<Q>
where
    Q: Query,
{
    pub fn apply(&mut self, doc_id_upto: i32) -> Result<(), LuceneError> {
        self.global_slice
            .apply(&mut self.global_buffered_updates, doc_id_upto)
    }
}

/// A delete slice for buffered updates.
pub struct DeleteSlice<Q>
where
    Q: Query,
{
    slice_head: Arc<Node<Q>>, // Head of the slice
    slice_tail: Arc<Node<Q>>, // Tail of the slice
}

impl<Q> DeleteSlice<Q>
where
    Q: Query,
{
    /// Creates a new delete slice with the head and tail pointing to the same node.
    pub fn new(current_tail: Arc<Node<Q>>) -> Self {
        Self {
            // Initially this is a 0 length slice pointing to the 'current' tail of
            // the queue. Once we update the slice we only need to assign the tail and
            // have a new slice
            slice_head: current_tail.clone(),
            slice_tail: current_tail,
        }
    }

    pub fn apply(
        &mut self,
        del: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        if Arc::ptr_eq(&self.slice_head, &self.slice_tail) {
            // 0 length slice
            return Ok(());
        }
        // When we apply a slice we take the head and get its next as our first
        //item to apply and continue until we applied the tail. If the head and
        //tail in this slice are not equal then there will be at least one more
        //non-null node in the slice!
        {
            let mut current = self.slice_head.clone();
            loop {
                let next_node_guard = current.next.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock".to_string())
                })?;
                debug_assert!(
                    next_node_guard.is_some(),
                    "slice property violated between the head on the tail must not be a null node"
                );

                next_node_guard.as_ref().unwrap().apply(del, doc_id_upto)?;
                if Arc::ptr_eq(next_node_guard.as_ref().unwrap(), &self.slice_tail) {
                    break;
                }

                let next_node = next_node_guard.as_ref().unwrap().clone();
                drop(next_node_guard);
                current = next_node;
            }
        }

        self.reset();
        Ok(())
    }
    pub fn reset(&mut self) {
        // Reset to a 0 length slice
        self.slice_head = self.slice_tail.clone();
    }
    /// Returns `true` if the given node is the slice's tail.
    pub fn is_tail(&self, node: &Arc<Node<Q>>) -> bool {
        Arc::ptr_eq(&self.slice_tail, node)
    }

    /// Returns `true` if the item of the given node matches the item in the tail.
    #[cfg(feature = "test_only")]
    pub fn is_tail_item(&self, item: &NodeEnum<Q>) -> bool {
        let node1 = NodeEnum::get_node_base(&self.slice_tail.item);
        let node2 = NodeEnum::get_node_base(item);
        debug_assert!(node1.is_some() && node2.is_some());
        if node1.as_ref().unwrap().item == node2.as_ref().unwrap().item {
            return true;
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        Arc::ptr_eq(&self.slice_head, &self.slice_tail)
    }
}

/// Represents a node in a linked list.
pub struct Node<Q>
where
    Q: Query,
{
    /// The next node in the list, or `None` if this is the last node.
    next: Mutex<Option<Arc<Node<Q>>>>,
    item: NodeEnum<Q>,
}

impl<Q> Node<Q>
where
    Q: Query,
{
    pub fn new(sub_node: NodeEnum<Q>) -> Self {
        Self {
            next: Mutex::new(None),
            item: sub_node,
        }
    }
}
impl<Q> NodeBase<Q> for Node<Q>
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        self.item.apply(buffered_deletes, doc_id_upto)
    }
}
impl<Q> Display for Node<Q>
where
    Q: Query,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.item)
    }
}
// empty node
pub struct EmptyNode;
impl Default for EmptyNode {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyNode {
    pub fn new() -> Self {
        Self {}
    }
}
impl<Q> NodeBase<Q> for EmptyNode
where
    Q: Query,
{
    fn apply(
        &self,
        _buffered_deletes: &mut BufferedUpdates<Q>,
        _doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        Ok(())
    }
}
impl Display for EmptyNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
// term node
pub struct TermNode {
    item: Term,
}
impl TermNode {
    pub fn new(term: Term) -> Self {
        Self { item: term }
    }
}
impl<Q> NodeBase<Q> for TermNode
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        buffered_deletes.add_term(&self.item, doc_id_upto)
    }
}
impl Display for TermNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "del={}", self.item)
    }
}
// query node
pub struct QueryNode<Q>
where
    Q: Query,
{
    item: Arc<Q>,
}
impl<Q> QueryNode<Q>
where
    Q: Query,
{
    pub fn new(query: Arc<Q>) -> Self {
        Self { item: query }
    }
}
impl<Q> NodeBase<Q> for QueryNode<Q>
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        buffered_deletes.add_query(self.item.clone(), doc_id_upto)
    }
}
impl<Q> Display for QueryNode<Q>
where
    Q: Query,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "del={}", self.item)
    }
}
// query node array
pub struct QueryNodeArray<Q>
where
    Q: Query,
{
    item: Vec<Arc<Q>>,
}
impl<Q> QueryNodeArray<Q>
where
    Q: Query,
{
    pub fn new(nodes: Vec<Arc<Q>>) -> Self {
        Self { item: nodes }
    }
}
impl<Q> NodeBase<Q> for QueryNodeArray<Q>
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        for query in &self.item {
            buffered_deletes.add_query(query.clone(), doc_id_upto)?;
        }
        Ok(())
    }
}
impl<Q> Display for QueryNodeArray<Q>
where
    Q: Query,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

// term node array
pub struct TermNodeArray {
    item: Vec<Term>,
}
impl TermNodeArray {
    pub fn new(nodes: Vec<Term>) -> Self {
        Self { item: nodes }
    }
}
impl<Q> NodeBase<Q> for TermNodeArray
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        for term in &self.item {
            buffered_deletes.add_term(term, doc_id_upto)?;
        }
        Ok(())
    }
}
impl Display for TermNodeArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "del={:?}", self.item)
    }
}

// doc values update node
pub struct DocValuesUpdatesNode {
    item: Vec<DocValuesUpdate>,
}
impl DocValuesUpdatesNode {
    pub fn new(nodes: Vec<DocValuesUpdate>) -> Self {
        Self { item: nodes }
    }
}
impl<Q> NodeBase<Q> for DocValuesUpdatesNode
where
    Q: Query,
{
    fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        for doc_values_update in &self.item {
            match doc_values_update.doc_values_type {
                DocValuesType::Binary => {
                    buffered_deletes.add_binary_update(doc_values_update, doc_id_upto)?;
                }
                DocValuesType::Numeric => {
                    buffered_deletes.add_numeric_update(doc_values_update, doc_id_upto)?;
                }
                _ => {
                    Err(LuceneError::illegal_argument(format!(
                        "{:?} DocValues updates not supported yet!",
                        doc_values_update.doc_values_type
                    )))?;
                }
            }
        }
        Ok(())
    }
}
impl Display for DocValuesUpdatesNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut sb = String::new();
        sb.push_str("docValuesUpdates: ");
        if !self.item.is_empty() {
            sb.push_str(&format!("term={}; updates: [", self.item[0].term));
            for update in &self.item {
                sb.push_str(&format!(
                    "{}:{},",
                    update.field,
                    update.sub_update.value_to_string()
                ));
            }
            if let Some(last_char) = sb.pop() {
                if last_char != ',' {
                    sb.push(last_char);
                }
            }
            sb.push(']');
        }
        write!(f, "{}", sb)
    }
}

pub enum NodeEnum<Q>
where
    Q: Query,
{
    EmptyNode(EmptyNode),
    TermNode(TermNode),
    QueryNode(QueryNode<Q>),
    QueryNodeArray(QueryNodeArray<Q>),
    TermNodeArray(TermNodeArray),
    DocValuesUpdatesNode(DocValuesUpdatesNode),
}
impl<Q> NodeEnum<Q>
where
    Q: Query,
{
    pub fn apply(
        &self,
        buffered_deletes: &mut BufferedUpdates<Q>,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        match self {
            NodeEnum::TermNode(node) => node.apply(buffered_deletes, doc_id_upto),
            NodeEnum::QueryNode(node) => node.apply(buffered_deletes, doc_id_upto),
            NodeEnum::QueryNodeArray(node) => node.apply(buffered_deletes, doc_id_upto),
            NodeEnum::TermNodeArray(node) => node.apply(buffered_deletes, doc_id_upto),
            NodeEnum::DocValuesUpdatesNode(node) => node.apply(buffered_deletes, doc_id_upto),
            NodeEnum::EmptyNode(node) => node.apply(buffered_deletes, doc_id_upto),
        }
    }
    #[cfg(feature = "test_only")]
    pub fn get_node_base(node: &NodeEnum<Q>) -> Option<&TermNodeArray> {
        match node {
            NodeEnum::TermNodeArray(node) => Some(node),
            _ => None,
        }
    }
}
impl<Q> Display for NodeEnum<Q>
where
    Q: Query,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeEnum::EmptyNode(node) => write!(f, "{}", node),
            NodeEnum::TermNode(node) => write!(f, "{}", node),
            NodeEnum::QueryNode(node) => write!(f, "{}", node),
            NodeEnum::QueryNodeArray(node) => write!(f, "{}", node),
            NodeEnum::TermNodeArray(node) => write!(f, "{}", node),
            NodeEnum::DocValuesUpdatesNode(node) => write!(f, "{}", node),
        }
    }
}

pub(crate) trait NodeBase<Q>
where
    Q: Query,
{
    fn apply(
        &self,
        _buffered_deletes: &mut BufferedUpdates<Q>,
        _doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        Err(LuceneError::illegal_argument(
            "sentinel item must never be applied".to_string(),
        ))
    }
    #[allow(unused)]
    fn is_delete(&self) -> bool {
        true
    }
}

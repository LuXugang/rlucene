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
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::{ReentrantMutex, RwLock};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

const REFERENCE_MANAGER_IS_CLOSED_MSG: &str = "this ReferenceManager is closed";

pub type RefreshListenerArc = Arc<dyn RefreshListener>;

/// Utility struct for safely sharing instances of a certain type across multiple threads, while
/// periodically refreshing them. This struct ensures each reference is closed only once all
/// threads have finished using it. Consult the documentation of [`ReferenceManager`]
/// implementations for their [`maybe_refresh`](Self::maybe_refresh) semantics.
///
/// `G` is the concrete type that will be [`acquire`](Self::acquire)d and
/// [`release`](Self::release)d.
///
/// @lucene.experimental
pub struct ReferenceManager<G, B>
where
  B: ReferenceManagerBase<G>,
{
  current: RwLock<Option<Arc<G>>>,
  reference_lock: ReentrantMutex<()>,
  refresh_lock: ReentrantMutex<()>,
  refresh_listeners: RwLock<Vec<RefreshListenerArc>>,
  base: B,
}

impl<G, B> ReferenceManager<G, B>
where
  B: ReferenceManagerBase<G>,
{
  pub(crate) fn new(current: G, base: B) -> Self {
    Self {
      current: RwLock::new(Some(Arc::new(current))),
      reference_lock: ReentrantMutex::new(()),
      refresh_lock: ReentrantMutex::new(()),
      refresh_listeners: RwLock::new(Vec::new()),
      base,
    }
  }

  fn ensure_open(&self) -> Result<()> {
    if self.current.read().is_none() {
      return Err(LuceneError::already_closed(
        REFERENCE_MANAGER_IS_CLOSED_MSG.to_string(),
      ));
    }
    Ok(())
  }

  fn swap_reference(&self, new_reference: Option<Arc<G>>) -> Result<()> {
    let _reference_lock = self.reference_lock.lock();
    self.ensure_open()?;
    let old_reference = {
      let mut current = self.current.write();
      let old_reference = current
        .take()
        .expect("ReferenceManager must be open after ensure_open");
      *current = new_reference;
      old_reference
    };
    self.release(old_reference)
  }

  /// Obtains the current reference. Every call to `acquire` must be matched with one call to
  /// [`release`](Self::release). It is best to release in a finally-equivalent path and stop using
  /// the reference after it has been released.
  ///
  /// # Errors
  ///
  /// Returns an already-closed error if the reference manager has been closed.
  pub fn acquire(&self) -> Result<Arc<G>> {
    loop {
      let reference =
        self.current.read().as_ref().cloned().ok_or_else(|| {
          LuceneError::already_closed(REFERENCE_MANAGER_IS_CLOSED_MSG.to_string())
        })?;
      if self.base.try_inc_ref(reference.as_ref())? {
        return Ok(reference);
      }
      if self.base.get_ref_count(reference.as_ref()) == 0
        && self
          .current
          .read()
          .as_ref()
          .is_some_and(|current| Arc::ptr_eq(current, &reference))
      {
        /* if we can't increment the reader but we are
        still the current reference the RM is in a
        illegal state since we can't make any progress
        anymore. The reference is closed but the RM still
        holds on to it as the actual instance.
        This can only happen if somebody outside of the RM
        decrements the refcount without a corresponding increment
        since the RM assigns the new reference before counting down
        the reference. */
        return Err(LuceneError::illegal_state(
          "The managed reference has already closed - this is likely a bug when the reference count is modified outside of the ReferenceManager",
        ));
      }
    }
  }

  /// Closes this [`ReferenceManager`] to prevent future acquiring. The managed resource might not
  /// be released immediately if a user is holding a previously acquired reference. The resource
  /// will be released once the last reference is released. Those references can still be used as
  /// if the manager were active.
  ///
  /// Calling this method more than once has no effect.
  ///
  /// # Errors
  ///
  /// Returns an error if the current reference could not be closed.
  pub fn close(&self) -> Result<()> {
    let _reference_lock = self.reference_lock.lock();
    if self.current.read().is_some() {
      // Make sure we can call this more than once. Closeable's contract says that invoking close
      // on an already-closed resource has no effect.
      self.swap_reference(None)?;
      self.base.after_close()?;
    }
    Ok(())
  }

  fn do_maybe_refresh(&self) -> Result<()> {
    // It is okay to call lock() here because callers have already obtained refresh_lock. This
    // protects against accidentally calling this method outside the lock's scope. ReentrantMutex,
    // like Java's ReentrantLock, permits the same thread to lock more than once as long as it
    // unlocks the same number of times.
    let _refresh_lock = self.refresh_lock.lock();
    let reference = self.acquire()?;
    let mut refreshed = false;
    let refresh_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      self.notify_refresh_listeners_before()?;
      if let Some(new_reference) = self.base.refresh_if_needed(reference.as_ref())? {
        let new_reference = Arc::new(new_reference);
        debug_assert!(
          !Arc::ptr_eq(&new_reference, &reference),
          "refresh_if_needed should return None if refresh wasn't needed"
        );
        let swap_result = catch_unwind(AssertUnwindSafe(|| {
          self.swap_reference(Some(new_reference.clone()))
        }));
        if !matches!(&swap_result, Ok(Ok(()))) {
          self.release(new_reference)?;
        }
        unwrap_caught_result!(swap_result)?;
        refreshed = true;
      }
      Ok(())
    }));

    let finally_result = (|| -> Result<()> {
      self.release(reference)?;
      self.notify_refresh_listeners_refreshed(refreshed)
    })();
    finally_result?;

    match refresh_result {
      Ok(result) => result?,
      Err(payload) => resume_unwind(payload),
    }
    self.base.after_maybe_refresh()
  }

  /// Call this, or [`maybe_refresh_blocking`](Self::maybe_refresh_blocking), periodically if
  /// [`acquire`](Self::acquire) should return refreshed instances.
  ///
  /// It is safe for more than one thread to call this at once. Only the first thread attempts the
  /// refresh; subsequent threads return immediately. A return value of `true` means the calling
  /// thread either refreshed or there were no changes to refresh. A return value of `false` means
  /// another thread is currently refreshing.
  ///
  /// # Errors
  ///
  /// Returns an error if refreshing the resource fails or the reference manager has been closed.
  pub fn maybe_refresh(&self) -> Result<bool> {
    self.ensure_open()?;

    // Ensure only one thread refreshes at once; other threads return immediately.
    let Some(_refresh_lock) = self.refresh_lock.try_lock() else {
      return Ok(false);
    };
    self.do_maybe_refresh()?;
    Ok(true)
  }

  /// Call this, or [`maybe_refresh`](Self::maybe_refresh), periodically if
  /// [`acquire`](Self::acquire) should return refreshed instances.
  ///
  /// Unlike [`maybe_refresh`](Self::maybe_refresh), this method blocks until another thread's
  /// refresh completes. It is useful when the next call to [`acquire`](Self::acquire) must return a
  /// refreshed instance.
  ///
  /// # Errors
  ///
  /// Returns an error if refreshing the resource fails or the reference manager has been closed.
  pub fn maybe_refresh_blocking(&self) -> Result<()> {
    self.ensure_open()?;

    // Ensure only one thread refreshes at once.
    let _refresh_lock = self.refresh_lock.lock();
    self.do_maybe_refresh()
  }

  /// Releases a reference previously obtained via [`acquire`](Self::acquire).
  ///
  /// It is safe to call this after [`close`](Self::close).
  ///
  /// # Errors
  ///
  /// Returns an error if releasing the resource fails.
  pub fn release(&self, reference: Arc<G>) -> Result<()> {
    self.base.dec_ref(reference.as_ref())
  }

  fn notify_refresh_listeners_before(&self) -> Result<()> {
    let refresh_listeners = self.refresh_listeners.read().clone();
    for refresh_listener in refresh_listeners {
      refresh_listener.before_refresh()?;
    }
    Ok(())
  }

  fn notify_refresh_listeners_refreshed(&self, did_refresh: bool) -> Result<()> {
    let refresh_listeners = self.refresh_listeners.read().clone();
    for refresh_listener in refresh_listeners {
      refresh_listener.after_refresh(did_refresh)?;
    }
    Ok(())
  }

  /// Adds a listener to be notified when a reference is refreshed or swapped.
  pub fn add_listener(&self, listener: RefreshListenerArc) {
    self.refresh_listeners.write().push(listener);
  }

  /// Removes a listener added with [`add_listener`](Self::add_listener).
  pub fn remove_listener(&self, listener: &RefreshListenerArc) {
    let mut refresh_listeners = self.refresh_listeners.write();
    if let Some(index) = refresh_listeners
      .iter()
      .position(|current| Arc::ptr_eq(current, listener))
    {
      refresh_listeners.remove(index);
    }
  }
}

impl<G, B> Closeable for ReferenceManager<G, B>
where
  B: ReferenceManagerBase<G>,
{
  fn close(&mut self) -> Result<()> {
    ReferenceManager::close(self)
  }
}

impl<G, B> CloseableRef for ReferenceManager<G, B>
where
  B: ReferenceManagerBase<G>,
{
  fn close(&self) -> Result<()> {
    ReferenceManager::close(self)
  }
}

/// Operations supplied by a concrete [`ReferenceManager`] implementation for its managed
/// reference type.
pub trait ReferenceManagerBase<G> {
  /// Decrements reference counting on the given reference.
  ///
  /// # Errors
  ///
  /// Returns an error if reference decrement on the given resource failed.
  fn dec_ref(&self, reference: &G) -> Result<()>;

  /// Refreshes the given reference if needed. Returns `None` if no refresh was needed, otherwise
  /// a new refreshed reference.
  ///
  /// # Errors
  ///
  /// Returns an already-closed error if the reference manager has been closed, or an I/O error if
  /// the refresh operation failed.
  fn refresh_if_needed(&self, reference_to_refresh: &G) -> Result<Option<G>>;

  /// Tries to increment reference counting on the given reference. Returns `true` if the operation
  /// was successful.
  ///
  /// # Errors
  ///
  /// Returns an already-closed error if the reference manager has been closed.
  fn try_inc_ref(&self, reference: &G) -> Result<bool>;

  /// Returns the current reference count of the given reference.
  fn get_ref_count(&self, reference: &G) -> i32;

  /// Called after `close()`, so an implementation can free any resources.
  ///
  /// # Errors
  ///
  /// Returns an error if the after-close operation in an implementation fails.
  fn after_close(&self) -> Result<()> {
    Ok(())
  }

  /// Called after a refresh was attempted, regardless of whether a new reference was in fact
  /// created.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  fn after_maybe_refresh(&self) -> Result<()> {
    Ok(())
  }
}

/// Use to receive notification when a refresh has finished. See
/// [`ReferenceManager::add_listener`].
pub trait RefreshListener: Send + Sync {
  /// Called right before a refresh attempt starts.
  fn before_refresh(&self) -> Result<()>;

  /// Called after the attempted refresh. If the refresh opened a new reference,
  /// `did_refresh` is `true` and [`ReferenceManager::acquire`] is guaranteed to return the new
  /// reference.
  fn after_refresh(&self, did_refresh: bool) -> Result<()>;
}

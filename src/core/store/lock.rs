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
use crate::core::store::NativeFSLock;
use crate::core::store::no_lock_factory::NoLock;
use crate::core::store::simple_fs_lock_factory::SimpleFSLock;
use crate::core::store::single_instance_lock_factory::SingleInstanceLock;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// An interprocess mutex lock.
///
/// # Example
/// Typical use might look like:
///
/// ```text
/// let lock = directory.obtain_lock("my.lock")?;
/// // ... code to execute while locked ...
/// ```
///
/// # See Also
/// [`Directory::obtain_lock`](crate::core::store::directory::Directory::obtain_lock)
///
/// # Note
/// This is an internal API.
pub trait Lock: Display + CloseableRef {
  /// Best effort check that this lock is still valid. Locks could become
  /// invalidated externally for a number of reasons, such as if a user
  /// deletes the lock file manually or when a network filesystem is in
  /// use.
  ///
  /// # Errors
  /// Returns an `LuceneError` if the lock is no longer valid.
  fn ensure_valid(&self) -> Result<()>;
}

pub type DynLock = dyn Lock + Send + Sync;
pub type CustomLock = Box<DynLock>;
pub enum LockEnum {
  Single(SingleInstanceLock),
  Simple(SimpleFSLock),
  Native(NativeFSLock),
  Custom(CustomLock),
  NoLock(NoLock),
}
impl LockEnum {
  pub fn custom<L>(lock: L) -> Self
  where
    L: Lock + Send + Sync + 'static,
  {
    Self::Custom(Box::new(lock))
  }
}

impl Display for LockEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Single(inner) => inner.fmt(f),
      Self::Simple(inner) => inner.fmt(f),
      Self::Native(inner) => inner.fmt(f),
      Self::Custom(inner) => inner.fmt(f),
      Self::NoLock(inner) => inner.fmt(f),
    }
  }
}

impl CloseableRef for LockEnum {
  fn close(&self) -> Result<()> {
    match self {
      Self::Single(inner) => inner.close(),
      Self::Simple(inner) => inner.close(),
      Self::Native(inner) => inner.close(),
      Self::Custom(inner) => inner.close(),
      Self::NoLock(inner) => inner.close(),
    }
  }
}

impl Lock for LockEnum {
  fn ensure_valid(&self) -> Result<()> {
    match self {
      Self::Single(inner) => inner.ensure_valid(),
      Self::Simple(inner) => inner.ensure_valid(),
      Self::Native(inner) => inner.ensure_valid(),
      Self::Custom(inner) => inner.ensure_valid(),
      Self::NoLock(inner) => inner.ensure_valid(),
    }
  }
}

macro_rules! either_lock {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Display for $name<$( $T ),+>
        where
            $( $T: Lock ),+
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$Variant(inner) => inner.fmt(f), )+
                }
            }
        }

        impl<$( $T ),+> CloseableRef for $name<$( $T ),+>
        where
            $( $T: Lock ),+
        {
            fn close(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.close(), )+
                }
            }
        }

        impl<$( $T ),+> Lock for $name<$( $T ),+>
        where
            $( $T: Lock ),+
        {
            fn ensure_valid(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.ensure_valid(), )+
                }
            }
        }
    };
}
either_lock!(pub LockEnum2 { A: A, B: B });
either_lock!(pub LockEnum3 { A: A, B: B, C: C });

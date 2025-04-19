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
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{Comparator, ToInt};
use crate::with_other;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
/// A generic, slice-like reference over an integer array with offset and length.
///
/// `IntsRef<AV>` provides a flexible abstraction for referencing a sub-slice of integers,
/// where `AV` is a container implementing [`AccessVec<i32>`].
///
/// This design supports different memory access models:
///
/// - **Single-threaded, shared ownership:**  
///   Use `Rc<RefCell<Vec<i32>>>` as the `AV` type. This allows multiple parts of the program
///   to mutate the same underlying data safely in a single-threaded context.
///
/// - **Multi-threaded, synchronized access:**  
///   Use `Arc<Mutex<Vec<i32>>>` for safe concurrent access and mutation across threads.
///
/// - **No sharing / performance-focused:**  
///   Use plain `Vec<i32>` if the data is owned locally and doesn’t need to be shared.
///   This offers the best performance with no synchronization overhead.
///
/// The generic `AccessVec` trait provides a unified interface for all three modes,
/// abstracting over access, mutation, cloning, and construction.
/// Represents int[], as a slice (offset + length) into an existing int[].
#[derive(Debug)]
pub struct IntsRef<AV: AccessVec<i32>> {
    /// The contents of the IntsRef
    pub ints: AV,
    /// Offset of first valid integer.
    pub offset: i32,
    /// Length of used ints.
    pub length: i32,
}
impl<AV> Default for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<AV> IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    /// Create an IntsRef with `EMPTY_INTS`.
    pub fn new() -> Self {
        IntsRef {
            ints: AV::new(),
            offset: 0,
            length: 0,
        }
    }

    /// Create an IntsRef pointing to a new array of size `capacity`.
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        Ok(IntsRef {
            ints: AV::with_capacity(capacity),
            offset: 0,
            length: 0,
        })
    }
    pub fn from_slice(ints: AV, offset: i32, length: i32) -> Self {
        let instance = IntsRef {
            ints,
            offset,
            length,
        };
        debug_assert!(instance.is_valid().unwrap());
        instance
    }
    /// Performs internal consistency checks. Always returns true (or Error)
    pub fn is_valid(&self) -> Result<bool> {
        self.ints.access(|ints| {
            if self.length < 0 {
                return Err(LuceneError::illegal_state(format!(
                    "length is negative {}",
                    self.length
                )));
            }
            if (self.length as usize) > ints.len() {
                return Err(LuceneError::illegal_state(format!(
                    "length is out of bounds: {}, ints.len()={}",
                    self.length,
                    ints.len()
                )));
            }
            if self.offset < 0 {
                return Err(LuceneError::illegal_state(format!(
                    "offset is negative {}",
                    self.offset
                )));
            }
            if (self.offset as usize) > ints.len() {
                return Err(LuceneError::illegal_state(format!(
                    "offset out of bounds: {}, ints.len()={}",
                    self.offset,
                    ints.len()
                )));
            }
            if self.offset + self.length < 0 {
                return Err(LuceneError::illegal_state(format!(
                    "offset+length is negative: offset={}, length={}",
                    self.offset, self.length
                )));
            }
            if ((self.offset + self.length) as usize) > ints.len() {
                return Err(LuceneError::illegal_state(format!(
                    "offset+length out of bounds: offset={}, length={}, ints.len()={}",
                    self.offset,
                    self.length,
                    ints.len()
                )));
            }
            Ok(true)
        })
    }
    pub fn ints_equals(&self, other: &IntsRef<AV>) -> Result<bool> {
        with_other!(self.ints, other.ints, |ints_bytes, other_bytes| {
            let self_slice =
                &ints_bytes[self.offset as usize..(self.offset + self.length) as usize];
            let other_slice =
                &other_bytes[other.offset as usize..(other.offset + other.length) as usize];
            Ok(self_slice == other_slice)
        })
    }
    /// Creates a new IntsRef that points to a copy of the ints from `other`
    ///
    /// The returned IntsRef will have a length of `other.length` and an offset of zero.
    pub fn deep_copy_of(other: &IntsRef<AV>) -> Result<Self> {
        let ints = other
            .ints
            .slice_clone(other.offset as usize, other.length as usize)?;
        Ok(IntsRef {
            ints,
            offset: 0,
            length: other.length,
        })
    }
}
impl<AV> Clone for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    /// Returns a shallow clone of this instance (the underlying ints are **not** copied
    /// and will be shared by both the returned object and this object).
    fn clone(&self) -> Self {
        IntsRef {
            ints: self.ints.clone(),
            offset: self.offset,
            length: self.length,
        }
    }
}
impl<AV> PartialOrd for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<AV> Ord for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        with_other!(self.ints, other.ints, |ints_bytes, other_bytes| {
            let self_slice =
                &ints_bytes[self.offset as usize..(self.offset + self.length) as usize];
            let other_slice =
                &other_bytes[other.offset as usize..(other.offset + other.length) as usize];
            Ok(self_slice.cmp(other_slice))
        })
        .expect("IntsRef Ord#cmp failed")
    }
}
impl<AV> PartialEq for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn eq(&self, other: &Self) -> bool {
        self.ints_equals(other)
            .expect("IntsRef PartialEq#eq failed")
    }
}
impl<AV> Eq for IntsRef<AV> where AV: AccessVec<i32> {}
impl<AV> Hash for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ints
            .access(|ints| {
                let slice = &ints[self.offset as usize..(self.offset + self.length) as usize];
                slice.hash(state);
                Ok(())
            })
            .expect("IntsRef Hash failed");
    }
}
impl<AV> Comparator<IntsRef<AV>> for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    const TYPE: &'static str = "IntsRefComparator";

    fn compare(&self, a: &IntsRef<AV>, b: &IntsRef<AV>) -> Result<i32> {
        with_other!(a.ints, b.ints, |a_bytes, b_bytes| {
            let a_slice = &a_bytes[a.offset as usize..(a.offset + a.length) as usize];
            let b_slice = &b_bytes[a.offset as usize..(a.offset + a.length) as usize];
            Ok(a_slice.cmp(b_slice).to_int())
        })
    }
}
impl<AV> Display for IntsRef<AV>
where
    AV: AccessVec<i32>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.ints
            .access(|ints| {
                let slice = &ints[self.offset as usize..(self.offset + self.length) as usize];
                write!(f, "[")?;
                for (i, v) in slice.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")?;
                Ok(())
            })
            .map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::ints_ref::IntsRef;

    #[allow(dead_code)] // for quick search
    struct TestIntsRef;
    #[test]
    fn test_empty() {
        let i: IntsRef<Vec<i32>> = IntsRef::default();
        assert!(i.ints.is_empty());
        assert_eq!(0, i.offset);
        assert_eq!(0, i.length);
    }

    #[test]
    fn test_from_ints() {
        let ints = vec![1, 2, 3, 4];
        let rc_ints = ints.clone();
        let i = IntsRef::from_slice(rc_ints.clone(), 0, 4);
        assert_eq!(ints, *i.ints);
        assert_eq!(0, i.offset);
        assert_eq!(4, i.length);

        let i2 = IntsRef::from_slice(rc_ints.clone(), 1, 3);
        let expected = IntsRef::from_slice(vec![2, 3, 4], 0, 3);
        assert_eq!(expected, i2);
        assert_ne!(i, i2);
    }

    #[test]
    #[should_panic]
    fn test_invalid_deep_copy() {
        let rc_ints = vec![1, 2];
        let mut from = IntsRef::from_slice(rc_ints, 0, 2);
        from.offset += 1; // now invalid
        let _ = IntsRef::deep_copy_of(&from);
    }
}

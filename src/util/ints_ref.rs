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
use crate::util::error::lucene_error::LuceneError;
use crate::util::Comparator;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
/// Represents int[], as a slice (offset + length) into an existing int[].
#[derive(Debug)]
pub struct IntsRef {
    /// The contents of the IntsRef
    pub ints: Rc<RefCell<Vec<i32>>>,
    /// Offset of first valid integer.
    pub offset: i32,
    /// Length of used ints.
    pub length: i32,
}
impl Default for IntsRef {
    fn default() -> Self {
        Self::new()
    }
}

impl IntsRef {
    /// Create an IntsRef with `EMPTY_INTS`.
    pub fn new() -> Self {
        IntsRef {
            ints: Rc::new(RefCell::new(Vec::new())),
            offset: 0,
            length: 0,
        }
    }

    /// Create an IntsRef pointing to a new array of size `capacity`. Offset and length will
    /// both be zero.
    pub fn with_capacity(capacity: i32) -> Self {
        IntsRef {
            ints: Rc::new(RefCell::new(vec![0; capacity as usize])),
            offset: 0,
            length: 0,
        }
    }

    /// This instance will directly reference ints w/o making a copy. ints should not be null.
    pub fn from_ints(ints: Rc<RefCell<Vec<i32>>>, offset: i32, length: i32) -> Self {
        let instance = IntsRef {
            ints,
            offset,
            length,
        };
        debug_assert!({ instance.is_valid().unwrap() });
        instance
    }
    /// Performs internal consistency checks. Always returns true (or Error)
    pub fn is_valid(&self) -> Result<bool, LuceneError> {
        let ints_ref = self.ints.borrow();
        if self.length < 0 {
            return Err(LuceneError::illegal_state(format!(
                "length is negative {}",
                self.length
            )));
        }
        if (self.length as usize) > ints_ref.len() {
            return Err(LuceneError::illegal_state(format!(
                "length is out of bounds: {}, ints.len()={}",
                self.length,
                ints_ref.len()
            )));
        }
        if self.offset < 0 {
            return Err(LuceneError::illegal_state(format!(
                "offset is negative {}",
                self.offset
            )));
        }
        if (self.offset as usize) > ints_ref.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset out of bounds: {}, ints.len()={}",
                self.offset,
                ints_ref.len()
            )));
        }
        if self.offset + self.length < 0 {
            return Err(LuceneError::illegal_state(format!(
                "offset+length is negative: offset={}, length={}",
                self.offset, self.length
            )));
        }
        if ((self.offset + self.length) as usize) > ints_ref.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset+length out of bounds: offset={}, length={}, ints.len()={}",
                self.offset,
                self.length,
                ints_ref.len()
            )));
        }
        Ok(true)
    }
    pub fn ints_equals(&self, other: &IntsRef) -> bool {
        let self_ints = self.ints.borrow();
        let other_ints = other.ints.borrow();
        let self_slice = &self_ints[self.offset as usize..(self.offset + self.length) as usize];
        let other_slice =
            &other_ints[other.offset as usize..(other.offset + other.length) as usize];
        self_slice == other_slice
    }
    /// Creates a new IntsRef that points to a copy of the ints from `other`
    ///
    /// The returned IntsRef will have a length of `other.length` and an offset of zero.
    pub fn deep_copy_of(other: &IntsRef) -> Self {
        let other_ints = other.ints.borrow();
        let start = other.offset as usize;
        let end = (other.offset + other.length) as usize;
        let new_vec = other_ints[start..end].to_vec();
        IntsRef {
            ints: Rc::new(RefCell::new(new_vec)),
            offset: 0,
            length: other.length,
        }
    }
}
impl Clone for IntsRef {
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
impl PartialEq for IntsRef {
    fn eq(&self, other: &Self) -> bool {
        self.ints_equals(other)
    }
}
impl Eq for IntsRef {}
impl Hash for IntsRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ints = self.ints.borrow();
        let slice = &ints[self.offset as usize..(self.offset + self.length) as usize];
        slice.hash(state);
    }
}
impl Comparator<IntsRef> for IntsRef {
    const TYPE: &'static str = "COMPARATOR_TYPE";

    fn compare(&self, a: &IntsRef, b: &IntsRef) -> i32 {
        let a_ints = a.ints.borrow();
        let b_ints = b.ints.borrow();
        let a_slice = &a_ints[a.offset as usize..(a.offset + a.length) as usize];
        let b_slice = &b_ints[a.offset as usize..(a.offset + a.length) as usize];
        match a_slice.cmp(b_slice) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 1,
            std::cmp::Ordering::Greater => 1,
        }
    }
}
impl Display for IntsRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ints = self.ints.borrow();
        write!(f, "[")?;
        let start = self.offset as usize;
        let end = (self.offset + self.length) as usize;
        for (i, value) in ints[start..end].iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:x}", value)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use crate::util::ints_ref::IntsRef;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[allow(dead_code)] // for quick search
    struct TestIntsRef;
    #[test]
    fn test_empty() {
        let i = IntsRef::new();
        assert!(i.ints.borrow().is_empty());
        assert_eq!(0, i.offset);
        assert_eq!(0, i.length);
    }

    #[test]
    fn test_from_ints() {
        let ints = vec![1, 2, 3, 4];
        let rc_ints = Rc::new(RefCell::new(ints.clone()));
        let i = IntsRef::from_ints(rc_ints.clone(), 0, 4);
        assert_eq!(ints, *i.ints.borrow());
        assert_eq!(0, i.offset);
        assert_eq!(4, i.length);

        let i2 = IntsRef::from_ints(rc_ints.clone(), 1, 3);
        let expected = IntsRef::from_ints(Rc::new(RefCell::new(vec![2, 3, 4])), 0, 3);
        assert_eq!(expected, i2);
        assert_ne!(i, i2);
    }

    #[test]
    #[should_panic]
    fn test_invalid_deep_copy() {
        let rc_ints = Rc::new(RefCell::new(vec![1, 2]));
        let mut from = IntsRef::from_ints(rc_ints, 0, 2);
        from.offset += 1; // now invalid
        let _ = IntsRef::deep_copy_of(&from);
    }
}

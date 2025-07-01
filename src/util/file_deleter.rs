/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
/// This struct provides ability to track the reference counts of a set of index files and delete them
/// when their counts decreased to 0.
///
/// This struct is NOT thread-safe, the user should make sure the thread-safety themselves
pub struct FileDeleter<D, M>
where
    D: Directory,
    M: Messenger,
{
    ref_counts: HashMap<String, RefCount>,
    directory: Arc<Mutex<D>>,
    ///  user specified message consumer, first argument will be message type second argument will be the actual message
    messenger: Option<M>,
}
impl<D, M> FileDeleter<D, M>
where
    D: Directory,
    M: Messenger,
{
    fn new(directory: Arc<Mutex<D>>, messenger: Option<M>) -> FileDeleter<D, M> {
        FileDeleter {
            ref_counts: HashMap::new(),
            directory,
            messenger,
        }
    }
    pub fn inc_ref<I, S>(&mut self, file_names: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for file in file_names {
            self.inc_ref_single(file.as_ref());
        }
    }

    pub fn inc_ref_single(&mut self, file_name: &str) {
        let rc = self.get_ref_count_internal(file_name);
        let count = rc.count;
        rc.inc_ref();

        if let Some(messenger) = &mut self.messenger {
            messenger.accept(
                MsgType::Ref,
                format!("IncRef \"{file_name}\": pre-incr count is {count}"),
            );
        }
    }

    /// Decrease ref counts for all provided files, delete them if ref counts down to 0, even on
    /// error. Throw first exception hit, if any
    pub fn dec_ref<I>(&mut self, file_names: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        let mut to_delete = Vec::new();

        for file_name in file_names {
            if self.dec_ref_single(&file_name) {
                to_delete.push(file_name)
            }
        }
        self.delete_files(to_delete)
    }
    /// Returns true if the file should be deleted
    fn dec_ref_single(&mut self, file_name: &str) -> bool {
        let rc = self.get_ref_count_internal(file_name);
        let count = rc.count;
        let v = if rc.dec_ref() == 0 {
            self.ref_counts.remove(file_name);
            true
        } else {
            false
        };
        if let Some(ref mut messenger) = self.messenger {
            messenger.accept(
                MsgType::Ref,
                format!("DecRef \"{file_name}\": pre-decr count is {count}"),
            );
        }
        v
    }
    fn get_ref_count_internal(&mut self, file_name: &str) -> &mut RefCount {
        if self.ref_counts.contains_key(file_name) {
            return self.ref_counts.get_mut(file_name).unwrap();
        }
        self.ref_counts
            .entry(file_name.to_string())
            .or_insert_with(|| RefCount::new(file_name))
    }
    /// If the file is not yet recorded, this method will create a new RefCount object with count 0
    pub fn init_ref_count(&mut self, file_name: &str) {
        if !self.ref_counts.contains_key(file_name) {
            self.ref_counts
                .insert(file_name.to_string(), RefCount::new(file_name));
        }
    }
    /// Get ref count for a provided file. If the file is not yet recorded, returns 0
    pub fn get_ref_count(&self, file_name: &str) -> usize {
        self.ref_counts
            .get(file_name)
            .map(|rc| rc.count)
            .unwrap_or(0)
    }
    /// Get all files, some of them may have ref count 0
    pub fn get_all_files(&self) -> impl Iterator<Item = &String> {
        self.ref_counts.keys()
    }

    pub fn exists(&self, file_name: &str) -> bool {
        self.ref_counts
            .get(file_name)
            .map(|rc| rc.count > 0)
            .unwrap_or(false)
    }
    /// get files that are touched but not incref'ed
    pub fn get_unrefed_files(&self) -> HashSet<String> {
        let mut unrefed = HashSet::new();
        for (file_name, rc) in &self.ref_counts {
            if rc.count == 0 {
                if let Some(messenger) = &self.messenger {
                    messenger.accept(
                        MsgType::File,
                        format!("removing unreferenced file \"{file_name}\""),
                    );
                }
                unrefed.insert(file_name.clone());
            }
        }
        unrefed
    }
    /// delete only files that are unref'ed
    pub fn delete_files_if_no_ref<I>(&mut self, files: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        let mut to_delete = HashSet::new();

        for file_name in files {
            // NOTE: it's very unusual yet possible for the
            // refCount to be present and 0: it can happen if you
            // open IW on a crashed index, and it removes a bunch
            // of unref'd files, and then you add new docs / do
            // merging, and it reuses that segment name.
            // TestCrash.testCrashAfterReopen can hit this:
            if !self.exists(&file_name) {
                if let Some(messenger) = &self.messenger {
                    messenger.accept(
                        MsgType::File,
                        format!("will delete new file \"{file_name}\""),
                    );
                }
                to_delete.insert(file_name);
            }
        }

        self.delete_files(to_delete)
    }

    pub fn force_delete(&mut self, file_name: &str) -> Result<()> {
        self.ref_counts.remove(file_name);
        self.delete_file(file_name)
    }

    pub fn delete_file_if_no_ref(&mut self, file_name: &str) -> Result<()> {
        if !self.exists(file_name) {
            if let Some(messenger) = &self.messenger {
                messenger.accept(
                    MsgType::File,
                    format!("will delete new file \"{file_name}\""),
                );
            }
            self.delete_file(file_name)?;
        }
        Ok(())
    }

    pub fn delete_files(&self, file_names: impl IntoIterator<Item = String>) -> Result<()> {
        let files: Vec<String> = file_names.into_iter().collect();

        if let Some(messenger) = &self.messenger {
            messenger.accept(
                MsgType::File,
                format!("now delete {} files: {:?}", files.len(), files),
            );
        }

        // First pass: delete any segments_N files.  We do these first to be certain stale commit points
        // are removed
        // before we remove any files they reference, in case we crash right now:
        for file_name in files
            .iter()
            .filter(|f| f.starts_with(IndexFileNames::SEGMENTS))
        {
            debug_assert!(!self.exists(file_name));
            self.delete_file(file_name)?;
        }

        // Only delete other files if we were able to remove the segments_N files; this way we never
        // leave a corrupt commit in the index even in the presense of virus checkers:
        for file_name in files
            .iter()
            .filter(|f| !f.starts_with(IndexFileNames::SEGMENTS))
        {
            debug_assert!(!self.exists(file_name));
            self.delete_file(file_name)?;
        }

        Ok(())
    }

    fn delete_file(&self, file_name: &str) -> Result<()> {
        match self.directory.lock().delete_file(file_name) {
            Ok(_) => Ok(()),
            Err(e) => {
                if cfg!(target_os = "windows") {
                    if matches!(
                        e,
                        LuceneError::Io(ref io_err)
                            if io_err.kind() == std::io::ErrorKind::NotFound
                    ) {
                        // TODO: can we remove this OS-specific hacky logic?  If windows deleteFile is buggy, we
                        // should instead contain this workaround in
                        // a WindowsFSDirectory ...
                        // LUCENE-6684: we suppress this assert for Windows, since a file could be in a confusing
                        // "pending delete" state, where we already
                        // deleted it once, yet it still shows up in directory listings, and if you try to delete it
                        // again you'll hit NSFE/FNFE:
                        Ok(())
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            },
        }
    }
}

/// Tracks the reference count for a single index file:
pub struct RefCount {
    // fileName used only for better assert error messages
    file_name: String,
    init_done: bool,
    count: usize,
}
impl RefCount {
    pub fn new(file_name: &str) -> Self {
        Self {
            file_name: file_name.to_string(),
            init_done: false,
            count: 0,
        }
    }

    pub fn inc_ref(&mut self) -> usize {
        if !self.init_done {
            self.init_done = true;
        } else {
            debug_assert!(
                self.count > 0,
                "{}: RefCount is 0 pre-increment for file `{}`",
                std::thread::current()
                    .name()
                    .unwrap_or("Thread name is None"),
                self.file_name
            );
        }
        self.count.saturating_add(1)
    }

    pub fn dec_ref(&mut self) -> usize {
        debug_assert!(
            self.count > 0,
            "{}: RefCount is 0 pre-increment for file `{}`",
            std::thread::current()
                .name()
                .unwrap_or("Thread name is None"),
            self.file_name
        );
        self.count.saturating_sub(1)
    }
}

pub trait Messenger {
    fn accept(&self, msg_type: MsgType, message: String);
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgType {
    Ref,
    File,
}

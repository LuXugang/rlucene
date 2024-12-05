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
use regex::Regex;

/// This class contains useful constants representing filenames and extensions used by lucene, as
/// well as convenience methods for querying whether a file name matches an extension
/// `#matchesExtension(&str, &str) matchesExtension`, as well as generating file names from a
/// segment name, generation and extension ( `#fileNameFromGeneration(&str, &str, u64) fileNameFromGeneration`
///`#segmentFileName(&str, &str, &str) segmentFileName`.
///
/// NOTE>: extensions used by codecs are not listed here. You must interact with the `Codec` directly.
pub struct IndexFileNames;

impl IndexFileNames {
    /// Computes the full file name from base, extension and generation. If the generation is -1, the
    /// file name is null. If it's 0, the file name is `<base>.<ext>`. If it's > 0, the
    /// file name is `<base>_<gen>.<ext>
    /// NOTE: `<ext>` is added to the name only if `ext` is not an empty string.
    pub fn file_name_from_generation(base: &str, ext: &str, gen: u64) -> String {
        if gen == 0 {
            IndexFileNames::segment_file_name(base, "", ext)
        } else {
            // base-36
            let gen_str = format!("{:x}", gen);
            let mut res = String::with_capacity(base.len() + 6 + ext.len());
            res.push_str(base);
            res.push('_');
            res.push_str(&gen_str);

            if !ext.is_empty() {
                res.push('.');
                res.push_str(ext);
            }
            res
        }
    }

    ///Returns a file name that includes the given segment name, your own custom name and extension.
    ///The format of the filename is: `<segmentName>(_<name>)(.<ext>)`.
    ///
    /// <p><b>NOTE:</b> .&lt;ext&gt; is added to the result file name only if <code>ext</code> is not
    /// empty.
    ///
    /// <p><b>NOTE:</b> _&lt;segmentSuffix&gt; is added to the result file name only if it's not the
    /// empty string
    ///
    /// <p><b>NOTE:</b> all custom files should be named using this method, or otherwise some
    /// structures may fail to handle them properly (such as if they are added to compound files).
    pub fn segment_file_name(segment_name: &str, segment_suffix: &str, ext: &str) -> String {
        if !ext.is_empty() || !segment_suffix.is_empty() {
            debug_assert!(!ext.starts_with('.'), "Extension should not start with '.'");
            let mut sb =
                String::with_capacity(segment_name.len() + 2 + segment_suffix.len() + ext.len());
            sb.push_str(segment_name);
            if !segment_suffix.is_empty() {
                sb.push('_');
                sb.push_str(segment_suffix);
            }
            if !ext.is_empty() {
                sb.push('.');
                sb.push_str(ext);
            }
            sb
        } else {
            segment_name.to_string()
        }
    }

    /// Returns true if the given filename ends with the given extension. One should provide a
    /// `pure` extension, without '.'.
    pub fn matches_extension(filename: &str, ext: &str) -> bool {
        // It doesn't make a difference whether we allocate a StringBuilder ourself
        // or not, since there's only 1 '+' operator.
        filename.ends_with(&format!(".{}", ext))
    }
    /// locates the boundary of the segment name, or -1 */
    pub fn index_of_segment_name(filename: &str) -> i32 {
        debug_assert!(filename.len() <= i32::MAX as usize);
        if let Some(idx) = filename[1..].find('_') {
            (idx + 1) as i32
        } else if let Some(idx) = filename.find('.') {
            idx as i32
        } else {
            -1
        }
    }

    /// Strips the segment name out of the given file name. If you used `segmentFileName` or
    /// `fileNameFromGeneration` to create your files, then this method simply removes whatever
    /// comes before the first '.', or the second '_' (excluding both).
    pub fn strip_segment_name(filename: &str) -> &str {
        let idx = IndexFileNames::index_of_segment_name(filename);
        if idx != -1 {
            &filename[idx as usize..]
        } else {
            filename
        }
    }

    /// Returns the generation from this file name, or 0 if there is no generation.
    pub fn parse_generation(filename: &str) -> u64 {
        debug_assert!(filename.starts_with('_'), "Filename must start with '_'");

        let stripped = IndexFileNames::strip_extension(filename);
        let parts: Vec<&str> = stripped[1..].split('_').collect();
        // 4 cases:
        // segment.ext
        // segment_gen.ext
        // segment_codec_suffix.ext
        // segment_gen_codec_suffix.ext
        if parts.len() == 2 || parts.len() == 4 {
            // base-36
            u64::from_str_radix(parts[1], 36).unwrap_or(0)
        } else {
            0
        }
    }
    /// Parses the segment name out of the given file name.
    /// @return the segment name only, or filename if it does not contain a '.' and '_'.
    pub fn parse_segment_name(filename: &str) -> &str {
        let idx = IndexFileNames::index_of_segment_name(filename);
        if idx != -1 {
            &filename[..idx as usize]
        } else {
            filename
        }
    }
    /// Removes the extension (anything after the first '.'), otherwise returns the original filename.
    pub fn strip_extension(filename: &str) -> &str {
        if let Some(idx) = filename.find('.') {
            &filename[..idx]
        } else {
            filename
        }
    }

    /// Return the extension (anything after the first '.'), or null if there is no '.' in the file
    /// name.
    pub fn get_extension(filename: &str) -> Option<&str> {
        if let Some(idx) = filename.find('.') {
            Some(&filename[idx + 1..])
        } else {
            None
        }
    }
}
/** Name of the index segment file */
pub const SEGMENTS: &str = "segments";
/** Name of pending index segment file */
pub const PENDING_SEGMENTS: &str = "pending_segments";

lazy_static::lazy_static! {
    ///   All files created by codecs much match this pattern (checked in SegmentInfo).
    pub static ref CODEC_FILE_PATTERN: Regex = Regex::new(r"_[a-z0-9]+(_.*)?\..*").unwrap();
}

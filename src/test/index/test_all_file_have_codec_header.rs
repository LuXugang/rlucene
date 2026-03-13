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
use crate::core::codecs::{Codec, CodecUtil, CompoundFormat, LATEST_CODEC};
use crate::core::index::IndexFileNames;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IOContext};
use crate::core::util::StringHelper;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Test that a plain default puts codec headers in all files
#[allow(dead_code)] // for quick search
pub struct TestAllFilesHaveCodecHeader;

// TODO LineFileDocs未实现
#[test]
fn test() -> Result<()> {
    // let mut random = random();
    // let dir = new_directory_shared(&mut random)?;
    //
    // let analyzer = MockAnalyzer::new(&mut random);
    // let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
    //
    // let mut riw = RandomIndexWriter::with_config(&mut random, dir.clone(), conf);
    //
    // let mut docs = LineFileDocs::new(&mut random);
    //
    // for i in 0..100 {
    //     riw.add_document(docs.next_doc()?)?;
    //
    //     if random.random_range(0..7) == 0 {
    //         riw.commit()?;
    //     }
    //
    //     if random.random_range(0..20) == 0 {
    //         riw.delete_documents_with_terms(vec![Term::from_text("docid", i.to_string())])?;
    //     }
    //
    //     if random.random_range(0..15) == 0 {
    //         riw.w.update_numeric_doc_value(
    //             Term::from_text("docid", i.to_string()),
    //             "page_views",
    //             i as i64,
    //         )?;
    //     }
    // }
    //
    // riw.close()?;
    //
    // check_headers(dir.clone(), &mut HashMap::<String, String>::new())?;
    Ok(())
}
fn check_headers<D>(dir: Arc<D>, names_to_extensions: &mut HashMap<String, String>) -> Result<()>
where
    D: Directory,
{
    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    check_header(
        dir.as_ref(),
        sis.get_segments_file_name().unwrap().as_ref(),
        names_to_extensions,
        sis.get_id().unwrap(),
    )?;

    for si in sis.iter() {
        for file in si.files()? {
            check_header(dir.as_ref(), &file, names_to_extensions, si.info.get_id())?;
        }

        if si.info.get_use_compound_file() {
            let cfs_dir = LATEST_CODEC
                .compound_format()
                .get_compound_reader(dir.as_ref(), &si.info)?;

            for cfs_file in cfs_dir.list_all()? {
                check_header(&cfs_dir, &cfs_file, names_to_extensions, si.info.get_id())?;
            }
        }
    }

    Ok(())
}
fn check_header<D>(
    dir: &D,
    file: &str,
    names_to_extensions: &mut HashMap<String, String>,
    id: &[u8; StringHelper::ID_LENGTH],
) -> Result<()>
where
    D: Directory,
{
    let mut input = dir.open_input(file, &IOContext::read_once_io_context()?)?;

    let val = CodecUtil::read_be_int(&mut input)?;
    assert_eq!(
        CodecUtil::CODEC_MAGIC,
        val,
        "{} has no codec header, instead found: {}",
        file,
        val
    );

    let codec_name = input.read_string()?;
    assert!(!codec_name.is_empty());

    let extension = match IndexFileNames::get_extension(file) {
        Some(ext) => ext.to_string(),
        None => {
            assert!(file.starts_with(IndexFileNames::SEGMENTS));
            "<segments> (not a real extension, designates segments file)".to_string()
        },
    };

    let previous = names_to_extensions.insert(codec_name.clone(), extension.clone());
    if let Some(previous) = previous {
        assert_eq!(
            previous, extension,
            "extensions {} and {} share same codecName {}",
            previous, extension, codec_name
        );
    }

    input.read_int()?;
    CodecUtil::check_index_header_id(&mut input, id)?;

    Ok(())
}

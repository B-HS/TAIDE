use std::io::{Cursor, Write};

use taide_native_app::preview_spreadsheet_xls::workbook_stream;

const SECTOR_BYTES: usize = 512;
const FIRST_DIRECTORY_OFFSET: usize = 48;
const FIRST_DIFAT_OFFSET: usize = 68;
const HEADER_FAT_OFFSET: usize = 76;
const DIRECTORY_ENTRY_BYTES: usize = 128;
const DIRECTORY_STREAM_START_OFFSET: usize = 116;
const DIRECTORY_STREAM_LENGTH_OFFSET: usize = 120;
const END_OF_CHAIN: u32 = 0xfffffffe;
const FREE_SECTOR: u32 = 0xffffffff;
const LARGE_STREAM_BYTES: usize = 4097;

fn compound(version: cfb::Version, name: &str, payload: &[u8]) -> Vec<u8> {
    let mut file =
        cfb::CompoundFile::create_with_version(version, Cursor::new(Vec::new())).unwrap();
    let mut stream = file.create_stream(name).unwrap();
    stream.write_all(payload).unwrap();
    stream.flush().unwrap();
    drop(stream);
    file.flush().unwrap();
    file.into_inner().into_inner()
}

fn integer(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + size_of::<u32>()].try_into().unwrap())
}

#[test]
fn xls_cfb는_v3_v4_mini_regular_book과_할당전_fat_cycle_경계를_검사한다() {
    for version in [cfb::Version::V3, cfb::Version::V4] {
        for name in ["/Workbook", "/Book", "/WORKBOOK"] {
            for payload in [
                b"synthetic BIFF stream".to_vec(),
                vec![b'x'; LARGE_STREAM_BYTES],
            ] {
                let bytes = compound(version, name, &payload);
                assert_eq!(workbook_stream(&bytes).unwrap(), payload);
            }
        }
    }
    let bytes = compound(cfb::Version::V3, "/Workbook", b"synthetic BIFF stream");
    let fat = integer(&bytes, HEADER_FAT_OFFSET);
    let directory = integer(&bytes, FIRST_DIRECTORY_OFFSET);
    let fat_offset = (fat as usize + 1) * SECTOR_BYTES;
    let directory_offset = (directory as usize + 1) * SECTOR_BYTES;
    let mut repeated = bytes.clone();
    repeated[HEADER_FAT_OFFSET + size_of::<u32>()..HEADER_FAT_OFFSET + size_of::<u32>() * 2]
        .copy_from_slice(&fat.to_le_bytes());
    assert!(
        workbook_stream(&repeated)
            .unwrap_err()
            .to_string()
            .contains("repeats a FAT sector before allocation")
    );
    let mut difat_cycle = bytes.clone();
    difat_cycle[FIRST_DIFAT_OFFSET..FIRST_DIFAT_OFFSET + size_of::<u32>()]
        .copy_from_slice(&directory.to_le_bytes());
    for chunk in difat_cycle[directory_offset..directory_offset + SECTOR_BYTES]
        .as_chunks_mut::<4>()
        .0
    {
        *chunk = FREE_SECTOR.to_le_bytes();
    }
    difat_cycle
        [directory_offset + SECTOR_BYTES - size_of::<u32>()..directory_offset + SECTOR_BYTES]
        .copy_from_slice(&directory.to_le_bytes());
    assert!(
        workbook_stream(&difat_cycle)
            .unwrap_err()
            .to_string()
            .contains("DIFAT chain contains a cycle")
    );
    let mut directory_cycle = bytes.clone();
    let link = fat_offset + directory as usize * size_of::<u32>();
    directory_cycle[link..link + size_of::<u32>()].copy_from_slice(&directory.to_le_bytes());
    assert!(workbook_stream(&directory_cycle).is_err());
    let mut huge_length = bytes.clone();
    let entry = directory_offset + DIRECTORY_ENTRY_BYTES;
    huge_length[entry + DIRECTORY_STREAM_LENGTH_OFFSET
        ..entry + DIRECTORY_STREAM_LENGTH_OFFSET + size_of::<u64>()]
        .copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(workbook_stream(&huge_length).is_err());
    let mut regular = compound(
        cfb::Version::V3,
        "/Workbook",
        &vec![b'x'; LARGE_STREAM_BYTES],
    );
    let regular_directory = (integer(&regular, FIRST_DIRECTORY_OFFSET) as usize + 1) * SECTOR_BYTES;
    let stream_start = integer(
        &regular,
        regular_directory + DIRECTORY_ENTRY_BYTES + DIRECTORY_STREAM_START_OFFSET,
    );
    let regular_fat = (integer(&regular, HEADER_FAT_OFFSET) as usize + 1) * SECTOR_BYTES;
    let link = regular_fat + stream_start as usize * size_of::<u32>();
    regular[link..link + size_of::<u32>()].copy_from_slice(&stream_start.to_le_bytes());
    assert!(workbook_stream(&regular).is_err());
    assert!(workbook_stream(&compound(cfb::Version::V3, "/Other", b"ignored")).is_err());
    assert!(workbook_stream(&bytes[..bytes.len() - 1]).is_err());
    assert!(workbook_stream(b"not CFB").is_err());
    let mut invalid_fat = bytes.clone();
    invalid_fat[HEADER_FAT_OFFSET..HEADER_FAT_OFFSET + size_of::<u32>()]
        .copy_from_slice(&END_OF_CHAIN.to_le_bytes());
    assert!(workbook_stream(&invalid_fat).is_err());
    assert!(
        workbook_stream(&vec![
            0;
            taide_model::file::READ_ONLY_FILE_BYTES as usize + 1
        ])
        .is_err()
    );
}

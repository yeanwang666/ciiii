use test_utils::{append_bytes, cleanup_file, read_bytes, temp_file, write_bytes};

#[test]
fn file_io_basic_smoke() {
    let file_path = temp_file("starry-file-io", true).expect("创建临时文件");
    let initial_payload = b"starry file io smoke test";
    write_bytes(&file_path, initial_payload).expect("写入初始内容");

    let append_payload = b" + append";
    append_bytes(&file_path, append_payload).expect("追加数据");

    let mut expected = initial_payload.to_vec();
    expected.extend_from_slice(append_payload);

    let read_back = read_bytes(&file_path).expect("读取文件内容");
    cleanup_file(&file_path).expect("清理临时文件");

    assert_eq!(read_back, expected, "读回内容不一致");
}

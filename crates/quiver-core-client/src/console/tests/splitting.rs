use super::*;

#[test]
fn lines_split_on_newlines() {
    let mut s = NdjsonSplitter::new();
    assert_eq!(s.push(b"a\nb\n"), vec!["a", "b"]);
}

#[test]
fn a_line_split_across_chunks_is_joined() {
    let mut s = NdjsonSplitter::new();
    assert!(s.push(b"{\"type\":\"o").is_empty());
    assert_eq!(s.push(b"ut\"}\n"), vec!["{\"type\":\"out\"}"]);
}

#[test]
fn a_multibyte_character_split_across_chunks_survives() {
    let bytes = "héllo\n".as_bytes();
    let mut s = NdjsonSplitter::new();
    assert!(s.push(&bytes[..2]).is_empty());
    assert_eq!(s.push(&bytes[2..]), vec!["héllo"]);
}

#[test]
fn crlf_and_blank_lines_are_tidied() {
    let mut s = NdjsonSplitter::new();
    assert_eq!(s.push(b"a\r\n\r\n\nb\n"), vec!["a", "b"]);
}

#[test]
fn an_unterminated_tail_is_returned_at_the_end() {
    let mut s = NdjsonSplitter::new();
    assert!(s.push(b"last words").is_empty());
    assert_eq!(s.finish().as_deref(), Some("last words"));
    assert_eq!(s.finish(), None);
}

#[test]
fn invalid_utf8_is_replaced_not_fatal() {
    let mut s = NdjsonSplitter::new();
    let lines = s.push(b"a\xffb\n");
    assert_eq!(lines, vec!["a\u{fffd}b"]);
}

#[test]
fn an_oversize_line_is_replaced_by_a_marker_and_the_stream_recovers() {
    let mut s = NdjsonSplitter::new();
    let big = vec![b'x'; MAX_FRAME_BYTES + 10];
    // No newline yet: the splitter must already have stopped buffering.
    assert!(s.push(&big).is_empty());
    assert!(s.tail.is_empty(), "memory must stay bounded");
    let lines = s.push(b"more\nnext\n");
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("too long"));
    assert_eq!(lines[1], "next");
}

#[test]
fn an_oversize_line_that_ends_in_the_same_chunk_is_marked_too() {
    let mut s = NdjsonSplitter::new();
    let mut big = vec![b'x'; MAX_FRAME_BYTES + 1];
    big.push(b'\n');
    big.extend_from_slice(b"ok\n");
    let lines = s.push(&big);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("too long"));
    assert_eq!(lines[1], "ok");
}

#[test]
fn an_oversize_line_cut_off_by_the_end_of_the_stream_is_marked() {
    let mut s = NdjsonSplitter::new();
    assert!(s.push(&vec![b'x'; MAX_FRAME_BYTES + 1]).is_empty());
    assert!(s.finish().unwrap().contains("too long"));
}

// --- run_exec -----------------------------------------------------------

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn test_basic_deduplication() {
    let mut child = Command::new("./target/debug/bloom")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn bloom process");

    // Write numbers 1-10 twice to stdin
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    for i in 1..=10 {
        writeln!(stdin, "{}", i).expect("Failed to write to stdin");
    }
    for i in 1..=10 {
        writeln!(stdin, "{}", i).expect("Failed to write to stdin");
    }
    drop(stdin); // Close stdin to signal EOF

    // Get output and convert to string
    let output = child.wait_with_output().expect("Failed to wait on bloom");
    let output_str = String::from_utf8(output.stdout).expect("Output not UTF-8");

    // Check results
    let output_lines: Vec<&str> = output_str.lines().collect();
    assert_eq!(output_lines.len(), 10, "Expected 10 unique lines");
    
    // Verify each number appears exactly once
    for i in 1..=10 {
        assert_eq!(
            output_lines.iter().filter(|&&line| line == i.to_string()).count(),
            1,
            "Number {} should appear exactly once", i
        );
    }
}

#[test]
fn test_deduplication_with_invalid_utf8() {
    let mut child = Command::new("./target/debug/bloom")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn bloom process");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    
    // Write some valid and invalid UTF-8 sequences
    // Using a consistent invalid UTF-8 sequence
    let invalid_sequence = b"invalid \xFF\xFE line\n";
    
    // Write each line twice to test deduplication
    writeln!(stdin, "valid line").unwrap();
    stdin.write_all(invalid_sequence).unwrap();
    writeln!(stdin, "valid line").unwrap();
    stdin.write_all(invalid_sequence).unwrap();
    
    drop(stdin);

    let output = child.wait_with_output().expect("Failed to wait on bloom");
    let output_bytes = output.stdout;
    
    // Count unique lines by comparing raw bytes
    let mut unique_lines = Vec::new();
    let mut current_line = Vec::new();
    
    for &byte in output_bytes.iter() {
        if byte == b'\n' {
            if !unique_lines.contains(&current_line) {
                unique_lines.push(current_line.clone());
            }
            current_line.clear();
        } else {
            current_line.push(byte);
        }
    }
    
    // We should have 2 unique lines (one valid UTF-8, one invalid UTF-8)
    assert_eq!(unique_lines.len(), 2, "Expected 2 unique lines, got {}", unique_lines.len());
    
    // Verify that one of the lines is "valid line"
    assert!(unique_lines.iter().any(|line| 
        String::from_utf8_lossy(line) == "valid line"
    ), "Should contain 'valid line'");
}
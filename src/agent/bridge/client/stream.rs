//! NDJSON event stream iterator.

use anyhow::Result;
use std::io::{BufRead, BufReader};

use super::super::types::BridgeEvent;

/// Iterator over NDJSON event stream
pub(super) struct EventStream<R: std::io::Read> {
    reader: BufReader<R>,
    buffer: String,
}

impl<R: std::io::Read> EventStream<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: String::new(),
        }
    }
}

impl<R: std::io::Read> Iterator for EventStream<R> {
    type Item = Result<BridgeEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        // Use a loop instead of recursion to skip empty lines (avoids stack overflow)
        loop {
            self.buffer.clear();

            match self.reader.read_line(&mut self.buffer) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let trimmed = self.buffer.trim();
                    if trimmed.is_empty() {
                        continue; // Skip empty lines without recursion
                    }

                    return match serde_json::from_str::<BridgeEvent>(trimmed) {
                        Ok(event) => Some(Ok(event)),
                        Err(e) => Some(Err(anyhow::anyhow!(
                            "Failed to parse event: {} (line: {})",
                            e,
                            trimmed
                        ))),
                    };
                }
                Err(e) => return Some(Err(anyhow::anyhow!("Failed to read from stream: {}", e))),
            }
        }
    }
}

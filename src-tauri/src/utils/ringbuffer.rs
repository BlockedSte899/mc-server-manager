use std::collections::VecDeque;

/// A bounded, thread-safe console buffer.
#[derive(Default)]
pub struct RingBuffer {
    buf: VecDeque<String>,
    cap: usize,
}

impl RingBuffer {
    pub fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::new(),
            cap,
        }
    }

    pub fn push(&mut self, line: String) {
        if !line.ends_with('\n') {
            self.buf.push_back(line)
        } else {
            self.buf.push_back(line);
        }
        while self.buf.len() > self.cap {
            self.buf.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.buf.iter().cloned().collect()
    }

    pub fn last(&self) -> Option<String> {
        self.buf.back().cloned()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

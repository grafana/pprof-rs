// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use smallvec::SmallVec;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use crate::MAX_DEPTH;

#[derive(Clone, Debug)]
pub struct Frame {
    pub ip: usize,
}

#[derive(Clone)]
pub struct UnresolvedFrames {
    pub frames: SmallVec<[Frame; MAX_DEPTH]>,
    pub thread_id: u64,
}

impl Default for UnresolvedFrames {
    fn default() -> Self {
        let frames = SmallVec::with_capacity(MAX_DEPTH);
        Self {
            frames,
            thread_id: 0,
        }
    }
}

impl Debug for UnresolvedFrames {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.frames.fmt(f)
    }
}

impl UnresolvedFrames {
    pub fn new(frames: SmallVec<[Frame; MAX_DEPTH]>, thread_id: u64) -> Self {
        Self { frames, thread_id }
    }
}

impl PartialEq for UnresolvedFrames {
    fn eq(&self, other: &Self) -> bool {
        let (frames1, frames2) = (&self.frames, &other.frames);
        if self.thread_id != other.thread_id || frames1.len() != frames2.len() {
            false
        } else {
            Iterator::zip(frames1.iter(), frames2.iter()).all(|(s1, s2)| s1.ip == s2.ip)
        }
    }
}

impl Eq for UnresolvedFrames {}

impl Hash for UnresolvedFrames {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.frames.iter().for_each(|frame| frame.ip.hash(state));
        self.thread_id.hash(state);
    }
}

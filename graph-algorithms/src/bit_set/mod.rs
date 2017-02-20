use std::marker::PhantomData;
use std::mem;

use super::{Graph, NodeIndex};

type Word = u32;

pub struct BitSet<G: Graph> {
    bits_per_node: usize,
    words: Vec<Word>,
    graph: PhantomData<G>,
}

impl<G: Graph> BitSet<G> {
    pub fn new(graph: &G, bits_per_node: usize) -> Self {
        let num_nodes = graph.num_nodes();
        let words_per_node = words(bits_per_node);
        let words = vec![0; words_per_node * num_nodes];
        BitSet {
            bits_per_node: bits_per_node,
            words: words,
            graph: PhantomData,
        }
    }

    fn index(&self, node: G::Node) -> usize {
        node.as_usize() * words(self.bits_per_node)
    }

    pub fn empty_buf(&self) -> BitBuf {
        let words = words(self.bits_per_node);
        BitBuf { words: vec![0; words] }
    }

    pub fn bits(&self, node: G::Node) -> BitSlice {
        let start = self.index(node);
        let end = start + words(self.bits_per_node);
        BitSlice { words: &self.words[start..end] }
    }

    pub fn is_set(&self, node: G::Node, bit: usize) -> bool {
        self.bits(node).get(bit)
    }

    pub fn insert(&mut self, node: G::Node, bit: usize) -> bool {
        let start = self.index(node);
        let (word, bit) = words_bits(bit);
        let old_value = self.words[start + word];
        let new_value = old_value | (1 << bit);
        self.words[start + word] = new_value;
        old_value != new_value
    }

    pub fn insert_bits_from_slice(&mut self, node: G::Node, bits: BitSlice) -> bool {
        let start = self.index(node);
        set_from(&mut self.words[start..], bits)
    }

    pub fn insert_bits_from_node(&mut self,
                                 source_node: G::Node,
                                 target_node: G::Node)
                                 -> bool {
        if source_node == target_node {
            return false;
        }
        let words_per_node = words(self.bits_per_node);
        let source_start = source_node.as_usize() * words_per_node;
        let target_start = target_node.as_usize() * words_per_node;
        let mut changed = false;
        for offset in 0..words_per_node {
            let source_word = self.words[source_start + offset];
            let target_word = self.words[target_start + offset];
            let new_word = source_word | target_word;
            self.words[target_start + offset] = new_word;
            changed |= new_word != target_word;
        }
        changed
    }
}

#[derive(Copy, Clone)]
pub struct BitSlice<'a> {
    words: &'a [Word]
}

impl<'a> BitSlice<'a> {
    pub fn to_buf(self) -> BitBuf {
        BitBuf {
            words: self.words.to_vec()
        }
    }

    pub fn get(self, index: usize) -> bool {
        let (word, bit) = words_bits(index);
        let old_value = self.words[word];
        (old_value & (1 << bit)) != 0
    }
}

pub struct BitBuf {
    words: Vec<Word>,
}

impl BitBuf {
    pub fn as_slice(&self) -> BitSlice {
        BitSlice { words: &self.words }
    }

    pub fn get(&self, index: usize) -> bool {
        self.as_slice().get(index)
    }

    pub fn set(&mut self, index: usize) -> bool {
        self.mutate(index, |v, mask| v | mask)
    }

    pub fn kill(&mut self, index: usize) -> bool {
        self.mutate(index, |v, mask| v & !mask)
    }

    fn mutate<OP>(&mut self, index: usize, op: OP) -> bool
        where OP: FnOnce(/* value */ Word, /* mask */ Word) -> Word
    {
        let (word, bit) = words_bits(index);
        let old_value = self.words[word];
        let new_value = op(old_value, 1 << bit);
        self.words[word] = new_value;
        old_value != new_value
    }

    pub fn set_from(&mut self, bits: BitSlice) -> bool {
        set_from(&mut self.words, bits)
    }

    pub fn clear(&mut self) {
        for p in &mut self.words {
            *p = 0;
        }
    }
}

#[inline]
fn words_bits(x: usize) -> (usize, usize) {
    let d = mem::size_of::<Word>() * 8;
    (x / d, x % d)
}

#[inline]
fn words(x: usize) -> usize {
    let (w, b) = words_bits(x);
    if b != 0 {w + 1} else {w}
}

#[inline]
fn set_from(words: &mut [Word], bits: BitSlice) -> bool {
    let mut changed = false;
    for (out_word, in_word) in words.iter_mut().zip(bits.words) {
        let old_value = *out_word;
        let new_value = old_value | *in_word;
        *out_word = new_value;
        changed |= old_value != new_value;
    }
    changed
}

use std::ops::Range;

/// Macro to get a specific bit from a u16 value.
/// Returns true if the bit at the given index is 1, false otherwise.
macro_rules! get_bit16_macro {
    ($a:expr, $b:expr) => {{ (($a >> $b) & 0x1u16) == 1u16 }};
}

/// Verus-proof-wrapped version of `get_bit16_macro`.
#[allow(unused_macros)]
macro_rules! get_bit16 {
    ($($a:tt)*) => {
        verus_proof_macro_exprs!(get_bit16_macro!($($a)*))
    }
}

/// Macro to extract a range of bits from a u16 value.
/// $val: The u16 value.
/// $start: The starting bit index (inclusive).
/// $end: The ending bit index (exclusive).
macro_rules! get_bits16_macro {
    ($val:expr, $start:expr, $end:expr) => {{
        let bitlen = 16u16;
        let bits = ($val << (bitlen - $end)) >> (bitlen - $end);
        bits >> $start
    }};
}

/// Verus-proof-wrapped version of `get_bits16_macro`.
#[allow(unused_macros)]
macro_rules! get_bits16 {
    ($($a:tt)*) => {
        verus_proof_macro_exprs!(get_bits16_macro!($($a)*))
    }
}

/// Macro to set a specific bit in a u16 value.
/// $val: The u16 value to modify.
/// $idx: The index of the bit to set.
/// $bit: The boolean value to set the bit to (true for 1, false for 0).
macro_rules! set_bit16_macro {
    ($val:expr,$idx:expr, $bit:expr) => {{
        if $bit {
            $val | 1u16 << $idx
        } else {
            $val & (!(1u16 << $idx))
        }
    }};
}

/// Verus-proof-wrapped version of `set_bit16_macro`.
#[allow(unused_macros)]
macro_rules! set_bit16 {
    ($($a:tt)*) => {
        verus_proof_macro_exprs!(set_bit16_macro!($($a)*))
    }
}

/// Macro to set a range of bits in a u16 value.
/// $val: The u16 value to modify.
/// $start: The starting bit index (inclusive).
/// $end: The ending bit index (exclusive).
/// $new_val: The u16 value containing the bits to set.
macro_rules! set_bits16_macro {
    ($val:expr, $start:expr, $end:expr, $new_val:expr) => {{
        let bitlen = 16;
        let mask = !(!0u16 << (bitlen - $end) >> (bitlen - $end) >> $start << $start);
        ($val & mask) | ($new_val << $start)
    }};
}

/// Verus-proof-wrapped version of `set_bits16_macro`.
#[allow(unused_macros)]
macro_rules! set_bits16 {
    ($($a:tt)*) => {
        verus_proof_macro_exprs!(set_bits16_macro!($($a)*))
    }
}

pub trait BitAllocView {
    /// The bitmap has a total of CAP bits, numbered from 0 to CAP-1 inclusively.
    fn cap() -> usize;

    /// The default value. Workaround for `const fn new() -> Self`.
    fn default() -> Self
    where
        Self: Sized;

    fn verieasy_get(&self) -> Vec<u16>;
    
    /// Find a index not less than a given key, where the bit is free.
    fn next(&self, key: usize) -> Option<usize>;

    /// Whether there are free bits remaining
    fn any(&self) -> bool;

    /// Whether a specific bit is free
    fn test(&self, key: usize) -> bool;
}

/// Allocator of a bitmap, able to allocate / free bits.
pub trait BitAlloc: BitAllocView {
    /// Allocate a free bit.
    fn alloc(&mut self) -> Option<usize>;

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize>;

    /// Free an allocated bit.
    fn dealloc(&mut self, key: usize);

    /// Mark bits in the range as val
    fn set_range_to(&mut self, range: Range<usize>, val: bool);

    /// Mark bits in the range as unallocated (available)
    fn insert(&mut self, range: Range<usize>);

    /// Reverse of insert
    fn remove(&mut self, range: Range<usize>);
}

/// A bitmap of 256 bits
pub type BitAlloc256 = BitAllocCascade16<BitAlloc16>;
/// A bitmap of 4K bits
pub type BitAlloc4K = BitAllocCascade16<BitAlloc256>;
/// A bitmap of 64K bits
pub type BitAlloc64K = BitAllocCascade16<BitAlloc4K>;
/// A bitmap of 1M bits
pub type BitAlloc1M = BitAllocCascade16<BitAlloc64K>;

/// Implement the bit allocator by segment tree algorithm.
#[derive(Copy)]
pub struct BitAllocCascade16<T: BitAllocView> {
    pub bitset: BitAlloc16, // for each bit, 1 indicates available, 0 indicates inavailable
    pub sub: [T; 16],
}

impl BitAlloc256 {
    pub fn verieasy_new(bitmap: [u16; 16]) -> Self {
        let mut res = BitAlloc256::default();
        for (i, &bits) in bitmap.iter().enumerate() {
            res.sub[i] = BitAlloc16::verieasy_new(bits);
            res.bitset.set_bit(i as u16, res.sub[i].any());
        }
        res
    }
}

impl BitAlloc4K {
    pub fn verieasy_new(bitmap: [[u16; 16]; 16]) -> Self {
        let mut res = BitAlloc4K::default();
        for (i, &sub_bitmap) in bitmap.iter().enumerate() {
            res.sub[i] = BitAlloc256::verieasy_new(sub_bitmap);
            res.bitset.set_bit(i as u16, res.sub[i].any());
        }
        res
    }
}

impl<T: BitAllocView + Copy> Clone for BitAllocCascade16<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: BitAllocView + std::marker::Copy> BitAllocView for BitAllocCascade16<T> {
    // 每个子分配器的容量都是固定且相等的
    fn cap() -> usize {
        (T::cap() * 16) as usize
    }

    /// Creates a new `BitAllocCascade16` with all bits set to 0 (all free).
    fn default() -> Self {
        BitAllocCascade16 {
            bitset: BitAlloc16 { bits: 0 },
            sub: [T::default(); 16], // need the trait "std::marker::Copy"
        }
    }

    fn verieasy_get(&self) -> Vec<u16> {
        let mut v = Vec::new();
        for child in &self.sub {
            let child_vec = child.verieasy_get();
            v.extend(child_vec);
        }
        v
    }

    /// Checks if there are any free bits (bits set to 1) in the bitmap.
    fn any(&self) -> bool {
        self.bitset.any()
    }

    /// Tests if a specific bit at `index` is free (1) or allocated (0).
    fn test(&self, key: usize) -> bool {
        let seq_index: usize = key / T::cap(); //证明seq_index < 16

        let bit_index: usize = key % T::cap();
        let res = self.sub[seq_index].test(bit_index);

        res
    }

    /// Finds the next free bit (1) starting from `key` (inclusive).
    /// Returns `Some(index)` of the next free bit, or `None` if no free bits are found.
    fn next(&self, key: usize) -> Option<usize> {
        let idx: usize = key / T::cap();

        let mut i = idx;

        let mut result: Option<usize> = None;
        let mut curr_key = T::cap() * idx;

        while i < 16 {
            if self.bitset.get_bit(i as u16) {
                let base_key = if i == idx { key - T::cap() * idx } else { 0 };
                // 这里要先保证16叉树成型！
                let child = self.sub[i].next(base_key);
                if child.is_some() {
                    curr_key = T::cap() * i + child.unwrap();

                    result = Some(curr_key);
                    break;
                } else {
                }
            } else {
            }
            let old_i = i;
            let next_end = T::cap() * (old_i + 1);
            i += 1;

            curr_key = T::cap() * i;
        }
        result
    }
}

impl<T: BitAlloc + std::marker::Copy> BitAlloc for BitAllocCascade16<T> {
    fn alloc(&mut self) -> Option<usize> {
        if !self.any() {
            return None;
        }
        // Find the first free bit (least significant 1-bit).
        let i = self.bitset.bits.trailing_zeros() as usize;

        // change value
        let mut child = self.sub[i];
        let res_is_some = child.alloc();

        self.sub[i] = child;

        let res = res_is_some.unwrap() + i * T::cap();

        let bv_old: u16 = self.bitset.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, i, self.sub[i].any());

        self.bitset.bits = bv_new;

        Some(res as usize)
    }

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize> {
        if let Some(base) = find_contiguous(self, Self::cap(), size, align_log2) {
            let start = base;
            let end = base + size;
            self.remove(start..end);
            Some(base)
        } else {
            None
        }
    }

    fn dealloc(&mut self, key: usize) {
        let i: usize = key / T::cap(); //i < 16

        let bit_index: usize = key % T::cap();

        let mut child = self.sub[i];
        child.dealloc(bit_index);

        self.sub[i] = child;
        self.bitset.set_bit(i as u16, true);
    }

    fn set_range_to(&mut self, range: Range<usize>, val: bool) {
        let st = range.start;
        let ed = range.end;

        let first = st / T::cap(); //first sub allocator
        let last = (ed - 1) / T::cap(); //last sub allocator
        let n = last + 1;

        let mut i = first;

        let mut current_end = st;

        while i < n {
            let begin = if i == st / T::cap() { st % T::cap() } else { 0 };
            let stop = if i == (ed - 1) / T::cap() {
                if ed % T::cap() == 0 {
                    T::cap()
                } else {
                    ed % T::cap()
                }
            } else {
                T::cap()
            };

            let mut child = self.sub[i];

            if val {
                child.insert(begin..stop);
            } else {
                child.remove(begin..stop);
            }

            self.sub[i] = child;
            self.bitset.set_bit(i as u16, self.sub[i].any());

            current_end = stop + i * T::cap();

            i += 1;
        }
    }

    fn insert(&mut self, range: Range<usize>) {
        self.set_range_to(range, true);
    }

    fn remove(&mut self, range: Range<usize>) {
        self.set_range_to(range, false);
    }
}

/// Represents a 16-bit bitmap allocator.
#[derive(Clone, Copy)]
pub struct BitAlloc16 {
    pub bits: u16,
}

impl BitAlloc16 {
    /// Gets the boolean value of a specific bit at `index`.
    fn get_bit(&self, index: u16) -> bool {
        get_bit16_macro!(self.bits, index as u16)
    }

    /// Extracts a range of bits from the bitmap as a u16 value.
    fn get_bits(&self, range: Range<u16>) -> u16 {
        let bv_gets = get_bits16_macro!(self.bits, range.start, range.end);
        bv_gets
    }

    /// Sets the boolean value of a specific bit at `index`.
    fn set_bit(&mut self, index: u16, bit: bool) {
        let bit_index: u16 = index % 16;
        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, bit_index as u16, bit);
        self.bits = bv_new;
    }

    /// Sets a range of bits in the bitmap to a given u16 value.
    fn set_bits(&mut self, range: Range<u16>, value: u16) {
        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bits16_macro!(bv_old, range.start, range.end, value);

        self.bits = bv_new;
    }
}

impl BitAlloc16 {
    pub fn verieasy_new(bits: u16) -> Self {
        Self { bits }
    }
}

impl BitAllocView for BitAlloc16 {
    /// The maximum capacity of the bitmap (16 bits).
    fn cap() -> usize {
        16
    }

    /// Creates a new `BitmapAllocator16` with all bits set to 0 (all free).
    fn default() -> Self {
        BitAlloc16 { bits: 0 }
    }

    fn verieasy_get(&self) -> Vec<u16> {
        let mut v = Vec::with_capacity(1);
        v.push(self.bits);
        v
    }

    /// Checks if there are any free bits (bits set to 1) in the bitmap.
    fn any(&self) -> bool {
        self.bits != 0
    }

    /// Tests if a specific bit at `index` is free (1) or allocated (0).
    fn test(&self, key: usize) -> bool {
        let res = self.get_bit(key as u16);
        res
    }

    /// Finds the next free bit (1) starting from `key` (inclusive).
    /// Returns `Some(index)` of the next free bit, or `None` if no free bits are found.
    fn next(&self, key: usize) -> Option<usize> {
        let n = u16::BITS as u16;
        let mut result = None;
        let mut i = key as u16;
        while i < n {
            if self.get_bit(i) {
                result = Some(i as usize);
                break;
            }
            i += 1;
        }
        result
    }
}

impl BitAlloc for BitAlloc16 {
    /// Allocates a single free bit (represented by 1) and sets it to 0 (allocated).
    /// Returns `Some(index)` if successful, `None` if no free bits are available.
    fn alloc(&mut self) -> Option<usize> {
        if !self.any() {
            return None;
        }
        // Find the first free bit (least significant 1-bit).
        let i = self.bits.trailing_zeros() as u16;

        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, i, false);

        self.bits = bv_new;

        Some(i as usize)
    }

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize> {
        if let Some(base) = find_contiguous(self, Self::cap(), size, align_log2) {
            let start = base;
            let end = base + size;
            self.remove(start..end);
            Some(base)
        } else {
            None
        }
    }

    /// Deallocates a single bit at `index` by setting it to 1 (free).
    fn dealloc(&mut self, key: usize) {
        self.set_bit(key as u16, true);
    }

    /// Marks a range of bits as allocated (sets them to 0).
    fn set_range_to(&mut self, range: Range<usize>, val: bool) {
        if val {
            self.insert(range);
        } else {
            self.remove(range);
        }
    }

    /// Marks a range of bits as free (sets them to 1).
    fn insert(&mut self, range: Range<usize>) {
        let width = (range.end - range.start) as u16;
        let insert_val = 0xffffu16 >> ((16 - width) as u16);

        let range_u16 = (range.start as u16)..(range.end as u16);
        self.set_bits(range_u16, insert_val);
    }

    /// Marks a range of bits as allocated (sets them to 0).
    fn remove(&mut self, range: Range<usize>) {
        let value: u16 = 0;
        let width: u16 = (range.end - range.start) as u16;

        let range_u16 = (range.start as u16)..(range.end as u16);
        self.set_bits(range_u16, value);
    }
}

/// Finds a contiguous block of `size` free bits within the bitmap, respecting `align_log2`.
/// Returns the base index of the found block, or `None` if no such block exists.
fn find_contiguous<T: BitAllocView>(
    ba: &T,
    capacity: usize,
    size: usize,
    align_log2: usize,
) -> Option<usize> {
    if (capacity < (1usize << align_log2)) || !ba.any() {
        return None;
    }

    let mut base: usize = 0;

    let mut offset: usize = base;
    let mut res: Option<usize> = None;
    while offset < capacity {
        if let Some(next) = ba.next(offset) {
            if next != offset {
                // it can be guarenteed that no bit in (offset..next) is free
                // move to next aligned position after next-1
                base = (((((next - 1) as usize) >> align_log2) + 1) as usize) << align_log2;
                offset = base;
                continue;
            }
        } else {
            // No more free bits found from `offset` to `capacity`.
            return None;
        }
        offset += 1;
        if offset - base == size {
            res = Some(base);
            return res;
        }
    }
    None
}

fn main() {}

// #[test]
pub fn bitalloc16() {
    let mut ba = BitAlloc16::default();
    assert_eq!(BitAlloc16::cap(), 16);
    ba.insert(0..16);
    for i in 0..16 {
        assert_eq!(ba.test(i), true);
    }
    ba.remove(2..8);
    assert_eq!(ba.alloc(), Some(0));
    assert_eq!(ba.alloc(), Some(1));
    assert_eq!(ba.alloc(), Some(8));
    ba.dealloc(0);
    ba.dealloc(1);
    ba.dealloc(8);

    for _ in 0..10 {
        assert!(ba.alloc().is_some());
    }
    assert!(!ba.any());
    assert!(ba.alloc().is_none());
}

// #[test]
pub fn bitalloc4k() {
    let mut ba = BitAlloc4K::default();
    assert_eq!(BitAlloc4K::cap(), 4096);
    ba.insert(0..4096);
    for i in 0..4096 {
        assert_eq!(ba.test(i), true);
    }
    ba.remove(2..4094);
    for i in 0..4096 {
        assert_eq!(ba.test(i), i < 2 || i >= 4094);
    }
    assert_eq!(ba.alloc(), Some(0));
    assert_eq!(ba.alloc(), Some(1));
    assert_eq!(ba.alloc(), Some(4094));
    ba.dealloc(0);
    ba.dealloc(1);
    ba.dealloc(4094);

    for _ in 0..4 {
        assert!(ba.alloc().is_some());
    }
    assert!(ba.alloc().is_none());
}

// #[test]
pub fn bitalloc_contiguous() {
    let mut ba0 = BitAlloc16::default();
    ba0.insert(0..BitAlloc16::cap());
    ba0.remove(3..6);
    assert_eq!(ba0.next(0), Some(0));
    assert_eq!(ba0.alloc_contiguous(1, 1), Some(0));
    assert_eq!(find_contiguous(&ba0, BitAlloc4K::cap(), 2, 0), Some(1));

    let mut ba = BitAlloc4K::default();
    ba.alloc();
    assert_eq!(BitAlloc4K::cap(), 4096);
    ba.insert(0..BitAlloc4K::cap());
    ba.remove(3..6);
    assert_eq!(ba.next(0), Some(0));
    assert_eq!(ba.alloc_contiguous(1, 1), Some(0));
    assert_eq!(ba.next(0), Some(1));
    assert_eq!(ba.next(1), Some(1));
    assert_eq!(ba.next(2), Some(2));
    assert_eq!(find_contiguous(&ba, BitAlloc4K::cap(), 2, 0), Some(1));
    assert_eq!(ba.alloc_contiguous(2, 0), Some(1));
    assert_eq!(ba.alloc_contiguous(2, 3), Some(8));
    ba.remove(0..4096 - 64);
    assert_eq!(ba.alloc_contiguous(128, 7), None);
    assert_eq!(ba.alloc_contiguous(7, 3), Some(4096 - 64));
    ba.insert(321..323);
    assert_eq!(ba.alloc_contiguous(2, 1), Some(4096 - 64 + 8));
    assert_eq!(ba.alloc_contiguous(2, 0), Some(321));
    assert_eq!(ba.alloc_contiguous(64, 6), None);
    assert_eq!(ba.alloc_contiguous(32, 4), Some(4096 - 48));
    for i in 0..4096 - 64 + 7 {
        ba.dealloc(i);
    }
    for i in 4096 - 64 + 8..4096 - 64 + 10 {
        ba.dealloc(i);
    }
    for i in 4096 - 48..4096 - 16 {
        ba.dealloc(i);
    }
}

pub fn bitalloc1m(){
    let mut ba0 = BitAlloc1M::default();
    ba0.insert(0..BitAlloc1M::cap());
    ba0.remove(3..6);
    assert_eq!(ba0.next(0), Some(0));
    assert_eq!(ba0.alloc_contiguous(1, 1), Some(0));
    assert_eq!(find_contiguous(&ba0, BitAlloc4K::cap(), 2, 0), Some(1));

    let mut ba = BitAlloc1M::default();
    ba.alloc();
    assert_eq!(BitAlloc1M::cap(), 1048576);
    ba.insert(0..BitAlloc1M::cap());
    ba.remove(3..6);
    assert_eq!(ba.next(0), Some(0));
    assert_eq!(ba.alloc_contiguous(1, 1), Some(0));
    assert_eq!(ba.next(0), Some(1));
    assert_eq!(ba.next(1), Some(1));
    assert_eq!(ba.next(2), Some(2));
    assert_eq!(ba.alloc_contiguous(2, 0), Some(1));
    assert_eq!(ba.alloc_contiguous(2, 3), Some(8));
    ba.remove(0..4096 - 64);
    assert_eq!(ba.alloc_contiguous(128, 7), Some(4096));
    assert_eq!(ba.alloc_contiguous(7, 3), Some(4096 - 64));
    ba.insert(321..323);
    assert_eq!(ba.alloc_contiguous(2, 1), Some(4096 - 64 + 8));
    assert_eq!(ba.alloc_contiguous(2, 0), Some(321));
    assert_eq!(ba.alloc_contiguous(64, 6), Some(4224));
    assert_eq!(ba.alloc_contiguous(32, 4), Some(4096 - 48));
    for i in 0..4096 - 64 + 7 {
        ba.dealloc(i);
    }
    for i in 4096 - 64 + 8..4096 - 64 + 10 {
        ba.dealloc(i);
    }
    for i in 4096 - 48..4096 - 16 {
        ba.dealloc(i);
    }
}

pub fn bitalloc1m_alloc(){
    let mut ba = BitAlloc1M::default();
    ba.alloc();
}

pub fn bitalloc1m_alloc_contiguous(){
    let mut ba = BitAlloc1M::default();
    ba.alloc_contiguous(1588, 1);
}

pub fn bitalloc1m_dealloc(){
    let mut ba = BitAlloc1M::default();
    ba.dealloc(251);
}

pub fn bitalloc1m_insert(){
    let mut ba = BitAlloc1M::default();
    ba.insert(0..BitAlloc1M::cap());
}

pub fn bitalloc1m_remove(){
    let mut ba = BitAlloc1M::default();
    ba.remove(0..BitAlloc1M::cap());
}

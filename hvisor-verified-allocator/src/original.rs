use std::ops::Range;

/// A generic trait which provides methods for extracting and setting specific bits or ranges of
/// bits.
pub trait BitField {
    /// Returns the length, eg number of bits, in this bit field.
    ///
    /// ```rust
    /// use bit_field::BitField;
    ///
    /// assert_eq!(u32::bit_length(), 32);
    /// assert_eq!(u64::bit_length(), 64);
    /// ```
    fn bit_length() -> usize;

    /// Obtains the bit at the index `bit`; note that index 0 is the least significant bit, while
    /// index `length() - 1` is the most significant bit.
    ///
    /// ```rust
    /// use bit_field::BitField;
    ///
    /// let value: u32 = 0b110101;
    ///
    /// assert_eq!(value.get_bit(1), false);
    /// assert_eq!(value.get_bit(2), true);
    /// ```
    ///
    /// ## Panics
    ///
    /// This method will panic if the bit index is out of bounds of the bit field.
    fn get_bit(&self, bit: usize) -> bool;

    /// Obtains the range of bits specified by `range`; note that index 0 is the least significant
    /// bit, while index `length() - 1` is the most significant bit.
    ///
    /// ```rust
    /// use bit_field::BitField;
    ///
    /// let value: u32 = 0b110101;
    ///
    /// assert_eq!(value.get_bits(0..3), 0b101);
    /// assert_eq!(value.get_bits(2..6), 0b1101);
    /// ```
    ///
    /// ## Panics
    ///
    /// This method will panic if the start or end indexes of the range are out of bounds of the
    /// bit field.
    fn get_bits(&self, range: Range<usize>) -> Self;

    /// Sets the bit at the index `bit` to the value `value` (where true means a value of '1' and
    /// false means a value of '0'); note that index 0 is the least significant bit, while index
    /// `length() - 1` is the most significant bit.
    ///
    /// ```rust
    /// use bit_field::BitField;
    ///
    /// let mut value = 0u32;
    ///
    /// value.set_bit(1, true);
    /// assert_eq!(value, 2u32);
    ///
    /// value.set_bit(3, true);
    /// assert_eq!(value, 10u32);
    ///
    /// value.set_bit(1, false);
    /// assert_eq!(value, 8u32);
    /// ```
    ///
    /// ## Panics
    ///
    /// This method will panic if the bit index is out of the bounds of the bit field.
    fn set_bit(&mut self, bit: usize, value: bool) -> &mut Self;

    /// Sets the range of bits defined by the range `range` to the lower bits of `value`; to be
    /// specific, if the range is N bits long, the N lower bits of `value` will be used; if any of
    /// the other bits in `value` are set to 1, this function will panic.
    ///
    /// ```rust
    /// use bit_field::BitField;
    ///
    /// let mut value = 0u32;
    ///
    /// value.set_bits(0..2, 0b11);
    /// assert_eq!(value, 0b11);
    ///
    /// value.set_bits(0..4, 0b1010);
    /// assert_eq!(value, 0b1010);
    /// ```
    ///
    /// ## Panics
    ///
    /// This method will panic if the range is out of bounds of the bit field, or if there are `1`s
    /// not in the lower N bits of `value`.
    fn set_bits(&mut self, range: Range<usize>, value: Self) -> &mut Self;
}

/// An internal macro used for implementing BitField on the standard integral types.
macro_rules! bitfield_numeric_impl {
    ($($t:ty)*) => ($(
        impl BitField for $t {
            fn bit_length() -> usize {
                ::std::mem::size_of::<Self>() as usize * 8
            }

            fn get_bit(&self, bit: usize) -> bool {
                assert!(bit < Self::bit_length());

                (*self & (1 << bit)) != 0
            }

            fn get_bits(&self, range: Range<usize>) -> Self {
                assert!(range.start < Self::bit_length());
                assert!(range.end <= Self::bit_length());
                assert!(range.start < range.end);

                // shift away high bits
                let bits = *self << (Self::bit_length() - range.end) >> (Self::bit_length() - range.end);

                // shift away low bits
                bits >> range.start
            }

            fn set_bit(&mut self, bit: usize, value: bool) -> &mut Self {
                assert!(bit < Self::bit_length());

                if value {
                    *self |= 1 << bit;
                } else {
                    *self &= !(1 << bit);
                }

                self
            }

            fn set_bits(&mut self, range: Range<usize>, value: Self) -> &mut Self {
                assert!(range.start < Self::bit_length());
                assert!(range.end <= Self::bit_length());
                assert!(range.start < range.end);
                assert!(value << (Self::bit_length() - (range.end - range.start)) >>
                        (Self::bit_length() - (range.end - range.start)) == value,
                        "value does not fit into bit range");

                let bitmask: Self = !(!0 << (Self::bit_length() - range.end) >>
                                    (Self::bit_length() - range.end) >>
                                    range.start << range.start);

                // set bits
                *self = (*self & bitmask) | (value << range.start);

                self
            }
        }
    )*)
}

bitfield_numeric_impl! { u16 }

/// Allocator of a bitmap, able to allocate / free bits.
pub trait BitAlloc: Default {
    /// The bitmap has a total of CAP bits, numbered from 0 to CAP-1 inclusively.
    const CAP: usize;

    /// The default value. Workaround for `const fn new() -> Self`.
    #[allow(clippy::declare_interior_mutable_const)]
    const DEFAULT: Self;

    fn verieasy_get(&self) -> Vec<u16>;

    /// Allocate a free bit.
    fn alloc(&mut self) -> Option<usize>;

    /// Allocate a free block with a given size, and return the first bit position.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize>;

    /// Find a index not less than a given key, where the bit is free.
    fn next(&self, key: usize) -> Option<usize>;

    /// Free an allocated bit.
    fn dealloc(&mut self, key: usize);

    /// Mark bits in the range as unallocated (available)
    fn insert(&mut self, range: Range<usize>);

    /// Reverse of insert
    fn remove(&mut self, range: Range<usize>);

    /// Whether there are free bits remaining
    fn any(&self) -> bool;

    /// Whether a specific bit is free
    fn test(&self, key: usize) -> bool;
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
#[derive(Default)]
pub struct BitAllocCascade16<T: BitAlloc> {
    bitset: u16, // for each bit, 1 indicates available, 0 indicates inavailable
    sub: [T; 16],
}

impl BitAlloc256 {
    pub fn verieasy_new(bitmap: [u16; 16]) -> Self {
        let mut res = BitAlloc256::DEFAULT;
        for (i, &bits) in bitmap.iter().enumerate() {
            res.sub[i] = BitAlloc16::verieasy_new(bits);
            res.bitset.set_bit(i, res.sub[i].any());
        }
        res
    }
}

impl BitAlloc4K {
    pub fn verieasy_new(bitmap: [u16; 256]) -> Self {
        let mut res = BitAlloc4K::DEFAULT;
        for i in 0..16 {
            let mut sub_bitmap = [0u16; 16];
            for j in 0..16 {
                sub_bitmap[j] = bitmap[i * 16 + j];
            }
            res.sub[i] = BitAlloc256::verieasy_new(sub_bitmap);
            res.bitset.set_bit(i, res.sub[i].any());
        }
        res
    }
}

impl BitAlloc64K {
    pub fn verieasy_new(bitmap: [u16; 4096]) -> Self {
        let mut res = BitAlloc64K::DEFAULT;
        for i in 0..16 {
            let mut sub_bitmap = [0u16; 256];
            for j in 0..256 {
                sub_bitmap[j] = bitmap[i * 256 + j];
            }
            res.sub[i] = BitAlloc4K::verieasy_new(sub_bitmap);
            res.bitset.set_bit(i, res.sub[i].any());
        }
        res
    }
}

impl BitAlloc1M {
    pub fn verieasy_new(bitmap: [u16; 65536]) -> Self {
        let mut res = BitAlloc1M::DEFAULT;
        for i in 0..16 {
            let mut sub_bitmap = [0u16; 4096];
            for j in 0..4096 {
                sub_bitmap[j] = bitmap[i * 4096 + j];
            }
            res.sub[i] = BitAlloc64K::verieasy_new(sub_bitmap);
            res.bitset.set_bit(i, res.sub[i].any());
        }
        res
    }
}

impl<T: BitAlloc> BitAlloc for BitAllocCascade16<T> {
    const CAP: usize = T::CAP * 16;

    const DEFAULT: Self = BitAllocCascade16 {
        bitset: 0,
        sub: [T::DEFAULT; 16],
    };

    fn verieasy_get(&self) -> Vec<u16> {
        let mut v = Vec::new();
        for child in &self.sub {
            let child_vec = child.verieasy_get();
            v.extend(child_vec);
        }
        v
    }

    fn alloc(&mut self) -> Option<usize> {
        if self.any() {
            let i = self.bitset.trailing_zeros() as usize;
            let res = self.sub[i].alloc().unwrap() + i * T::CAP;
            self.bitset.set_bit(i, self.sub[i].any());
            Some(res)
        } else {
            None
        }
    }
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize> {
        if let Some(base) = find_contiguous(self, Self::CAP, size, align_log2) {
            self.remove(base..base + size);
            Some(base)
        } else {
            None
        }
    }
    fn dealloc(&mut self, key: usize) {
        let i = key / T::CAP;
        self.sub[i].dealloc(key % T::CAP);
        self.bitset.set_bit(i, true);
    }
    fn insert(&mut self, range: Range<usize>) {
        self.for_range(range, |sub: &mut T, range| sub.insert(range));
    }
    fn remove(&mut self, range: Range<usize>) {
        self.for_range(range, |sub: &mut T, range| sub.remove(range));
    }
    fn any(&self) -> bool {
        self.bitset != 0
    }
    fn test(&self, key: usize) -> bool {
        self.sub[key / T::CAP].test(key % T::CAP)
    }
    fn next(&self, key: usize) -> Option<usize> {
        let idx = key / T::CAP;
        (idx..16).find_map(|i| {
            if self.bitset.get_bit(i) {
                let key = if i == idx { key - T::CAP * idx } else { 0 };
                self.sub[i].next(key).map(|x| x + T::CAP * i)
            } else {
                None
            }
        })
    }
}

impl<T: BitAlloc> BitAllocCascade16<T> {
    fn for_range(&mut self, range: Range<usize>, f: impl Fn(&mut T, Range<usize>)) {
        let Range { start, end } = range;
        assert!(start <= end);
        assert!(end <= Self::CAP);
        for i in start / T::CAP..=(end - 1) / T::CAP {
            let begin = if start / T::CAP == i {
                start % T::CAP
            } else {
                0
            };
            let end = if end / T::CAP == i {
                end % T::CAP
            } else {
                T::CAP
            };
            f(&mut self.sub[i], begin..end);
            self.bitset.set_bit(i, self.sub[i].any());
        }
    }
}

/// A bitmap consisting of only 16 bits.
/// BitAlloc16 acts as the leaf (except the leaf bits of course) nodes
/// in the segment trees.
#[derive(Default)]
pub struct BitAlloc16(u16);

impl BitAlloc16 {
    pub fn verieasy_new(bits: u16) -> Self {
        Self(bits)
    }
}

impl BitAlloc for BitAlloc16 {
    const CAP: usize = 16;

    const DEFAULT: Self = BitAlloc16(0);

    fn verieasy_get(&self) -> Vec<u16> {
        let mut v = Vec::with_capacity(1);
        v.push(self.0);
        v
    }

    fn alloc(&mut self) -> Option<usize> {
        if self.any() {
            let i = self.0.trailing_zeros() as usize;
            self.0.set_bit(i, false);
            Some(i)
        } else {
            None
        }
    }
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> Option<usize> {
        if let Some(base) = find_contiguous(self, Self::CAP, size, align_log2) {
            self.remove(base..base + size);
            Some(base)
        } else {
            None
        }
    }
    fn dealloc(&mut self, key: usize) {
        self.0.set_bit(key, true);
    }
    fn insert(&mut self, range: Range<usize>) {
        self.0.set_bits(range.clone(), 0xffff.get_bits(range));
    }
    fn remove(&mut self, range: Range<usize>) {
        self.0.set_bits(range, 0);
    }
    fn any(&self) -> bool {
        self.0 != 0
    }
    fn test(&self, key: usize) -> bool {
        self.0.get_bit(key)
    }
    fn next(&self, key: usize) -> Option<usize> {
        (key..16).find(|&i| self.0.get_bit(i))
    }
}

fn find_contiguous(
    ba: &impl BitAlloc,
    capacity: usize,
    size: usize,
    align_log2: usize,
) -> Option<usize> {
    if align_log2 >= 64 || capacity < (1 << align_log2) || !ba.any() {
        return None;
    }
    let mut base = 0;
    let mut offset = base;
    while offset < capacity {
        if let Some(next) = ba.next(offset) {
            if next != offset {
                // it can be guarenteed that no bit in (offset..next) is free
                // move to next aligned position after next-1
                assert!(next > offset);
                base = (((next - 1) >> align_log2) + 1) << align_log2;
                assert_ne!(offset, next);
                offset = base;
                continue;
            }
        } else {
            return None;
        }
        offset += 1;
        if offset - base == size {
            return Some(base);
        }
    }
    None
}

// #[test]
#[ignore]
pub fn bitalloc16() {
    let mut ba = BitAlloc16::default();
    assert_eq!(BitAlloc16::CAP, 16);
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
#[ignore]
pub fn bitalloc4k() {
    let mut ba = BitAlloc4K::default();
    assert_eq!(BitAlloc4K::CAP, 4096);
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
#[ignore]
pub fn bitalloc_contiguous() {
    let mut ba0 = BitAlloc16::default();
    ba0.insert(0..BitAlloc16::CAP);
    ba0.remove(3..6);
    assert_eq!(ba0.next(0), Some(0));
    assert_eq!(ba0.alloc_contiguous(1, 1), Some(0));
    assert_eq!(find_contiguous(&ba0, BitAlloc4K::CAP, 2, 0), Some(1));

    let mut ba = BitAlloc4K::default();
    ba.alloc();
    assert_eq!(BitAlloc4K::CAP, 4096);
    ba.insert(0..BitAlloc4K::CAP);
    ba.remove(3..6);
    assert_eq!(ba.next(0), Some(0));
    assert_eq!(ba.alloc_contiguous(1, 1), Some(0));
    assert_eq!(ba.next(0), Some(1));
    assert_eq!(ba.next(1), Some(1));
    assert_eq!(ba.next(2), Some(2));
    assert_eq!(find_contiguous(&ba, BitAlloc4K::CAP, 2, 0), Some(1));
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

#[ignore]
pub fn bitalloc1m() {
    let mut ba0 = BitAlloc1M::default();
    ba0.insert(0..BitAlloc1M::CAP);
    ba0.remove(3..6);
    assert_eq!(ba0.next(0), Some(0));
    assert_eq!(ba0.alloc_contiguous(1, 1), Some(0));
    assert_eq!(find_contiguous(&ba0, BitAlloc4K::CAP, 2, 0), Some(1));

    let mut ba = BitAlloc1M::default();
    ba.alloc();
    assert_eq!(BitAlloc1M::CAP, 1048576);
    ba.insert(0..BitAlloc1M::CAP);
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

#[ignore]
pub fn bitalloc1m_alloc() {
    let mut ba = BitAlloc1M::default();
    ba.alloc();
}

#[ignore]
pub fn bitalloc1m_alloc_contiguous() {
    let mut ba = BitAlloc1M::default();
    ba.alloc_contiguous(1588, 1);
}

#[ignore]
pub fn bitalloc1m_dealloc() {
    let mut ba = BitAlloc1M::default();
    ba.dealloc(251);
}

#[ignore]
pub fn bitalloc1m_insert() {
    let mut ba = BitAlloc1M::default();
    ba.insert(0..BitAlloc1M::CAP);
}

#[ignore]
pub fn bitalloc1m_remove() {
    let mut ba = BitAlloc1M::default();
    ba.remove(0..BitAlloc1M::CAP);
}

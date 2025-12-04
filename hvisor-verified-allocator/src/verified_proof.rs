use vstd::{prelude::*, seq_lib::*};
use core::ops::Range;
use core::clone::Clone;
use core::marker::{Sized, Copy};
use core::option::Option::*;

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

verus! {

global layout usize is size == 8;

/// Converts a u16 value into a sequence of boolean bits.
pub open spec fn u16_view(u: u16) -> Seq<bool> {
    Seq::new(16, |i: int| get_bit16!(u, i as u16))
}

#[verifier::bit_vector]
proof fn set_bit_u16_preserves_others(bv_new: u16, bv_old: u16, index: u16, bit: bool)
    requires
        bv_new == set_bit16!(bv_old, index, bit),
        index < 16,
    ensures
        get_bit16!(bv_new, index) == bit,
        forall|loc2: u16|
            (loc2 < 16 && loc2 != index) ==> (get_bit16!(bv_new, loc2) == get_bit16!(bv_old, loc2)),
{
}

#[verifier::bit_vector]
proof fn set_bits_u16_preserves_others(bv_new: u16, bv_old: u16, st:u16, ed:u16, val: u16)
    requires
        st < 16,
        ed <= 16,
        st < ed,
        // Ensure `val` fits within the specified bit range (ed - st).
        val << ((u16::BITS) as u16 - (ed - st)) as usize >>
            ((u16::BITS) as u16 - (ed - st)) as usize == val,
        bv_new == set_bits16!(bv_old, st, ed, val),
    ensures
        get_bits16!(bv_new, st, ed) == val,
        forall|loc2: u16|
            (0 <= loc2 < st || ed <= loc2 < 16) ==> (get_bit16!(bv_new, loc2) == get_bit16!(bv_old, loc2)),
{
}

#[verifier::bit_vector]
proof fn get_bits_u16_correctness(bv_gets: u16, bits: u16, st:u16, ed:u16)
    requires
        bv_gets == get_bits16!(bits, st, ed),
        st < 16,
        ed <= 16,
        st < ed,
    ensures
        forall|i: u16| 0 <= i < (ed - st) ==> ((get_bit16!(bv_gets, i)) == get_bit16!(bits, (st + i) as u16)),
{
}

pub trait BitAllocView {
    /// Specification function to view the internal u16 as a sequence of booleans.
    spec fn view(&self) -> Seq<bool>;

    /// The bitmap has a total of CAP bits, numbered from 0 to CAP-1 inclusively.
    fn cap() -> (res:usize)
        requires
            Self::cascade_not_overflow(),
        ensures
            res == Self::spec_cap(),
    ;

    spec fn spec_cap() -> (res:usize);

    spec fn cascade_not_overflow() -> bool;

    spec fn lemma_cap_is_pow16_pre() -> bool;

    /// The capacity is an exponential multiple of 16.
    /// and the current bitmap only supports the maximum allocatable page size of 1M.
    proof fn lemma_cap_is_pow16()
        requires
            Self::lemma_cap_is_pow16_pre(),
            Self::cascade_not_overflow(),
        ensures
            is_pow16(Self::spec_cap()),
    ;

    /// The default value. Workaround for `const fn new() -> Self`.
    fn default() -> Self where Self: Sized;

    /// Structure is well_formed
    spec fn wf(&self) -> bool;

    /// Find a index not less than a given key, where the bit is free.
    fn next(&self, key: usize) -> (res: Option<usize>)
        requires
            self.wf(),
            key < Self::spec_cap(),
        ensures
            self.wf(),
            match res {
                Some(re) => {
                    // If successful, returns the first free index `re` that is not less than `key`.
                    // All indices between `key` and `re` (exclusive) must be allocated (false).
                    &&& self@[re as int] == true
                    &&& re < Self::spec_cap()
                    &&& re >= key
                    &&& forall|i: int| key <= i < re ==> self@[i] == false
                },
                None => {
                    // If failed, all indices from `key` to the end are allocated (false).
                    forall|i: int| key <= i < Self::spec_cap() ==> self@[i] == false
                }
            },
    ;

    /// Lemma: When self is well-formed, spec_any() is equivalent to
    /// “there exists an index j such that self@[j] == true”,
    /// linking the abstract semantics with the concrete boolean bits.
    proof fn lemma_bits_nonzero_implies_exists_true(&self)
        requires
            self.wf(),
        ensures
            self.wf(),
            self.spec_any() == exists|j:int| 0 <= j < Self::spec_cap() && self@[j] == true,
    ;

    /// Whether there are free bits remaining
    fn any(&self) -> (res: bool)
        ensures
            res == self.spec_any(),
    ;

    spec fn spec_any(&self) -> bool;

    /// Whether a specific bit is free
    fn test(&self, key: usize) -> (res: bool)
        requires
            self.wf(),
            key < Self::spec_cap(),
        ensures
            self.wf(),
            res == self@[key as int],
    ;
}

/// Allocator of a bitmap, able to allocate / free bits.
pub trait BitAlloc: BitAllocView{
    /// Allocate a free bit.
    fn alloc(&mut self) -> (res:Option<usize>)
        requires
            old(self).wf(),
        ensures
            match res {
                Some(i) => {
                    // If successful, a previously free index `i` is now allocated (set to false).
                    // Other indices remain unchanged.
                    &&& i < Self::spec_cap()
                    &&& old(self)@[i as int] == true
                    &&& forall |j: int| 0 <= j < i ==> old(self)@[j] == false
                    // &&& Self::update_in_place(old(self)@,self@,i as int)
                    &&& self@ == old(self)@.update(i as int, false)
                    &&& self.wf()
                    // &&& self.view_16() == old(self).view_16().update((i / 16) as int,self.sub[i].spec_any())
                    // &&& Self::spec_cap() != 16 ==> self.bitset@ == old(self).bitset@.update(i as int, self.sub[i].spec_any())
                },
                None => {
                    // If failed, all bits were already allocated (self.bits is 0), and the state is unchanged.
                    &&& !old(self).spec_any()
                    &&& forall |j: int| 0 <= j < Self::spec_cap() ==> old(self)@[j] == false
                    &&& self@ == old(self)@
                    &&& self.wf()
                },
            }
    ;

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> (res: Option<usize>)
        requires
            old(self).wf(),
            0 < size && size <= Self::spec_cap(),
            align_log2 < 64,
        ensures
            self.wf(),
            match res {
                Some(base) => {
                    // If successful, a contiguous block from `base` to `base + size` is allocated (set to false).
                    // Other indices remain unchanged.
                    &&& base % (1usize << align_log2) == 0
                    &&& forall|loc1: int|
                        (base <= loc1 < (base + size)) ==> self@[loc1] == false
                    &&& forall|loc2: int|
                        (0 <= loc2 < base || (base + size) <= loc2  < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2]
                },
                None => {
                    // If failed, no suitable space was found, and the state is unchanged.
                    // This implies either no free bits, or all free contiguous blocks are too small or misaligned.
                    Self::spec_cap() < (1usize << align_log2) || !self.spec_any() ||
                    // self.bits == 0 || self::spec_cap() < (1usize << align_log2) ||
                    forall|i: int| (0 <= i <= (Self::spec_cap() - size) as int) ==> has_obstruction(self@, i, size as int,(1usize << align_log2) as int)
                }
            },
    ;

    /// Free an allocated bit.
    fn dealloc(&mut self, key: usize)
        requires
            old(self).wf(),
            key < Self::spec_cap(),
            !old(self)@[key as int],
        ensures
            self@ == old(self)@.update(key as int, true),
            self.wf(),
    ;

    /// Mark bits in the range as val
    fn set_range_to(&mut self, range: Range<usize>, val: bool)
        requires
            old(self).wf(),
            range.start < Self::spec_cap(),
            range.end <= Self::spec_cap(),
            range.start < range.end,
        ensures
            forall|loc1: int|
                (range.start <= loc1 < range.end) ==> self@[loc1] == val,
            forall|loc2: int|
                (0 <= loc2 < range.start || range.end <= loc2 < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2],
            self@.len() == old(self)@.len(),
            self.wf(),
    ;

    /// Mark bits in the range as unallocated (available)
    fn insert(&mut self, range: Range<usize>)
        requires
            old(self).wf(),
            range.start < Self::spec_cap(),
            range.end <= Self::spec_cap(),
            range.start < range.end,
        ensures
            forall|loc1: int|
                (range.start <= loc1 < range.end) ==> self@[loc1] == true,
            forall|loc2: int|
                (0 <= loc2 < range.start || range.end <= loc2 < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2],
            self@.len() == old(self)@.len(),
            self.wf(),
    ;

    /// Reverse of insert
    fn remove(&mut self, range: Range<usize>)
        requires
            old(self).wf(),
            range.start < Self::spec_cap(),
            range.end <= Self::spec_cap(),
            range.start < range.end,
        ensures
            forall|loc1: int|
                (range.start <= loc1 < range.end) ==> self@[loc1] == false,
            forall|loc2: int|
                (0 <= loc2 < range.start || range.end <= loc2 < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2],
            self@.len() == old(self)@.len(),
            self.wf(),
    ;
}

/// A bitmap of 256 bits
pub type BitAlloc256 = BitAllocCascade16<BitAlloc16>; //8
/// A bitmap of 4K bits
pub type BitAlloc4K = BitAllocCascade16<BitAlloc256>; //12
/// A bitmap of 64K bits
pub type BitAlloc64K = BitAllocCascade16<BitAlloc4K>; //16
/// A bitmap of 1M bits
pub type BitAlloc1M = BitAllocCascade16<BitAlloc64K>; //20


/// Implement the bit allocator by segment tree algorithm.
#[derive(Copy)]
pub struct BitAllocCascade16<T: BitAllocView> {
    pub bitset: BitAlloc16, // for each bit, 1 indicates available, 0 indicates inavailable
    pub sub: [T; 16],
}

impl<T: BitAllocView + Copy> Clone for BitAllocCascade16<T> {
    fn clone(&self) -> Self { *self }
}

impl<T: BitAllocView + Copy> BitAllocView for BitAllocCascade16<T> {
    open spec fn view(&self) -> Seq<bool> {
        // 把 16 个子分配器的 view 拼接在一起
        let sub_len = T::spec_cap() as int;
        // let sub_len = self.sub[0]@.len() as int;
        // Seq::new((sub_len * 16) as nat, |idx: int| {
        //     let i   = idx / sub_len;                   // 第几个子分配器
        //     let off = idx % sub_len;                   // 子分配器内偏移
        //     self.sub[i as int]@[off as int]           // 取对应 bit
        // })
        Seq::new((sub_len * 16) as nat, |idx: int| {
            self.sub[idx / sub_len as int]@[idx % sub_len as int]           // 取对应 bit
        })
    }

    // 每个子分配器的容量都是固定且相等的
    fn cap() -> (res:usize) {
        (T::cap() * 16) as usize
    }

    open spec fn spec_cap() -> (res:usize){
        (T::spec_cap() * 16) as usize
    }

    open spec fn cascade_not_overflow() -> bool {
        T::cascade_not_overflow() && T::spec_cap() * 16 < 0x100000
    }

    open spec fn lemma_cap_is_pow16_pre() -> bool {
        &&& Self::spec_cap() == T::spec_cap() * 16
        &&& T::lemma_cap_is_pow16_pre()
    }

    proof fn lemma_cap_is_pow16()
    {
        assert(is_pow16(T::spec_cap())) by{
            // self.lemma_cap_is_pow16()
            T::lemma_cap_is_pow16();
        };
        assert(Self::spec_cap() == 16 * T::spec_cap());
        assert(is_pow16(Self::spec_cap())) by {
            if T::spec_cap() == 16 {
                assert(is_pow16(Self::spec_cap()));
            } else if T::spec_cap() == 256 {
                assert(is_pow16(Self::spec_cap()));
            } else if T::spec_cap() == 4096 {
                assert(is_pow16(Self::spec_cap()));
            } else {
                assert(is_pow16(Self::spec_cap()));
            }
        }
    }

    /// Creates a new `BitAllocCascade16` with all bits set to 0 (all free).
    fn default() -> Self {
        BitAllocCascade16 {
            bitset: BitAlloc16 { bits: 0 },
            sub: [T::default(); 16], // need the trait "std::marker::Copy"
        }
    }

    /// Checks if there are any free bits (bits set to 1) in the bitmap.
    fn any(&self) -> (res:bool){
        self.bitset.any()
    }

    open spec fn spec_any(&self) -> bool{
        self.bitset.spec_any()
    }

    proof fn lemma_bits_nonzero_implies_exists_true(&self){
        let cap = T::spec_cap() as int;
        if self.spec_any() {
            self.bitset.lemma_bits_nonzero_implies_exists_true();
            assert(forall|k:int|
                    0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any());
            assert(exists|k:int| 0 <= k < 16 && self.bitset@[k] == true);
            assert(exists|k:int| 0 <= k < 16 && self.sub[k].spec_any() == true);

            // 用 choose 抽出见证 k₀
            let k0 = choose|k:int| 0 <= k < 16 && self.sub[k].spec_any() == true;
            assert(0 <= k0 < 16);
            assert(self.sub[k0].spec_any() == true);

            assert(self.sub[k0].wf());

            self.sub[k0].lemma_bits_nonzero_implies_exists_true();

            // 得到：∃ j∈[0,cap). self.sub[k0]@[j] == true
            let j0 = choose|j:int| 0 <= j && j < cap && self.sub[k0]@[j] == true;
            assert(0 <= j0 < cap);
            assert(self.sub[k0]@[j0] == true);

            assert(forall|k:int, j:int|
                0 <= k < 16 && 0 <= j < cap ==> self@[(k*cap) + j] == self.sub[k]@[j]
            );

            let loc = k0 * cap + j0;

            assert(0 <= loc < (k0 + 1) * cap) by(nonlinear_arith)
                requires
                    loc == k0 * cap + j0,
                    0 <= k0 < 16,
                    0 <= j0 < cap,
                    cap >0,
            ;

            assert(loc < Self::spec_cap()) by(nonlinear_arith)
                requires
                    loc == k0 * cap + j0,
                    0 <= k0 < 16,
                    0 <= j0 < cap,
                    cap >0,
                    Self::spec_cap() == cap * 16,
            ;

            assert(self@[loc] == self.sub[k0]@[j0]);
            assert(self@[loc] == true);

            assert(exists|j:int| 0 <= j < Self::spec_cap() && self@[j] == true);
        } else {
            // self.bitset.bits == 0
            self.bitset.lemma_bits_nonzero_implies_exists_true();
            assert(forall|k:int|
                    0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any());

            assert(forall|k:int| 0 <= k < 16 ==> self.bitset@[k] == false);
            assert forall|k:int| 0 <= k < 16 implies self.sub[k].spec_any() == false by{
                assert(self.bitset@[k] == self.sub[k].spec_any());
                assert(self.bitset@[k] == false);
            };

            assert forall|k:int, j:int|
                0 <= k < 16 && 0 <= j < cap implies self.sub[k]@[j] == false by{
                    assert(self.sub[k].spec_any() == false);
                    self.sub[k].lemma_bits_nonzero_implies_exists_true();
            };

            assert forall|loc2:int| 0 <= loc2 < Self::spec_cap() implies self@[loc2] == false by{
                let k = loc2 / cap;
                let j = loc2 % cap;

                assert(0 <= j < cap);

                assert(0 <= k < 16 ) by(nonlinear_arith)
                    requires
                        k == loc2 / cap,
                        0 <= loc2 < Self::spec_cap(),
                        Self::spec_cap() == 16*cap,
                ;
                assert(loc2 == k*cap + j) by(nonlinear_arith)
                    requires
                        k == loc2 / cap,
                        j == loc2 % cap,
                        0 <= loc2 < Self::spec_cap(),
                        Self::spec_cap() == 16*cap,
                ;

                assert(self@[k*cap + j] == self.sub[k]@[j]);
                assert(self.sub[k]@[j] == false);

                assert(self@[loc2] == false);
            }
        }
        assert(self.spec_any() == exists|j:int| 0 <= j < Self::spec_cap() && self@[j] == true);
    }

    /// Tests if a specific bit at `index` is free (1) or allocated (0).
    fn test(&self, key: usize) -> (res:bool)
    {
        let seq_index: usize = key / T::cap(); //证明seq_index < 16

        assert(seq_index < 16) by(nonlinear_arith)
            requires
                seq_index == key / T::spec_cap(),
                Self::spec_cap() == T::spec_cap() * 16,
                key < Self::spec_cap(),
                key < (T::spec_cap() * 16),
                T::spec_cap() > 0,
        ;

        let bit_index: usize = key % T::cap();
        let res = self.sub[seq_index].test(bit_index);
        assert(res == self.sub[seq_index as int]@[bit_index as int]);

        res
    }

    open spec fn wf(&self) -> bool {
        let cap = T::spec_cap() as int;
        &&& Self::cascade_not_overflow()
        &&& Self::lemma_cap_is_pow16_pre()
        // &&& T::spec_cap() > 0
        &&& self.sub.len() == 16
        &&& forall|k:int| 0 <= k < 16 ==> self.sub[k]@.len() == cap
        &&& forall|k:int| 0 <= k < 16 ==> self.sub[k].wf()
        // 父层 bitset 的第 i 位，等价于“子分配器存在可用位”
        &&& forall|k:int|
                0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any()
        // 父层大bool序列与子序列的映射关系
        &&& forall|k:int| 0 <= k < 16 ==> view_index_mapping(self@, k, self.sub[k]@, cap)
    }

    /// Finds the next free bit (1) starting from `key` (inclusive).
    /// Returns `Some(index)` of the next free bit, or `None` if no free bits are found.
    fn next(&self, key: usize) -> (res: Option<usize>){
        let idx: usize = key / T::cap();
        assert(self.wf());

        assert(0 <= idx < 16) by(nonlinear_arith)
            requires
                idx == key / T::spec_cap(),
                Self::spec_cap() == T::spec_cap() * 16,
                key < Self::spec_cap(),
                key < (T::spec_cap() * 16),
                T::spec_cap() > 0,
        ;

        let mut i = idx;
        assert(T::spec_cap() * idx < Self::spec_cap()) by(nonlinear_arith)
            requires
                Self::spec_cap() == T::spec_cap() * 16,
                0 <= idx < 16,
                T::spec_cap() > 0,
        ;

        let mut result: Option<usize> = None;
        let mut curr_key = T::cap() * idx;
        assert(curr_key <= key) by(nonlinear_arith)
            requires
                curr_key == T::spec_cap() * idx,
                idx == key / T::spec_cap(),
        ;
        while i < 16
            invariant_except_break
                result.is_none(),
                curr_key == T::spec_cap() * i,
            invariant
                0 <= i <= 16,
                0 <= idx < 16,
                i >= idx,
                Self::cascade_not_overflow(),
                T::spec_cap() * 16 == Self::spec_cap(),
                curr_key <= Self::spec_cap(),
                idx == key / T::spec_cap(),
                key < Self::spec_cap(),
                self.wf(),
                forall|k: int|
                    key <= k < curr_key ==> self@[k] == false,
                // “已检查范围均为 false”的循环不变量，可能存在self.bitset@[k] == true,但是从key开始的值都为false
            ensures
                (i == 16 && result.is_none() && curr_key == Self::spec_cap()) || (i < 16 && result.is_some() && (result.unwrap() == curr_key as usize) && curr_key < Self::spec_cap() && curr_key >= key && self@[result.unwrap() as int] == true),
            decreases
                16 - i,
        {
            if self.bitset.get_bit(i as u16) {
                let base_key = if i == idx {
                    assert(0 <= (key - T::spec_cap() * idx) < T::spec_cap()) by(nonlinear_arith)
                        requires
                            idx == key / T::spec_cap(),
                            Self::spec_cap() == T::spec_cap() * 16,
                            key < Self::spec_cap(),
                            key < (T::spec_cap() * 16),
                            T::spec_cap() > 0,
                    ;
                    key - T::cap() * idx
                } else {
                    0
                };
                // 这里要先保证16叉树成型！
                let child = self.sub[i].next(base_key);
                if child.is_some() {
                    assert(i<16);
                    assert(forall|t: int| base_key <= t < child.unwrap() as int ==> self.sub[i as int]@[t] == false);

                    assert(self.sub[i as int]@[child.unwrap() as int] == true);

                    curr_key = T::cap() * i + child.unwrap();

                    assert(curr_key < Self::spec_cap()) by(nonlinear_arith)
                        requires
                            curr_key == T::spec_cap() * i + child.unwrap(),
                            child.unwrap() < T::spec_cap(),
                            Self::spec_cap() == T::spec_cap() * 16,
                            i < 16,
                    ;
                    assert(curr_key >= key) by(nonlinear_arith)
                        requires
                            idx == key / T::spec_cap(),
                            i >= idx,
                            0 <= child.unwrap() < T::spec_cap(),
                            curr_key == T::spec_cap() * i + child.unwrap(),
                            i == idx ==> child.unwrap() >= key - T::spec_cap() * idx,
                    ;

                    assert(self@[curr_key as int] == true);

                    proof{
                        lemma_view_indexs_st_to_ed_mapping_false(self@, i as int, self.sub[i as int]@, T::spec_cap() as int, base_key as int, child.unwrap() as int)
                    }
                    assert(forall|t: int| (T::spec_cap() * i + base_key) as int <= t < curr_key as int ==> self@[t] == false);

                    result = Some(curr_key);
                    assert(forall|t: int| key <= t < curr_key as int ==> self@[t] == false);
                    break;
                } else {
                    assert(forall|t: int| base_key <= t < T::spec_cap() as int ==> self.sub[i as int]@[t] == false);
                    proof{
                        lemma_view_indexs_st_to_ed_mapping_false(self@, i as int, self.sub[i as int]@, T::spec_cap() as int, base_key as int, T::spec_cap() as int)
                    }
                    assert(forall|t: int| (T::spec_cap() * i + base_key) as int <= t < (T::spec_cap() * i + T::spec_cap()) as int ==> self@[t] == false);
                }
            } else {
                assert(self.bitset@[i as int] == false);
                assert(self.sub[i as int].spec_any() == false);
                proof{
                    self.sub[i as int].lemma_bits_nonzero_implies_exists_true();
                }
                assert(forall|t: int| 0 <= t < T::spec_cap() as int ==> self.sub[i as int]@[t] == false);
                proof{
                    lemma_view_indexs_st_to_ed_mapping_false(self@, i as int, self.sub[i as int]@, T::spec_cap() as int, 0, T::spec_cap() as int)
                }
                assert(forall|t: int| (T::spec_cap() * i + 0) as int <= t < (T::spec_cap() * i + T::spec_cap()) as int ==> self@[t] == false);
            }
            assert(forall|t: int| key as int <= t < (T::spec_cap() * i + T::spec_cap()) as int ==> self@[t] == false);
            let old_i = i;
            assert(T::spec_cap() * (old_i + 1) <= Self::spec_cap()) by(nonlinear_arith)
                requires
                    i < 16,
                    old_i == i,
                    Self::spec_cap() == T::spec_cap() * 16,
            ;
            let next_end = T::cap() * (old_i + 1);
            assert(T::spec_cap() * (old_i + 1) == (T::spec_cap() * i + T::spec_cap())) by(nonlinear_arith)
                requires
                    old_i == i,
            ;
            assert(forall|t: int| key as int <= t < next_end as int ==> self@[t] == false);
            i += 1;
            assert(i == old_i + 1);
            assert(T::spec_cap() * i <= Self::spec_cap()) by(nonlinear_arith)
                requires
                    i <= 16,
                    Self::spec_cap() == T::spec_cap() * 16,
            ;
            curr_key = T::cap() * i;
            assert(curr_key == T::spec_cap() * (old_i + 1));
            assert(curr_key == next_end);
            assert(forall|t: int| key <= t < curr_key as int ==> self@[t] == false);
        }
        result
    }
}


pub open spec fn view_index_mapping(ba: Seq<bool>, i: int, sub_ba: Seq<bool>, cap: int) -> bool {
    forall|j:int| 0 <= j < cap ==> ba[(cap * i + j)] == sub_ba[j]
}

impl<T: BitAlloc + Copy> BitAlloc for BitAllocCascade16<T>{

    fn alloc(&mut self) -> (res:Option<usize>)
    {
        let ghost cap= T::spec_cap() as int;
        if !self.any() {
            assert(forall|j:int| 0 <= j < 16 ==> self.bitset@[j] == false);
            assert forall|j:int| 0 <= j < 16 implies no_available(self.sub[j]@,cap) by{
                assert(self.bitset@[j] == false);
                self.sub[j].lemma_bits_nonzero_implies_exists_true();
            }
            assert(forall|j:int| 0 <= j < 16 ==> view_index_mapping(self@,j,self.sub[j]@,cap));
            assert forall|loc2:int| 0 <= loc2 < 16*cap implies self@[loc2] == false by{
                let j = loc2 / cap;
                let k = loc2 % cap;

                assert(0 <= k < cap);

                assert(0 <= j < 16) by(nonlinear_arith)
                    requires
                        j == loc2 / cap,
                        0 <= loc2 < 16*cap,
                ;

                assert(self@[j*cap + k] == self.sub[j]@[k]);
                assert(loc2 == j*cap + k) by(nonlinear_arith)
                    requires
                        j == loc2 / cap,
                        k == loc2 % cap,
                        0 <= loc2 < 16*cap,
                ;

                assert(no_available(self.sub[j]@, cap));
                assert(self.sub[j]@[k] == false);

                assert(self@[loc2] == false);
            };
            return None;
        }
        // Find the first free bit (least significant 1-bit).
        let i = self.bitset.bits.trailing_zeros() as usize;
        assert(i < u16::BITS) by{
            assert(self.bitset.bits != 0); // Prove that if self.bits != 0, trailing_zeros() < 16.
        };
        assert(self.bitset@[i as int] == get_bit16!(self.bitset.bits, i));

        // 证明第i位为空闲
        proof{
            vstd::std_specs::bits::axiom_u16_trailing_zeros(self.bitset.bits);
        }
        assert(get_bit16!(self.bitset.bits, i) == true);
        assert(self.bitset@[i as int] == true);
        proof{
            self.sub[i as int].lemma_bits_nonzero_implies_exists_true();
        }

        assert(exists|j:int| 0 <= j < T::spec_cap() && self.sub[i as int]@[j] == true);
        assert(forall|j:int| 0 <= j < i ==> self.bitset@[j] == false);

        // 证明父亲的bits为false，儿子全为false
        // j<i 的每个子分配器全 false
        assert forall|j:int| 0 <= j < i implies no_available(self.sub[j]@,cap) by{
            assert(self.bitset@[j] == false);
            self.sub[j].lemma_bits_nonzero_implies_exists_true();
        };
        assert forall|loc2:int| 0 <= loc2 < i*cap implies self@[loc2] == false by{
            let j = loc2 / cap;
            let k = loc2 % cap;

            assert(0 <= k < cap);

            assert(0 <= j < i) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    0 <= loc2 < i*cap,
            ;

            assert(self@[j*cap + k] == self.sub[j]@[k]);
            assert(loc2 == j*cap + k) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    k == loc2 % cap,
                    0 <= loc2 < i*cap,
            ;

            assert(no_available(self.sub[j]@, cap));
            assert(self.sub[j]@[k] == false);

            assert(self@[loc2] == false);
        };

        // 开始改值，调用子分配器的alloc
        let mut child = self.sub[i];
        let res_is_some = child.alloc();

        // assert(forall|loc2:int| (0 <= loc2 < i*T::spec_cap() || (i+1)*T::spec_cap()<= loc2< Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2]);

        self.sub[i] = child;

        assert(child.wf());
        assert(res_is_some.unwrap() + i * T::spec_cap() < Self::spec_cap()) by(nonlinear_arith)
            requires
                Self::spec_cap() == T::spec_cap() * 16,
                0 <= i < 16,
                T::spec_cap() > 0,
                res_is_some.unwrap() < T::spec_cap(),
        ;
        let res = res_is_some.unwrap() + i * T::cap();

        let bv_old: u16 = self.bitset.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, i, self.sub[i].any());
        proof {
            set_bit_u16_preserves_others(bv_new, bv_old, i as u16, self.sub[i as int].spec_any());
        }
        ;
        self.bitset.bits = bv_new;

        //改完值后确保仍然保持 wellformed
        assert(self.bitset@[i as int] == self.sub[i as int].spec_any());

        assert(forall|k:int| 0 <= k < 16 ==> self.sub[k].wf());

        assert(forall|j:int| 0 <= j < 16 && j!=i ==> self.bitset@[j] == old(self).bitset@[j]);
        assert(forall|k:int|
                0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any());

        assert(forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> self@[loc2] == self.sub[loc2 / cap]@[loc2 % cap]);

        assert(forall|j:int| 0 <= j < 16 && j!=i ==> self.sub[j]@ == old(self).sub[j]@);


        assert(self.sub[i as int]@ == old(self).sub[i as int]@.update(res_is_some.unwrap() as int, false));

        // 证明大bool序列改了的那一位
        assert((i*cap + res_is_some.unwrap()) / cap == i) by(nonlinear_arith)
            requires
                res_is_some.unwrap() + i * cap < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                res_is_some.unwrap() < cap,
        ;
        assert((i*cap + res_is_some.unwrap()) % cap == res_is_some.unwrap()) by(nonlinear_arith)
            requires
                res_is_some.unwrap() + i * cap < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                res_is_some.unwrap() < cap,
        ;
        assert(self@[(i*cap + res_is_some.unwrap()) as int] == self.sub[i as int]@[res_is_some.unwrap() as int]) by(nonlinear_arith)
            requires
                (i*cap + res_is_some.unwrap()) / cap == i,
                (i*cap + res_is_some.unwrap()) % cap == res_is_some.unwrap(),
                forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> self@[loc2] == self.sub[loc2 / cap]@[loc2 % cap],
                res_is_some.unwrap() + i * cap < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                res_is_some.unwrap() < cap,
        ;
        assert(self@[res as int] == self.sub[i as int]@[res_is_some.unwrap() as int]);

        // 证明大bool序列其他位没有变
        assert forall|loc2:int| (0 <= loc2 < Self::spec_cap() && loc2 != (i*cap + res_is_some.unwrap()) as int) implies self@[loc2] == old(self)@[loc2] by{
            let j = loc2 / cap;
            let k = loc2 % cap;

            assert(0 <= k < cap);

            assert(0 <= j < 16 ) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    0 <= loc2 < Self::spec_cap() && loc2 != (i*cap + res_is_some.unwrap()) as int,
                    Self::spec_cap() == 16*cap,
            ;
            assert(loc2 == j*cap + k) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    k == loc2 % cap,
                    0 <= loc2 < Self::spec_cap() && loc2 != (i*cap + res_is_some.unwrap()) as int,
                    Self::spec_cap() == 16*cap,
            ;

            assert(self@[j*cap + k] == self.sub[j]@[k]);
            assert(old(self).sub[j]@[k] == self.sub[j]@[k]);

            assert(self@[loc2] == old(self)@[loc2]);
        };

        // 到此证完大bool序列只修改了新分配的那一位，其他位没有修改
        proof {
            assert_seqs_equal!(
                self@,
                old(self)@.update(res as int, false)
            );
        }

        // 证明原有大bool序列从0到res都为false
        assert(forall |j: int| 0 <= j < res_is_some.unwrap() as int ==> old(self).sub[i as int]@[j] == false);
        assert forall|loc2:int| i*cap <= loc2 < (i*cap + res_is_some.unwrap()) as int implies old(self)@[loc2] == false by{
            assert(old(self)@[loc2] == old(self).sub[i as int]@[loc2 - (i*cap) as int]) by{

                assert(i == loc2 / cap) by(nonlinear_arith)
                    requires
                        i*cap <= loc2 < (i*cap + res_is_some.unwrap()) as int,
                        res_is_some.unwrap() + i * cap < Self::spec_cap(),
                        Self::spec_cap() == cap * 16,
                        0 <= i < 16,
                        cap > 0,
                        res_is_some.unwrap() < cap,
                ;
                assert(loc2 % cap == loc2 - (i*cap) as int) by(nonlinear_arith)
                    requires
                        i*cap <= loc2 < (i*cap + res_is_some.unwrap()) as int,
                        res_is_some.unwrap() + i * cap < Self::spec_cap(),
                        Self::spec_cap() == cap * 16,
                        0 <= i < 16,
                        cap > 0,
                        res_is_some.unwrap() < cap,
                ;
            }
            assert(forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> old(self)@[loc2] == old(self).sub[loc2 / cap]@[loc2 % cap]);
        };

        assert(self@[res as int] == false);

        // 证明更新后任然保持view_index_mapping
        assert forall|j:int| 0 <= j < 16 implies view_index_mapping(self@,j,self.sub[j]@,cap) by{
            self.lemma_maintain_view_indexs_mapping();
        };

        Some(res as usize)

    }

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> (res: Option<usize>)
    {
        assert(Self::spec_cap() % 16 == 0) by(nonlinear_arith)
            requires
                Self::spec_cap() == 16 * T::spec_cap(),
                T::spec_cap()>0,
        ;

        assert(is_pow16(Self::spec_cap())) by{
            Self::lemma_cap_is_pow16();
        };
        if let Some(base) = find_contiguous(self, Self::cap(), size, align_log2) {
            let start = base;
            let end = base + size;
            self.remove(start..end);
            Some(base)
        } else {
            None
        }
    }

    fn dealloc(&mut self, key: usize)
    {
        let i: usize = key / T::cap(); //i < 16
        let ghost cap = T::spec_cap() as int;
        assert(i < 16) by(nonlinear_arith)
            requires
                i == key / T::spec_cap(),
                Self::spec_cap() == T::spec_cap() * 16,
                key < Self::spec_cap(),
                key < (T::spec_cap() * 16),
                T::spec_cap() > 0,
        ;

        let bit_index: usize = key % T::cap();

        let mut child = self.sub[i];
        child.dealloc(bit_index);
        assert(child@[bit_index as int]);
        assert(child.spec_any()) by{
            child.lemma_bits_nonzero_implies_exists_true();
        };

        self.sub[i] = child;
        self.bitset.set_bit(i as u16, true);
        //改完值后确保仍然保持 wellformed
        assert(self.bitset@[i as int] == self.sub[i as int].spec_any());
        assert(forall|k:int| 0 <= k < 16 ==> self.sub[k].wf());
        assert(forall|j:int| 0 <= j < 16 && j!=i ==> self.bitset@[j] == old(self).bitset@[j]);
        assert(forall|k:int|
                0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any());
        assert(forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> self@[loc2] == self.sub[loc2 / cap]@[loc2 % cap]);

        assert(forall|j:int| 0 <= j < 16 && j!=i ==> self.sub[j]@ == old(self).sub[j]@);

        assert(self.sub[i as int]@ == old(self).sub[i as int]@.update(bit_index as int, true));

        assert(self.sub[i as int]@[bit_index as int]);

        assert(bit_index + i * cap < Self::spec_cap()) by(nonlinear_arith)
            requires
                bit_index == key % T::spec_cap(),
                bit_index < cap,
                key < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
        ;
        // 证明大bool序列改了的那一位
        assert((i*cap + bit_index) / cap == i) by(nonlinear_arith)
            requires
                bit_index + i * cap < Self::spec_cap(),
                key < Self::spec_cap(),
                bit_index == key % T::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                bit_index < cap,
                cap == T::spec_cap(),
        ;
        assert((i*cap + bit_index) % cap == bit_index) by(nonlinear_arith)
            requires
                bit_index + i * cap < Self::spec_cap(),
                key < Self::spec_cap(),
                bit_index == key % T::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                bit_index < cap,
                cap == T::spec_cap(),
        ;
        assert(self@[(i*cap + bit_index) as int] == self.sub[i as int]@[bit_index as int]) by(nonlinear_arith)
            requires
                (i*cap + bit_index) / cap == i,
                (i*cap + bit_index) % cap == bit_index,
                forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> self@[loc2] == self.sub[loc2 / cap]@[loc2 % cap],
                bit_index + i * cap < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                0 <= i < 16,
                cap > 0,
                bit_index < cap,
        ;
        assert(self@[key as int] == self.sub[i as int]@[bit_index as int]);
        
        assert(key == i*cap + bit_index) by(nonlinear_arith)
            requires
                i == key / T::spec_cap(),
                bit_index == key % T::spec_cap(),
                (i*cap + bit_index) / cap == i,
                (i*cap + bit_index) % cap == bit_index,
                bit_index + i * cap < Self::spec_cap(),
                Self::spec_cap() == cap * 16,
                cap == T::spec_cap(),
                0 <= i < 16,
                cap > 0,
                bit_index < cap,
        ;


        // 证明大bool序列其他位没有变
        assert forall|loc2:int| (0 <= loc2 < Self::spec_cap() && loc2 != key as int) implies self@[loc2] == old(self)@[loc2] by{
            let j = loc2 / cap;
            let k = loc2 % cap;

            assert(0 <= k < cap);

            assert(0 <= j < 16 ) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    0 <= loc2 < Self::spec_cap() && loc2 != key as int,
                    Self::spec_cap() == 16*cap,
            ;
            assert(loc2 == j*cap + k) by(nonlinear_arith)
                requires
                    j == loc2 / cap,
                    k == loc2 % cap,
                    0 <= loc2 < Self::spec_cap() && loc2 != key as int,
                    Self::spec_cap() == 16*cap,
            ;            

            assert(self@[j*cap + k] == self.sub[j]@[k]);
            assert(old(self).sub[j]@[k] == self.sub[j]@[k]);

            assert(self@[loc2] == old(self)@[loc2]);
        };

        // 到此证完大bool序列只修改了新分配的那一位，其他位没有修改
        proof {
            assert_seqs_equal!(
                self@,
                old(self)@.update(key as int, true)
            );
        }
        // 证明更新后任然保持view_index_mapping
        assert forall|j:int| 0 <= j < 16 implies view_index_mapping(self@,j,self.sub[j]@,cap) by{
            self.lemma_maintain_view_indexs_mapping();
        }
    }

    fn set_range_to(&mut self, range: Range<usize>, val: bool)
    {
        let st = range.start;
        let ed = range.end;

        let first = st / T::cap(); //首个子分配器
        let last = (ed - 1) / T::cap(); //末尾子分配器
        let n = last + 1; //结束循环条件

        let mut i = first;

        let mut current_end = st;

        assert(0 <= i <= last) by (nonlinear_arith)
            requires
                i == first,
                first == st / T::spec_cap(),
                last == (ed - 1) / T::spec_cap() as int,
                st < ed,
                T::spec_cap() > 0,
        ;
        assert(n <= 16) by (nonlinear_arith)
            requires
                n == last + 1,
                last == (ed - 1) / T::spec_cap() as int,
                ed <= Self::spec_cap(),
                Self::spec_cap() == 16 * T::spec_cap(),
                T::spec_cap() > 0,
        ;

        while i < n
            invariant
                0 <= i <= n,
                n <= 16,

                i >= first,
                first == st / T::spec_cap(),
                last == (ed - 1) / T::spec_cap() as int,
                n == last +1,
                st < ed,
                ed <= Self::spec_cap(),
                T::spec_cap() > 0,

                current_end == if i == st / T::spec_cap() {st} else if i == n as int { ed } else {(i * T::spec_cap()) as usize},
                forall|loc1: int| st <= loc1 < current_end ==> self@[loc1] == val,
                forall|loc2: int|
                    (0 <= loc2 < st || current_end <= loc2 < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2],
                forall|loc3: int|
                    (0 <= loc3 < first || i <= loc3 < 16) ==> self.sub[loc3]@ == old(self).sub[loc3]@,
                self@.len() == old(self)@.len(),
                self.wf(),
            decreases
                n - i,
        {
            let begin = if i == st / T::cap() {
                st % T::cap()
            } else {
                0
            };
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
            let ghost old_child = child;
            let ghost cap = T::spec_cap() as int;

            // 分情况讨论证明
            assert(begin < stop) by{
                if i == st / T::spec_cap(){
                    assert(begin == st % T::spec_cap());
                    if i == (ed - 1) / T::spec_cap() as int {
                        if ed % T::spec_cap() == 0 {
                            assert(stop == T::spec_cap());
                            assert(begin < stop);
                        } else {
                            assert(begin < stop) by (nonlinear_arith)
                                requires
                                    begin == st % T::spec_cap(),
                                    stop  == ed % T::spec_cap(),
                                    i == st / T::spec_cap(),
                                    i == (ed - 1) / T::spec_cap() as int,
                                    ed % T::spec_cap() != 0,
                                    i < 16,
                                    st < ed,
                                    cap == T::spec_cap(),
                                    ed <= Self::spec_cap(),
                                    Self::spec_cap() == 16 * cap,
                                    cap > 0,
                            ;
                        }
                        assert(begin < stop);
                    } else {
                        assert(stop == T::spec_cap());
                        assert(begin < stop);
                    }
                    assert(begin < stop);
                } else {
                    assert(begin == 0);
                    if i == (ed - 1) / T::spec_cap() as int {
                        if ed % T::spec_cap() == 0 {
                            assert(stop == T::spec_cap());
                            assert(begin < stop);
                        } else {
                            assert(stop == ed % T::spec_cap());
                            assert(st < ed);
                        }
                        assert(begin < stop);
                    } else {
                        assert(stop == T::spec_cap());
                        assert(begin < stop);
                    }
                    assert(begin < stop);
                }
            }

            // 修改了子分配器
            if val {
                child.insert(begin..stop);
            } else {
                child.remove(begin..stop);
            }

            let ghost pre_self = self@;
            assert(forall|loc1: int| 0 <= loc1 < Self::spec_cap() ==> pre_self[loc1] == self.sub[loc1 / cap as int]@[loc1 % cap as int]);
            assert forall|k:int,j:int| 0 <= k < 16 && 0<=j<cap implies self@[k*cap + j] == self.sub[k]@[j] by{
                let loc1=k*cap+j;
                assert(k==loc1/cap as int) by(nonlinear_arith)
                    requires
                        loc1 == k*cap+j,
                        cap>0,
                        0<=j<cap,
                ;
                assert(j==loc1%cap as int) by(nonlinear_arith)
                    requires
                        loc1 == k*cap+j,
                        cap>0,
                        0<=j<cap,
                ;
            };
            assert(forall|k:int,j:int| 0 <= k < 16 && 0<=j<cap ==> pre_self[k*cap + j] == self.sub[k]@[j]);

            //证明(i+1)*cap到Self::spec_cap()的pre_self[loc1]==old(self)@[loc1]
            assert(forall|loc2: int|
                (0 <= loc2 < st || current_end <= loc2 < Self::spec_cap()) ==> pre_self[loc2] == old(self)@[loc2]);
            assert(current_end == (begin + i * cap)) by(nonlinear_arith)
                requires
                    i < 16,
                    st < ed,
                    cap == T::spec_cap(),
                    ed <= Self::spec_cap(),
                    Self::spec_cap() == 16 * cap,
                    ed > 0,
                    cap > 0,
                    begin == (if i == st / T::spec_cap() { st % T::spec_cap() } else { 0 }),
                    current_end == if i == st / T::spec_cap() {st} else {(i * cap) as usize},
            ;
            assert(0 <= begin < cap) by(nonlinear_arith)
                requires
                    0 <= i < 16,
                    begin == (if i == st / T::spec_cap() { st % T::spec_cap() } else { 0 }),
                    T::spec_cap() == cap,
                    cap > 0,
            ;
            assert forall|loc1: int| (i+1)*cap <= loc1 < Self::spec_cap() implies pre_self[loc1] == old(self)@[loc1] by{
                assert(current_end<(i+1)*cap) by(nonlinear_arith)
                    requires
                        current_end == (begin + i * cap),
                        0 <= begin < cap,
                        cap>0,
                ;
                assert(forall|loc2: int|
                    (current_end <= loc2 < Self::spec_cap()) ==> pre_self[loc2] == old(self)@[loc2]);
                assert(pre_self[loc1] == old(self)@[loc1]);
            };

            self.sub[i] = child;// i*cap -- (i+1)*cap        begin - stop
            self.bitset.set_bit(i as u16, self.sub[i].any());

            assert(forall|loc1: int| st <= loc1 < current_end ==> pre_self[loc1] == val);

            //改完值后确保仍然保持 wellformed
            assert(self.bitset@[i as int] == self.sub[i as int].spec_any());
            assert(forall|k:int| 0 <= k < 16 ==> self.sub[k].wf());

            assert(forall|k:int|
                    0 <= k < 16 ==> self.bitset@[k] == self.sub[k].spec_any());

            // 证明更新后任然保持view_index_mapping
            assert forall|j:int| 0 <= j < 16 implies view_index_mapping(self@,j,self.sub[j]@,cap as int) by{
                self.lemma_maintain_view_indexs_mapping();
            }
            assert(forall|loc1: int| 0 <= loc1 < Self::spec_cap() ==> self@[loc1] == self.sub[loc1 / cap as int]@[loc1 % cap as int]);
            assert(self@.len() == old(self)@.len());
            assert(self.wf());

            //证明self.sub[i] = child;只修改了子分配器i范围内的值
            assert(forall|k:int,j:int| 0<=k <16 && k!=i && 0<=j<cap ==> pre_self[k*cap + j] == self.sub[k]@[j]);

            assert forall|loc1: int| 0 <= loc1 < i*cap || (i+1)*cap <= loc1 < Self::spec_cap() implies pre_self[loc1] == self.sub[loc1 / cap as int]@[loc1 % cap as int] by{
                let k=loc1/cap;
                let j=loc1%cap;

                assert(0 <= k < 16 && k != i) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        k == loc1/cap,
                        0 <= loc1 < i*cap || (i+1)*cap <= loc1 < Self::spec_cap(),
                        Self::spec_cap() == 16*cap,
                        cap>0,
                ;

                assert(0<=j<cap) by(nonlinear_arith)
                    requires
                        j == loc1%cap,
                        0 <= loc1 < i*cap || (i+1)*cap <= loc1 < Self::spec_cap(),
                        Self::spec_cap() == 16*cap,
                        cap>0,
                ;

                assert(pre_self[k*cap + j] == self.sub[k]@[j]);
                assert(loc1 == k*cap + j) by(nonlinear_arith)
                    requires
                        k == loc1/cap,
                        j == loc1%cap,
                        cap>0,
                ;
                assert(pre_self[loc1] == self.sub[k]@[j]);
            };

            assert forall|loc1: int| 0 <= loc1 < i*cap || (i+1)*cap <= loc1 < Self::spec_cap() implies self@[loc1] == self.sub[loc1 / cap as int]@[loc1 % cap as int] by{
                assert(0<=i*cap<Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0<=i<16,
                        cap>0,
                        Self::spec_cap()==16*cap,
                ;
                assert(0<(i+1)*cap<=Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0<=i<16,
                        cap>0,
                        Self::spec_cap()==16*cap,
                ;
            };
            assert(forall|loc1: int| 0 <= loc1 < i*cap || (i+1)*cap <= loc1 < Self::spec_cap() ==> self@[loc1] == pre_self[loc1]);

            //证明从st到还未更新的current_end（即当前子分配器的上边界）都为false
            assert forall|loc1: int| st <= loc1 < current_end implies self@[loc1] == val by{
                if i==first{
                    assert(self@[loc1] == val);
                }else {
                    assert(current_end == i*cap);
                    assert(forall|loc2: int| 0 <= loc2 < i*cap ==> self@[loc2] == pre_self[loc2]);
                    assert(forall|loc2: int| st <= loc2 < current_end ==> pre_self[loc2] == val);
                    assert(0 <= st < i*cap) by(nonlinear_arith)
                        requires
                            i>first,
                            first==st/T::spec_cap(),
                            cap == T::spec_cap(),
                            cap>0,
                            0 <= st,
                    ;
                    assert(forall|loc2: int| st <= loc2 < current_end ==> self@[loc2] == pre_self[loc2]);
                    assert(pre_self[loc1] == val);
                    assert(self@[loc1] == val);
                }
            };

            // 证明current_end不会overflow
            assert(stop + i * cap <= Self::spec_cap()) by (nonlinear_arith)
                requires
                    stop == (if i == (ed - 1) / T::spec_cap() as int { if ed % T::spec_cap() == 0 { T::spec_cap() } else {
                            ed % T::spec_cap() }} else { T::spec_cap() }),
                    0<=i<16,
                    Self::spec_cap() == 16 * cap,
                    cap == T::spec_cap(),
                    cap >0,
            ;
            current_end = stop + i * T::cap();

            // 证明父子分配器按要求修改了值，并保持映射关系
            assert(forall|loc1: int|
                (begin <= loc1 < stop) ==> child@[loc1] == val);
            assert(forall|loc1: int|
                (begin <= loc1 < stop) ==> self.sub[i as int]@[loc1] == val);

            assert forall|loc1: int|
                ((begin + i * cap) <= loc1 < (stop + i * cap)) implies self@[loc1] == self.sub[i as int]@[loc1 - (i*cap) as int] by{
                    assert(i == loc1 / cap as int) by(nonlinear_arith)
                        requires
                            (begin + i * cap) <= loc1 < (stop + i * cap),
                            stop <= cap,
                            Self::spec_cap() == cap * 16,
                            stop + i * cap <= Self::spec_cap(),
                            0 <= i < 16,
                            cap > 0,
                    ;
                    assert(loc1 % cap as int == loc1 - (i*cap) as int) by(nonlinear_arith)
                        requires
                            (begin + i * cap) <= loc1 < (stop + i * cap),
                            stop + i * cap <= Self::spec_cap(),
                            Self::spec_cap() == cap * 16,
                            0 <= i < 16,
                            cap > 0,
                            stop <= cap,
                    ;
                };

            assert forall|loc1: int| ((begin + i * cap) <= loc1 < (stop + i * cap)) implies self@[loc1] == val by{
                assert(forall|k: int|
                    (begin <= k < stop) ==> self.sub[i as int]@[k] == val);
            };

            assert(forall|loc1: int| st <= loc1 < current_end ==> self@[loc1] == val);

            // 证明改完值后其他位值的值没有改变
            assert(forall|loc1: int|
                (0 <= loc1 < begin || stop <= loc1 < T::spec_cap()) ==> child@[loc1] == old_child@[loc1]);

            assert(forall|loc1: int|
                (0 <= loc1 < begin || stop <= loc1 < T::spec_cap()) ==> self.sub[i as int]@[loc1] == old_child@[loc1]);

            assert(forall|loc1: int|
                (0 <= loc1 < begin || stop <= loc1 < T::spec_cap()) ==> self.sub[i as int]@[loc1] == old(self).sub[i as int]@[loc1]);

            //先证明0~st没变
            assert forall|loc2:int| (0+i*cap <= loc2 < begin+i*cap) implies self@[loc2] == self.sub[i as int]@[loc2 % cap as int] by{
                assert(begin+i*cap <= Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        0 <= begin < cap,
                        Self::spec_cap() == 16 *cap,
                        cap > 0,
                ;
                assert(i == loc2 / cap as int) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        (0+i*cap <= loc2 < begin+i*cap),
                        begin == (if i == st / T::spec_cap() { st % T::spec_cap() } else { 0 }),
                        cap == T::spec_cap(),
                        cap >0,
                ;
            };

            assert forall|loc2:int| (0+i*cap <= loc2 < begin+i*cap) implies self@[loc2] == old(self).sub[i as int]@[loc2 % cap as int] by{
                let j=loc2%cap;
                assert(0 <= j < begin) by(nonlinear_arith)
                    requires
                        j==loc2%cap,
                        cap>0,
                        (0+i*cap <= loc2 < begin+i*cap),
                ;
            };
            assert(forall|loc1: int| 0 <= loc1 < Self::spec_cap() ==> old(self)@[loc1] == old(self).sub[loc1 / cap as int]@[loc1 % cap as int]);
            assert forall|loc2:int| (0+i*cap <= loc2 < begin+i*cap) implies self@[loc2] == old(self)@[loc2] by{
                let k=loc2/cap;
                let j=loc2%cap;
                assert(k == i) by(nonlinear_arith)
                    requires
                        k==loc2/cap,
                        cap>0,
                        0+i*cap <= loc2 < begin+i*cap,
                        0 <= begin < cap,
                ;
                assert(0 <= j < begin) by(nonlinear_arith)
                    requires
                        j==loc2%cap,
                        cap>0,
                        (0+i*cap <= loc2 < begin+i*cap),
                ;
                assert(self@[loc2] == old(self).sub[i as int]@[loc2 % cap as int]);
                assert(old(self)@[loc2] == old(self).sub[loc2 / cap as int]@[loc2 % cap as int]);
                assert(self@[loc2] == old(self)@[loc2]);
            };
            assert(forall|loc2: int|
                (0 <= loc2 < st || current_end <= loc2 < Self::spec_cap()) ==> pre_self[loc2] == old(self)@[loc2]);

            assert forall|loc2: int| (0 <= loc2 < st) implies self@[loc2] == old(self)@[loc2] by{
                if i==first{
                    assert(forall|loc1: int| 0 <= loc1 < i*cap  ==> self@[loc1] == old(self)@[loc1]);
                    assert(forall|loc1:int| (0+i*cap <= loc1 < begin + i*cap) ==> self@[loc1] == old(self)@[loc1]);
                    assert(self@[loc2] == old(self)@[loc2]);
                }else {
                    assert(forall|loc1: int| 0 <= loc1 < i*cap  ==> self@[loc1] == pre_self[loc1]);
                    assert(forall|loc1: int| (0 <= loc1 < st) ==> pre_self[loc1] == old(self)@[loc1]);
                    assert(st < i*cap) by(nonlinear_arith)
                        requires
                            first==st/cap as usize,
                            cap>0,
                            i>first,
                    ;
                    assert(self@[loc2] == old(self)@[loc2]);
                }
            };

            //再证明current_end ~ Self::spec_cap()没变
            assert(0 <= stop <= cap) by(nonlinear_arith)
                requires
                    0 <= i < 16,
                    stop == (if i == (ed - 1) / T::spec_cap() as int { if ed % T::spec_cap() == 0 { T::spec_cap() } else {
                            ed % T::spec_cap() }} else { T::spec_cap() }),
                    T::spec_cap() == cap,
                    cap > 0,
            ;
            assert forall|loc2:int| (stop+i*cap <= loc2 < (i+1)*cap) implies self@[loc2] == self.sub[i as int]@[loc2 % cap as int] by{
                assert((i+1)*cap <= Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        Self::spec_cap() == 16 *cap,
                        cap > 0,
                ;
                assert(i == loc2 / cap as int) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        stop+i*cap <= loc2 < (i+1)*cap,
                        0 <= stop <= cap,
                        cap == T::spec_cap(),
                        cap >0,
                ;
            };
            assert forall|loc2:int| (stop+i*cap <= loc2 < (i+1)*cap) implies self@[loc2] == old(self).sub[i as int]@[loc2 % cap as int] by{
                let j=loc2%cap;
                assert(stop <= j < cap) by(nonlinear_arith)
                    requires
                        j==loc2%cap,
                        cap>0,
                        (stop+i*cap <= loc2 < (i+1)*cap),
                ;
            };
            assert forall|loc2:int| (stop+i*cap <= loc2 < (i+1)*cap) implies self@[loc2] == old(self)@[loc2] by{
                let k=loc2/cap;
                let j=loc2%cap;
                assert(k == i) by(nonlinear_arith)
                    requires
                        k==loc2/cap,
                        cap>0,
                        stop+i*cap <= loc2 < (i+1)*cap,
                        0 <= stop <= cap,
                ;
                assert(stop <= j < cap) by(nonlinear_arith)
                    requires
                        j==loc2%cap,
                        cap>0,
                        (stop+i*cap <= loc2 < (i+1)*cap),
                ;
                assert((i+1)*cap <= Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0 <= i < 16,
                        Self::spec_cap() == 16 *cap,
                        cap > 0,
                ;
                assert(self@[loc2] == old(self).sub[i as int]@[loc2 % cap as int]);
                assert(old(self)@[loc2] == old(self).sub[loc2 / cap as int]@[loc2 % cap as int]);
                assert(self@[loc2] == old(self)@[loc2]);
            };

            assert(forall|loc1: int| (i+1)*cap <= loc1 < Self::spec_cap() ==> self@[loc1] == pre_self[loc1]);
            assert(forall|loc1: int| (i+1)*cap <= loc1 < Self::spec_cap() ==> pre_self[loc1] == old(self)@[loc1]);

            assert(forall|loc2: int| (0 <= loc2 < st) ==> self@[loc2] == old(self)@[loc2]);
            assert(forall|loc2: int| current_end <= loc2 < Self::spec_cap() ==> self@[loc2] == old(self)@[loc2]);
            assert(forall|loc2: int|
                    (0 <= loc2 < st || current_end <= loc2 < Self::spec_cap()) ==> self@[loc2] == old(self)@[loc2]);
            i += 1;
            if i == n {
                if ed % T::cap() == 0 {
                    assert(ed == n * cap)by (nonlinear_arith)
                        requires
                            // i <= 16,
                            cap == T::spec_cap(),
                            0 < ed <= Self::spec_cap(),
                            Self::spec_cap() == 16 * cap,
                            ed % T::spec_cap() == 0,
                            last == (ed - 1) / T::spec_cap() as int,
                            n == last +1,
                            cap > 0,
                    ;
                    assert(current_end == ed) by (nonlinear_arith)
                        requires
                            i <= 16,
                            cap == T::spec_cap(),
                            0 < ed <= Self::spec_cap(),
                            Self::spec_cap() == 16 * cap,
                            cap > 0,
                            current_end == stop + (i -1) * T::spec_cap(),
                            stop == cap,
                            ed == n * cap,
                            i == n,
                    ;
                } else {
                    assert(ed == (n-1) * cap + stop )by (nonlinear_arith)
                        requires
                            cap == T::spec_cap(),
                            0 < ed <= Self::spec_cap(),
                            Self::spec_cap() == 16 * cap,
                            last == (ed - 1) / T::spec_cap() as int,
                            n == last +1,
                            cap > 0,
                            stop == ed % T::spec_cap(),
                            ed % T::spec_cap() != 0,
                    ;
                    assert(current_end == ed) by (nonlinear_arith)
                        requires
                            i <= 16,
                            cap == T::spec_cap(),
                            0 < ed <= Self::spec_cap(),
                            Self::spec_cap() == 16 * cap,
                            cap > 0,
                            current_end == stop + (i -1) * T::spec_cap(),
                            stop == ed % T::spec_cap(),
                            i==n,
                            ed == (n-1) * cap + stop,
                    ;
                }
            } else{
                assert(current_end == (i * T::spec_cap()) as usize)by (nonlinear_arith)
                    requires
                        i < 16,
                        cap == T::spec_cap(),
                        0 < ed <= Self::spec_cap(),
                        Self::spec_cap() == 16 * cap,
                        cap > 0,
                        current_end == stop + (i -1) * T::spec_cap(),
                        stop == cap,
                ;
            }
            assert(forall|loc3: int|
                    (0 <= loc3 < first || i <= loc3 < 16) ==> self.sub[loc3]@ == old(self).sub[loc3]@);
        }
    }

    fn insert(&mut self, range: Range<usize>)
    {
        self.set_range_to(range, true);
    }

    fn remove(&mut self, range: Range<usize>)
    {
        self.set_range_to(range, false);
    }
}

impl<T: BitAllocView + Copy> BitAllocCascade16<T> {
    /// Lemma: Ensures the parent view correctly maps each index range to its corresponding child sub-allocator.
    proof fn lemma_maintain_view_indexs_mapping(&self)
        requires
            0 < T::spec_cap(),
            Self::spec_cap() == 16 * T::spec_cap(),
            forall|loc2:int|
                0 <= loc2 < Self::spec_cap()
                ==> self@[loc2] == self.sub[(loc2 / T::spec_cap() as int)]@[(loc2 % T::spec_cap() as int)],
        ensures
            forall|j:int| 0 <= j < 16 ==> view_index_mapping(self@,j,self.sub[j]@,T::spec_cap() as int),
    {
        let ghost cap = T::spec_cap() as int;
        assert forall|j:int| 0 <= j < 16 implies view_index_mapping(self@,j,self.sub[j]@,cap) by{
            assert(forall|loc2:int| (0 <= loc2 < Self::spec_cap()) ==> self@[loc2] == self.sub[loc2 / cap]@[loc2 % cap]);

            assert forall|loc:int| 0 <= loc < cap implies self@[(cap * j + loc)] == self.sub[j]@[loc] by{
                assert(0 <= (cap * j + loc) < Self::spec_cap()) by(nonlinear_arith)
                    requires
                        0 <= j < 16,
                        0 <= loc < cap,
                        Self::spec_cap() == 16*cap,
                ;
                assert((cap * j + loc) / cap == j) by(nonlinear_arith)
                    requires
                        (cap * j + loc) < Self::spec_cap(),
                        Self::spec_cap() == cap * 16,
                        0 <= j < 16,
                        cap > 0,
                        0 <= loc < cap,
                ;
                assert((cap * j + loc) % cap == loc) by(nonlinear_arith)
                    requires
                        (cap * j + loc) < Self::spec_cap(),
                        Self::spec_cap() == cap * 16,
                        0 <= j < 16,
                        cap > 0,
                        0 <= loc < cap,
                ;
            };
        }
    }
}


pub proof fn lemma_view_indexs_st_to_ed_mapping_false(ba: Seq<bool>, i: int, sub_ba: Seq<bool>, cap: int, start: int, end: int)
    requires
        view_index_mapping(ba, i, sub_ba, cap),
        0 <= start < cap,
        0 <= end <= cap,
        start <= end,
        forall|t: int| start <= t < end ==> sub_ba[t] == false,
    ensures
        forall|j: int| (cap * i + start) <= j < (cap * i + end) ==> ba[j] == false,
{
    assert forall|j:int|
        (cap * i + start) <= j < (cap * i + end) implies ba[j] == false
    by {
        // 把 j 写成 cap*i + t 的形状
        let t = j - cap * i;

        // 由区间条件推出 t 的范围
        assert(start <= t) by { j - cap*i >= (cap*i+start) - cap*i ; }
        assert(t <= end)   by { j - cap*i <=  (cap*i+end)   - cap*i ; }

        // 结合前提得到 0 ≤ t < cap
        assert(0 <= t <= cap);

        // 用 j = cap*i + t 改写
        assert(j == cap * i + t);

        // 实例化 view_index_mapping 的量词：
        assert(ba[cap * i + t] == sub_ba[t]);

        // 子序列区间为 false
        assert(sub_ba[t] == false);

        // 合并
        assert(ba[j] == ba[cap*i + t]);
        assert(ba[j] == false);
    };
}

/// Represents a 16-bit bitmap allocator.
#[derive(Clone, Copy)]
pub struct BitAlloc16 {
    pub bits: u16,
}

impl BitAlloc16 {
    /// Gets the boolean value of a specific bit at `index`.
    fn get_bit(&self, index: u16) -> (bit: bool)
        requires
            index < self@.len(),
        ensures
            bit == self@[index as int],
    {
        get_bit16_macro!(self.bits, index as u16)
    }

    /// Extracts a range of bits from the bitmap as a u16 value.
    fn get_bits(&self, range:Range<u16>) -> (bits:u16)
        requires
            range.start < self@.len(),
            range.end <= self@.len(),
            range.start < range.end,
        ensures
            forall|i: int| 0 <= i < (range.end - range.start) ==> (u16_view(bits)[i]) == self@[range.start + i],
    {
        let bv_gets = get_bits16_macro!(self.bits, range.start, range.end);
        proof {
            get_bits_u16_correctness(bv_gets, self.bits, range.start, range.end);
        };
        bv_gets
    }

    /// Sets the boolean value of a specific bit at `index`.
    fn set_bit(&mut self, index: u16, bit: bool)
        requires
            index < old(self)@.len(),
        ensures
            self@ == old(self)@.update(index as int, bit),
    {
        let bit_index: u16 = index % 16;
        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, bit_index as u16, bit);
        proof {
            set_bit_u16_preserves_others(bv_new, bv_old, bit_index as u16, bit);
        }
        ;
        self.bits = bv_new;
        proof {
            assert_seqs_equal!(
                self.view(),
                old(self).view().update(index as int, bit)
            );
        }
        ;
    }

    /// Sets a range of bits in the bitmap to a given u16 value.
    fn set_bits(&mut self, range: Range<u16>, value: u16)
        requires
            range.start < old(self)@.len(),
            range.end <= old(self)@.len(),
            range.start < range.end,
            // Ensure `value` fits within the specified bit range.
            value << ((u16::BITS) as u16 - (range.end - range.start)) as usize >>
                ((u16::BITS) as u16 - (range.end - range.start)) as usize == value,
        ensures
            get_bits16!(self.bits, range.start, range.end) == value,
            forall|loc2: int|
                (0 <= loc2 < range.start || range.end <= loc2  < 16) ==> self@[loc2] == old(self)@[loc2],
    {
        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bits16_macro!(bv_old, range.start, range.end, value);
        proof {
            set_bits_u16_preserves_others(bv_new, bv_old, range.start, range.end, value);
        }
        self.bits = bv_new;
        assert(get_bits16!(bv_new, range.start, range.end) == value);
    }
}

impl BitAllocView for BitAlloc16 {
    /// Specification function to view the internal u16 as a sequence of booleans.
    open spec fn view(&self) -> Seq<bool> {
        let width = Self::spec_cap() as nat;
        Seq::new(width, |i: int| u16_view(self.bits)[i])
        // Seq::new(width, |i: int| get_bit16!(self.bits, i as u16))
    }

    /// The maximum capacity of the bitmap (16 bits).
    fn cap() -> (res:usize) {
        16
    }

    open spec fn spec_cap() -> (res:usize){
        16
    }

    open spec fn cascade_not_overflow() -> bool {
        true
    }

    open spec fn lemma_cap_is_pow16_pre() -> bool {
        true
    }

    proof fn lemma_cap_is_pow16()
    {
        assert(is_pow16(16)) by (compute);
    }

    /// Creates a new `BitmapAllocator16` with all bits set to 0 (all free).
    fn default() -> Self {
        BitAlloc16 { bits: 0 }
    }

    /// Checks if there are any free bits (bits set to 1) in the bitmap.
    fn any(&self) -> (res:bool){
        self.bits != 0
    }

    open spec fn spec_any(&self) -> bool{
        self.bits != 0
    }

    proof fn lemma_bits_nonzero_implies_exists_true(&self)
    {
        let bits = self.bits;
        let ba = self@;
        if self.spec_any() {
            let i: u16 = bits.trailing_zeros() as u16;
            // 1) bits != 0  ==>  0 <= i < 16
            assert(0 <= i < 16);
            // 2) bits != 0  ==>  get_bit16!(bits, i) == true
            assert(get_bit16!(bits, i) == true);

            // 用映射得到 ba[i] == true
            assert(ba[i as int] == get_bit16!(bits, i));
            assert(ba[i as int] == true);

            // 给出见证
            assert(exists|k:int| 0 <= k < 16 && ba[k] == true);
        } else {
            let i: u16 = bits.trailing_zeros() as u16;
            vstd::std_specs::bits::axiom_u16_trailing_zeros(bits);

            assert(i == 16);
            assert(forall|j: u16| 0 <= j < i ==> #[trigger] (bits >> j) & 1u16 == 0u16)
        }

        assert(self.spec_any() == exists|k:int| 0 <= k < 16 && ba[k] == true);
    }

    /// Tests if a specific bit at `index` is free (1) or allocated (0).
    fn test(&self, key: usize) -> (res:bool)
    {
        let res = self.get_bit(key as u16);
        res
    }

    open spec fn wf(&self) -> bool {
        Self::cascade_not_overflow()
        && Self::spec_cap() == 16
    }

    /// Finds the next free bit (1) starting from `key` (inclusive).
    /// Returns `Some(index)` of the next free bit, or `None` if no free bits are found.
    fn next(&self, key: usize) -> (res: Option<usize>){
        let n = u16::BITS as u16;
        let mut result = None;
        let mut i = key as u16;
        assert(i<n);
        while i < n
            invariant_except_break
                result.is_none(),
            invariant
                key <= i <= n,
                n == Self::spec_cap(),
                forall|k: int|
                    key <= k < i ==> self@[k] == false,
            ensures
                (i == n && result.is_none()) ||  (i < n && result.is_some() && (result.unwrap() == i as usize) && self@[i as int] == true),
            decreases
                n-i,
        {
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
    fn alloc(&mut self) -> (res: Option<usize>)
    {
        if !self.any() {
            return None;
        }
        // Find the first free bit (least significant 1-bit).
        let i = self.bits.trailing_zeros() as u16;
        assert(i < u16::BITS) by{
            assert(self.bits != 0); // Prove that if self.bits != 0, trailing_zeros() < 16.
        };
        assert(self@[i as int] == get_bit16!(self.bits, i));
        proof{
            vstd::std_specs::bits::axiom_u16_trailing_zeros(self.bits);
        }
        assert(get_bit16!(self.bits, i) == true);
        let bv_old: u16 = self.bits;
        let bv_new: u16 = set_bit16_macro!(bv_old, i, false);
        proof {
            set_bit_u16_preserves_others(bv_new, bv_old, i, false);
        }
        ;
        self.bits = bv_new;
        proof {
            assert_seqs_equal!(
                self.view(),
                old(self).view().update(i as int, false)
            );
        }
        assert(self@[i as int] == false);
        Some(i as usize)
    }

    /// Allocates a contiguous block of `size` bits with specified `align_log2` alignment.
    /// Returns `Some(base_index)` if successful, `None` if no suitable block is found.
    fn alloc_contiguous(&mut self, size: usize, align_log2: usize) -> (res: Option<usize>)
    {
        assert(Self::spec_cap() == 16);
        // let i = self.cap().trailing_zeros() as usize;
        assert(is_pow16(Self::spec_cap())) by (compute);
        assert(Self::spec_cap() % 16 == 0);
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
    fn dealloc(&mut self, key: usize)
    {
        self.set_bit(key as u16, true);
        assert(self@[key as int]);
        assert(self.spec_any()) by{
            self.lemma_bits_nonzero_implies_exists_true();
        };
    }

    /// Marks a range of bits as allocated (sets them to 0).
    fn set_range_to(&mut self, range: Range<usize>, val: bool)
    {
        if val {
            self.insert(range);
        } else {
            self.remove(range);
        }
    }

    /// Marks a range of bits as free (sets them to 1).
    fn insert(&mut self, range: Range<usize>)
    {
        let ghost st = range.start;
        let ghost ed = range.end;
        let width = (range.end - range.start) as u16;
        assert(width<=16);
        let insert_val = 0xffffu16 >> ((16 - width) as u16);

        // Proof that insert_val's higher (16 - width) bits are zero.
        assert(insert_val << (16 - width) as u16 >> (16 - width) as u16 == insert_val) by (bit_vector)
            requires
                insert_val == 0xffffu16 >> ((16 - width) as u16)
        ;

        let range_u16 = (range.start as u16)..(range.end as u16);
        self.set_bits(range_u16, insert_val);

        assert(forall|j: u16| 0 <= j < width ==> (insert_val >> j) & 1u16 == 1u16) by (bit_vector)
            requires
                width == (ed - st) as u16,
                0 < width <= 16,
                insert_val == 0xffffu16 >> ((16 - width) as u16),
        ;

        assert forall|loc1: int| (st <= loc1 < ed) implies self@[loc1] == true by {
            // 将 loc1 归一化到区间起点的第 i 位
            let i = loc1 - st;
            assert(0 <= i < width) by (nonlinear_arith)
                requires
                    st <= loc1 < ed,
                    i == loc1 - st,
                    width == ed - st,
            ;

            assert(get_bits16!(self.bits, st as u16, ed as u16) == (0xffffu16 >> ((16 - width) as u16)));

            get_bits_u16_correctness(insert_val, self.bits, st as u16, ed as u16);

            assert((u16_view(insert_val)[i]) == self@[st + i]);

            assert((insert_val >> i) & 1u16 == 1u16);

            assert((u16_view(insert_val)[i]) == true);
        }
    }

    /// Marks a range of bits as allocated (sets them to 0).
    fn remove(&mut self, range: Range<usize>)
    {
        let ghost st = range.start;
        let ghost ed = range.end;
        let value:u16 = 0;
        let width:u16 = (range.end - range.start) as u16;
        assert((16 - width) >= 0);
        assert(0u16 << (16 - width) as u16 >>
        (16 - width) as u16 == 0u16) by (bit_vector);
        let range_u16 = (range.start as u16)..(range.end as u16);
        self.set_bits(range_u16, value);

        assert(forall|j: u16| 0 <= j < width ==> (value >> j) & 1u16 == 0u16) by (bit_vector)
            requires
                width == (ed - st) as u16,
                0 < width <= 16,
                value == 0,
        ;

        assert forall|loc1: int| st <= loc1 < ed implies self@[loc1] == false by{
            let i = loc1 - st;
            assert(0 <= i < width) by (nonlinear_arith)
                requires
                    st <= loc1 < ed,
                    i == loc1 - st,
                    width == ed - st,
            ;
            assert(get_bits16!(self.bits, st as u16, ed as u16) == 0);

            get_bits_u16_correctness(value, self.bits, st as u16, ed as u16);

            assert((u16_view(value)[i]) == self@[st + i]);

            assert((value >> i) & 1u16 == 0u16);

            assert((u16_view(value)[i]) == false);
        }
    }

}

/// Specification function to check if a contiguous block starting at `i` of `size` contains any allocated bits (false).
/// or `i` is not a multiple of `align`
pub open spec fn has_obstruction(ba: Seq<bool>, i: int, size: int, align: int) -> bool {
    (i % align != 0) ||
    exists |k: int| i <= k < i + size && #[trigger] ba[k] == false
}

pub open spec fn no_available(ba: Seq<bool>, cap: int) -> bool {
    forall|j:int| 0 <= j < cap ==> ba[j] == false
}

#[verifier::bit_vector]
proof fn safe_shr_lemma(x: usize, shift: usize)
    requires shift < 64
    ensures (x >> shift) <= x,
{
}

/// The capacity is an exponential multiple of 16.
/// and the current bitmap only supports the maximum allocatable page size of 1M.
pub open spec fn is_pow16(cap: usize) -> bool {
    cap == 16 || cap == 256 || cap == 4096 || cap == 65536 || cap == 1048576
}

// pub open spec fn is_pow16(n: usize) -> bool
//     decreases n
// {
//     n == 16 || (n >= 256 && n % 16 == 0 && is_pow16(n / 16))
// }

// /// The capacity is an exponential multiple of 16.
// /// cap == 16 || cap == 256 || cap == 4096 || cap == 65536 || cap == 1048576
// pub open spec fn is_pow16(cap: usize) -> bool {
//     &&& cap >= 16
//     &&& (cap & (cap-1usize) as usize) == 0
//     &&& (cap % 15usize == 1)
// }

/// Finds a contiguous block of `size` free bits within the bitmap, respecting `align_log2`.
/// Returns the base index of the found block, or `None` if no such block exists.
fn find_contiguous<T: BitAllocView>(ba: &T, capacity: usize, size: usize, align_log2: usize) -> (res: Option<usize>)
    requires
        ba.wf(),
        capacity == T::spec_cap(),
        align_log2 < 64,
        0 < size <= capacity,
        capacity < 0x100000,
        is_pow16(capacity),
    ensures
        ba.wf(),
        match res {
            Some(base) => {
                // If successful, a block of `size` free bits is found starting at `base`.
                // The block must be within capacity, aligned, and all bits within the block must be free (true).
                &&& base + size <= capacity
                &&& base % (1usize << align_log2) == 0
                &&& forall|i: int| base <= i < base + size ==> ba@[i] == true //ba.next(i) != None
            },
            None => {
                // If failed, no suitable block exists.
                // This implies either no free bits, or all potential blocks are obstructed or misaligned.
                capacity < (1usize << align_log2) || !ba.spec_any() ||
                forall|i: int| (0 <= i <= capacity - size) ==> has_obstruction(ba@, i, size as int,(1usize << align_log2) as int)
            }
        }
{
    // assert(capacity==16);
    assert(capacity == T::spec_cap());
    if (capacity < (1usize << align_log2)) || !ba.any() {
        return None;
    }
    assert(capacity >= (1usize << align_log2));
    assert(ba.spec_any() == true);
    let mut base:usize = 0;
    // Proof that initial base (0) is aligned.
    assert(base % (1usize << align_log2) == 0) by (bit_vector)
        requires
            base == 0,
            align_log2 < 64,
    ;

    let mut offset:usize = base;
    let mut res:Option<usize> = None;
    while offset < capacity
        invariant
            ba.wf(),
            capacity < 0x100000,
            is_pow16(capacity),
            capacity >= (1usize << align_log2),
            offset <= capacity,
            offset - base < size,
            forall|i: int| (0 <= i < base) ==> has_obstruction(ba@, i, size as int, (1usize << align_log2) as int),
            forall|i: int| (0 <= i < capacity) && (i % (1usize << align_log2) as int != 0) ==> has_obstruction(ba@, i, size as int, (1usize << align_log2) as int),
            capacity == T::spec_cap(),
            align_log2 < 64,
            0 < size <= capacity,
            0 <= base <= offset,
            base % (1usize << align_log2) == 0,
            forall|i: int| base <= i < offset ==> ba@.index(i),
        decreases
            capacity - offset,
    {
        if let Some(next) = ba.next(offset) {
            assert(next < T::spec_cap());
            assert(offset - base < size);
            assert(next >= offset);
            assert(ba@[next as int] == true);
            if next != offset {
                assert(next > offset);
                assert(offset<=capacity);
                // it can be guarenteed that no bit in (offset..next) is free
                // move to next aligned position after next-1
                assert(next > 0);

                assert(ba@[offset as int] == false);
                assert(forall|i: usize| (offset - size < i <= offset) ==> has_obstruction(ba@, i as int, size as int, (1usize << align_log2) as int));

                assert(((next - 1) as usize) >= 0);
                proof{
                    safe_shr_lemma(((next - 1) as usize),align_log2);
                }
                base = (((((next - 1) as usize) >> align_log2) + 1) as usize) << align_log2;

                // assert((capacity & (capacity-1usize) as usize) == 0);

                proof{
                    lemma_up_align_ge_original(next, capacity, align_log2);
                }
                assert(base >= next);

                assert(forall|i: int| (offset <= i < next) ==> (ba@[i] == false));
                // lemma_bit_false_implies_has_obstruction(ba@,);
                assert forall|i: usize| (offset <= i < next) implies has_obstruction(ba@, i as int, size as int, (1usize << align_log2) as int) by{
                    assert(ba@[i as int] == false);
                }

                offset = base;
                assert(offset >= next); // decreases
                assert(base % (1usize << align_log2) == 0) by (bit_vector)
                    requires
                        base == (((((next - 1) as usize) >> align_log2) + 1) as usize) << align_log2,
                        align_log2 < 64,
                        capacity < 0x100000,
                        0 < next < capacity,
                        (1usize << align_log2) <= capacity,
                ;
                proof{
                    lemma_up_align_le_capacity(next, capacity, align_log2);
                }
                assert(base <= capacity);
                assert(offset - base < size);
                assert forall|i: usize| (next <= i < base) implies has_obstruction(ba@, i as int, size as int, (1usize << align_log2) as int) by{
                    assert(i % (1usize << align_log2) != 0) by (bit_vector)
                        requires
                            next <= i < base,
                            0 < next <= base,
                            base == (((((next - 1) as usize) >> align_log2) + 1) as usize) << align_log2,
                            base <= capacity,
                            align_log2 < 64,
                            capacity < 0x100000,
                            (1usize << align_log2) <= capacity,
                    ;
                }
                continue;
            }
            assert(offset == next);

        } else {
            // No more free bits found from `offset` to `capacity`.
            assert(ba@[offset as int] == false);
            assert(forall|i: usize| (offset - size < i <= offset) ==> has_obstruction(ba@, i as int, size as int, (1usize << align_log2) as int));
            assert forall|i: usize| (offset <= i < capacity) implies has_obstruction(ba@, i as int, size as int, (1usize << align_log2) as int) by{
                assert(ba@[i as int] == false);
            }
            return None;
        }
        assert(size > 0);
        assert(offset - base < size);
        offset += 1;
        assert(offset > base);
        if offset - base == size {
            assert(offset - base == size);
            assert(base + size == offset);
            res = Some(base);
            return res;
        }
    }
    None
}


/// Lemma to prove that aligning `next` upwards results in a value greater than or equal to `next`.
#[verifier::bit_vector]
proof fn lemma_up_align_ge_original(next:usize, capacity:usize, align_log2:usize)
    requires
        capacity < 0x100000,
        0 < next < capacity,
        align_log2 < 64,
        (1usize << align_log2) <= capacity,
    ensures
        (((((next - 1) as usize) >> align_log2) + 1) as usize) << align_log2 >= next
{
}

// /// Lemma to prove that aligning `next` upwards results in a value less than or equal to the capacity (16).
#[verifier::bit_vector]
proof fn lemma_up_align_le_capacity(next:usize, capacity:usize, align_log2:usize)
    requires
        // 16 <= capacity <= 0x100000,
        // capacity <= 0x100000,
        0 < next < capacity,
        align_log2 < 64,
        (1usize << align_log2) <= capacity,
        capacity == 16 || capacity == 256 || capacity == 4096 || capacity == 65536 || capacity == 1048576,
        // (capacity & (capacity-1usize) as usize) == 0,
        // (capacity % 15usize == 1),
        // is_pow16(capacity),
    ensures
        (((((next - 1usize) as usize) >> align_log2) + 1usize) as usize) << align_log2 <= capacity
{
}

#[verifier::external]
fn main() {

}

} // verus!

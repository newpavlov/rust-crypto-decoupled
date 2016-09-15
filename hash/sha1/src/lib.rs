//! An implementation of the SHA-1 cryptographic hash algorithm.

//! To use this module, first create a `Sha1` object using the `Sha1` constructor,
//! then feed it an input message using the `input` or `input_str` methods,
//! which may be called any number of times; they will buffer the input until
//! there is enough to call the block algorithm.
//!
//! After the entire input has been fed to the hash read the result using
//! the `result` or `result_str` methods. The first will return bytes, and
//! the second will return a `String` object of the same bytes represented
//! in hexadecimal form.

//! The `Sha1` object may be reused to create multiple hashes by calling
//! the `reset()` method. These traits are implemented by all hash digest
//! algorithms that implement the `Digest` trait. An example of use is:

//! ```rust, ignore3
//! use self::crypto::digest::Digest;
//! use self::crypto::sha1::Sha1;
//!
//! // create a Sha1 object
//! let mut hasher = Sha1::new();
//! 
//! // write input message
//! hasher.input_str("hello world");
//!
//! // read hash digest
//! let hex = hasher.result_str();
//!
//! assert_eq!(hex, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
//! ```
//!
//! # Mathematics
//!
//! The mathematics of the SHA-1 algorithm are quite interesting. In its
//! definition, The SHA-1 algorithm uses:
//!
//! * 1 binary operation on bit-arrays:
//!   * "exclusive or" (XOR)
//! * 2 binary operations on integers:
//!   * "addition" (ADD)
//!   * "rotate left" (ROL)
//! * 3 ternary operations on bit-arrays:
//!   * "choose" (CH)
//!   * "parity" (PAR)
//!   * "majority" (MAJ)
//!
//! Some of these functions are commonly found in all hash digest
//! algorithms, but some, like "parity" is only found in SHA-1.


#![no_std]
#![feature(test)]
extern crate test;
extern crate crypto_bytes;
extern crate crypto_digest;
extern crate crypto_fixed_buffer;
extern crate simd;
#[cfg(test)]
#[macro_use]
extern crate crypto_tests;

use crypto_digest::Digest;
use crypto_bytes::{write_u32_be, read_u32v_be, add_bytes_to_bits};
use crypto_fixed_buffer::{FixedBuffer, FixedBuffer64, StandardPadding};
use simd::u32x4;

const STATE_LEN: usize = 5;
const BLOCK_LEN: usize = 16;

const K0: u32 = 0x5A827999u32;
const K1: u32 = 0x6ED9EBA1u32;
const K2: u32 = 0x8F1BBCDCu32;
const K3: u32 = 0xCA62C1D6u32;

/// Not an intrinsic, but gets the first element of a vector.
#[inline]
pub fn sha1_first(w0: u32x4) -> u32 {
    w0.0
}

/// Not an intrinsic, but adds a word to the first element of a vector.
#[inline]
pub fn sha1_first_add(e: u32, w0: u32x4) -> u32x4 {
    let u32x4(a, b, c, d) = w0;
    u32x4(e.wrapping_add(a), b, c, d)
}

/// Emulates `llvm.x86.sha1msg1` intrinsic.
fn sha1msg1(a: u32x4, b: u32x4) -> u32x4 {
    let u32x4(_, _, w2, w3) = a;
    let u32x4(w4, w5, _, _) = b;
    a ^ u32x4(w2, w3, w4, w5)
}

/// Emulates `llvm.x86.sha1msg2` intrinsic.
fn sha1msg2(a: u32x4, b: u32x4) -> u32x4 {
    let u32x4(x0, x1, x2, x3) = a;
    let u32x4(_, w13, w14, w15) = b;

    let w16 = (x0 ^ w13).rotate_left(1);
    let w17 = (x1 ^ w14).rotate_left(1);
    let w18 = (x2 ^ w15).rotate_left(1);
    let w19 = (x3 ^ w16).rotate_left(1);

    u32x4(w16, w17, w18, w19)
}

/// Performs 4 rounds of the message schedule update.
pub fn sha1_schedule_x4(v0: u32x4, v1: u32x4, v2: u32x4, v3: u32x4) -> u32x4 {
    sha1msg2(sha1msg1(v0, v1) ^ v2, v3)
}

/// Emulates `llvm.x86.sha1nexte` intrinsic.
#[inline]
pub fn sha1_first_half(abcd: u32x4, msg: u32x4) -> u32x4 {
    sha1_first_add(sha1_first(abcd).rotate_left(30), msg)
}

/// Emulates `llvm.x86.sha1rnds4` intrinsic.
/// Performs 4 rounds of the message block digest.
pub fn sha1_digest_round_x4(abcd: u32x4, work: u32x4, i: i8) -> u32x4 {
    const K0V: u32x4 = u32x4(K0, K0, K0, K0);
    const K1V: u32x4 = u32x4(K1, K1, K1, K1);
    const K2V: u32x4 = u32x4(K2, K2, K2, K2);
    const K3V: u32x4 = u32x4(K3, K3, K3, K3);

    match i {
        0 => sha1rnds4c(abcd, work + K0V),
        1 => sha1rnds4p(abcd, work + K1V),
        2 => sha1rnds4m(abcd, work + K2V),
        3 => sha1rnds4p(abcd, work + K3V),
        _ => panic!("unknown icosaround index"),
    }
}

/// Not an intrinsic, but helps emulate `llvm.x86.sha1rnds4` intrinsic.
fn sha1rnds4c(abcd: u32x4, msg: u32x4) -> u32x4 {
    let u32x4(mut a, mut b, mut c, mut d) = abcd;
    let u32x4(t, u, v, w) = msg;
    let mut e = 0u32;

    macro_rules! bool3ary_202 {
        ($a:expr, $b:expr, $c:expr) => (($c ^ ($a & ($b ^ $c))))
    } // Choose, MD5F, SHA1C

    e = e.wrapping_add(a.rotate_left(5))
        .wrapping_add(bool3ary_202!(b, c, d))
        .wrapping_add(t);
    b = b.rotate_left(30);

    d = d.wrapping_add(e.rotate_left(5))
        .wrapping_add(bool3ary_202!(a, b, c))
        .wrapping_add(u);
    a = a.rotate_left(30);

    c = c.wrapping_add(d.rotate_left(5))
        .wrapping_add(bool3ary_202!(e, a, b))
        .wrapping_add(v);
    e = e.rotate_left(30);

    b = b.wrapping_add(c.rotate_left(5))
        .wrapping_add(bool3ary_202!(d, e, a))
        .wrapping_add(w);
    d = d.rotate_left(30);

    u32x4::new(b, c, d, e)
}

/// Not an intrinsic, but helps emulate `llvm.x86.sha1rnds4` intrinsic.
fn sha1rnds4p(abcd: u32x4, msg: u32x4) -> u32x4 {
    let u32x4(mut a, mut b, mut c, mut d) = abcd;
    let u32x4(t, u, v, w) = msg;
    let mut e = 0u32;

    macro_rules! bool3ary_150 {
        ($a:expr, $b:expr, $c:expr) => (($a ^ $b ^ $c))
    } // Parity, XOR, MD5H, SHA1P

    e = e.wrapping_add(a.rotate_left(5))
        .wrapping_add(bool3ary_150!(b, c, d))
        .wrapping_add(t);
    b = b.rotate_left(30);

    d = d.wrapping_add(e.rotate_left(5))
        .wrapping_add(bool3ary_150!(a, b, c))
        .wrapping_add(u);
    a = a.rotate_left(30);

    c = c.wrapping_add(d.rotate_left(5))
        .wrapping_add(bool3ary_150!(e, a, b))
        .wrapping_add(v);
    e = e.rotate_left(30);

    b = b.wrapping_add(c.rotate_left(5))
        .wrapping_add(bool3ary_150!(d, e, a))
        .wrapping_add(w);
    d = d.rotate_left(30);

    u32x4(b, c, d, e)
}

/// Not an intrinsic, but helps emulate `llvm.x86.sha1rnds4` intrinsic.
fn sha1rnds4m(abcd: u32x4, msg: u32x4) -> u32x4 {
    let u32x4(mut a, mut b, mut c, mut d) = abcd;
    let u32x4(t, u, v, w) = msg;
    let mut e = 0u32;

    macro_rules! bool3ary_232 {
        ($a:expr, $b:expr, $c:expr) => (($a & $b) ^ ($a & $c) ^ ($b & $c))
    } // Majority, SHA1M

    e = e.wrapping_add(a.rotate_left(5))
        .wrapping_add(bool3ary_232!(b, c, d))
        .wrapping_add(t);
    b = b.rotate_left(30);

    d = d.wrapping_add(e.rotate_left(5))
        .wrapping_add(bool3ary_232!(a, b, c))
        .wrapping_add(u);
    a = a.rotate_left(30);

    c = c.wrapping_add(d.rotate_left(5))
        .wrapping_add(bool3ary_232!(e, a, b))
        .wrapping_add(v);
    e = e.rotate_left(30);

    b = b.wrapping_add(c.rotate_left(5))
        .wrapping_add(bool3ary_232!(d, e, a))
        .wrapping_add(w);
    d = d.rotate_left(30);

    u32x4(b, c, d, e)
}

/// Process a block with the SHA-1 algorithm.
pub fn sha1_digest_block_u32(state: &mut [u32; 5], block: &[u32; 16]) {

    macro_rules! schedule {
        ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => (
            sha1msg2(sha1msg1($v0, $v1) ^ $v2, $v3)
        )
    }

    macro_rules! rounds4 {
        ($h0:ident, $h1:ident, $wk:expr, $i:expr) => (
            sha1_digest_round_x4($h0, sha1_first_half($h1, $wk), $i)
        )
    }

    // Rounds 0..20
    let mut h0 = u32x4::load(state, 0);
    let mut w0 = u32x4::load(block, 0);
    let mut h1 = sha1_digest_round_x4(h0, sha1_first_add(state[4], w0), 0);
    let mut w1 = u32x4::load(block, 4);
    h0 = rounds4!(h1, h0, w1, 0);
    let mut w2 = u32x4::load(block, 8);
    h1 = rounds4!(h0, h1, w2, 0);
    let mut w3 = u32x4::load(block, 12);
    h0 = rounds4!(h1, h0, w3, 0);
    let mut w4 = schedule!(w0, w1, w2, w3);
    h1 = rounds4!(h0, h1, w4, 0);

    // Rounds 20..40
    w0 = schedule!(w1, w2, w3, w4);
    h0 = rounds4!(h1, h0, w0, 1);
    w1 = schedule!(w2, w3, w4, w0);
    h1 = rounds4!(h0, h1, w1, 1);
    w2 = schedule!(w3, w4, w0, w1);
    h0 = rounds4!(h1, h0, w2, 1);
    w3 = schedule!(w4, w0, w1, w2);
    h1 = rounds4!(h0, h1, w3, 1);
    w4 = schedule!(w0, w1, w2, w3);
    h0 = rounds4!(h1, h0, w4, 1);

    // Rounds 40..60
    w0 = schedule!(w1, w2, w3, w4);
    h1 = rounds4!(h0, h1, w0, 2);
    w1 = schedule!(w2, w3, w4, w0);
    h0 = rounds4!(h1, h0, w1, 2);
    w2 = schedule!(w3, w4, w0, w1);
    h1 = rounds4!(h0, h1, w2, 2);
    w3 = schedule!(w4, w0, w1, w2);
    h0 = rounds4!(h1, h0, w3, 2);
    w4 = schedule!(w0, w1, w2, w3);
    h1 = rounds4!(h0, h1, w4, 2);

    // Rounds 60..80
    w0 = schedule!(w1, w2, w3, w4);
    h0 = rounds4!(h1, h0, w0, 3);
    w1 = schedule!(w2, w3, w4, w0);
    h1 = rounds4!(h0, h1, w1, 3);
    w2 = schedule!(w3, w4, w0, w1);
    h0 = rounds4!(h1, h0, w2, 3);
    w3 = schedule!(w4, w0, w1, w2);
    h1 = rounds4!(h0, h1, w3, 3);
    w4 = schedule!(w0, w1, w2, w3);
    h0 = rounds4!(h1, h0, w4, 3);

    let e = sha1_first(h1).rotate_left(30);
    let u32x4(a, b, c, d) = h0;

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

/// Process a block with the SHA-1 algorithm. (See more...)
///
/// SHA-1 is a cryptographic hash function, and as such, it operates
/// on an arbitrary number of bytes. This function operates on a fixed
/// number of bytes. If you call this function with anything other than
/// 64 bytes, then it will panic! This function takes two arguments:
///
/// * `state` is reference to an **array** of 5 words.
/// * `block` is reference to a **slice** of 64 bytes.
///
/// If you want the function that performs a message digest on an arbitrary
/// number of bytes, then see also the `Sha1` struct above.
///
/// # Implementation
///
/// First, some background. Both ARM and Intel are releasing documentation
/// that they plan to include instruction set extensions for SHA1 and SHA256
/// sometime in the near future. Second, LLVM won't lower these intrinsics yet,
/// so these functions were written emulate these instructions. Finally,
/// the block function implemented with these emulated intrinsics turned out
/// to be quite fast! What follows is a discussion of this CPU-level view
/// of the SHA-1 algorithm and how it relates to the mathematical definition.
///
/// The SHA instruction set extensions can be divided up into two categories:
///
/// * message work schedule update calculation ("schedule" v., "work" n.)
/// * message block 80-round digest calculation ("digest" v., "block" n.)
///
/// The schedule-related functions can be used to easily perform 4 rounds
/// of the message work schedule update calculation, as shown below:
///
/// ```ignore
/// macro_rules! schedule_x4 {
///     ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => (
///         sha1msg2(sha1msg1($v0, $v1) ^ $v2, $v3)
///     )
/// }
///
/// macro_rules! round_x4 {
///     ($h0:ident, $h1:ident, $wk:expr, $i:expr) => (
///         sha1rnds4($h0, sha1_first_half($h1, $wk), $i)
///     )
/// }
/// ```
///
/// and also shown above is how the digest-related functions can be used to
/// perform 4 rounds of the message block digest calculation.
///
pub fn sha1_digest_block(state: &mut [u32; 5], block: &[u8]) {
    assert_eq!(block.len(), BLOCK_LEN * 4);
    let mut block2 = [0u32; BLOCK_LEN];
    read_u32v_be(&mut block2[..], block);
    sha1_digest_block_u32(state, &block2);
}

fn add_input(st: &mut Sha1, msg: &[u8]) {
    assert!((!st.computed));
    // Assumes that msg.len() can be converted to u64 without overflow
    st.length_bits = add_bytes_to_bits(st.length_bits, msg.len() as u64);
    let st_h = &mut st.h;
    st.buffer.input(msg, |d: &[u8]| {
        sha1_digest_block(st_h, d);
    });
}

fn mk_result(st: &mut Sha1, rs: &mut [u8]) {
    if !st.computed {
        let st_h = &mut st.h;
        st.buffer
            .standard_padding(8, |d: &[u8]| sha1_digest_block(&mut *st_h, d));
        write_u32_be(st.buffer.next(4), (st.length_bits >> 32) as u32);
        write_u32_be(st.buffer.next(4), st.length_bits as u32);
        sha1_digest_block(st_h, st.buffer.full_buffer());

        st.computed = true;
    }

    write_u32_be(&mut rs[0..4], st.h[0]);
    write_u32_be(&mut rs[4..8], st.h[1]);
    write_u32_be(&mut rs[8..12], st.h[2]);
    write_u32_be(&mut rs[12..16], st.h[3]);
    write_u32_be(&mut rs[16..20], st.h[4]);
}

/// Structure representing the state of a Sha1 computation
#[derive(Clone, Copy)]
pub struct Sha1 {
    h: [u32; STATE_LEN],
    length_bits: u64,
    buffer: FixedBuffer64,
    computed: bool,
}

impl Sha1 {
    /// Construct a `sha` object
    pub fn new() -> Sha1 {
        let mut st = Sha1 {
            h: [0u32; STATE_LEN],
            length_bits: 0u64,
            buffer: FixedBuffer64::new(),
            computed: false,
        };
        st.reset();
        st
    }
}

impl Default for Sha1 {
    fn default() -> Self { Self::new() }
}

impl Digest for Sha1 {
    fn reset(&mut self) {
        self.length_bits = 0;
        self.h[0] = 0x67452301u32;
        self.h[1] = 0xEFCDAB89u32;
        self.h[2] = 0x98BADCFEu32;
        self.h[3] = 0x10325476u32;
        self.h[4] = 0xC3D2E1F0u32;
        self.buffer.reset();
        self.computed = false;
    }
    fn input(&mut self, msg: &[u8]) { add_input(self, msg); }
    fn result(&mut self, out: &mut [u8]) { mk_result(self, out) }
    fn output_bytes(&self) -> usize { 20 }
    fn block_size(&self) -> usize { 64 }
}


#[cfg(test)]
mod tests;

#[cfg(test)]
mod bench;
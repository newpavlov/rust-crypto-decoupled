#![no_std]
#![feature(test)]
extern crate test;
extern crate ripemd160;
extern crate crypto_digest;

use test::Bencher;
use crypto_digest::Digest;
use ripemd160::Ripemd160;


#[bench]
pub fn ripemd160_10(bh: &mut Bencher) {
    let mut sh = Ripemd160::new();
    let bytes = [1u8; 10];
    bh.iter(|| {
        sh.input(&bytes);
    });
    bh.bytes = bytes.len() as u64;
}

#[bench]
pub fn ripemd160_1k(bh: &mut Bencher) {
    let mut sh = Ripemd160::new();
    let bytes = [1u8; 1024];
    bh.iter(|| {
        sh.input(&bytes);
    });
    bh.bytes = bytes.len() as u64;
}

#[bench]
pub fn ripemd160_64k(bh: &mut Bencher) {
    let mut sh = Ripemd160::new();
    let bytes = [1u8; 65536];
    bh.iter(|| {
        sh.input(&bytes);
    });
    bh.bytes = bytes.len() as u64;
}
#!/bin/sh
cd md5; cargo test;
cd ../ripemd160; cargo test;
cd ../sha1; cargo test;
cd ../sha2; cargo test;
cd ../sha3; cargo test;
cd ../whirlpool; cargo test;
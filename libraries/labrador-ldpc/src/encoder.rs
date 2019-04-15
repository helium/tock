// Copyright 2017 Adam Greig
// Licensed under the MIT license, see LICENSE for details.

//! This module provides the encoding function for turning data into codewords.
//!
//! Please refer to the `encode` and `copy_encode` methods on
//! [`LDPCCode`](../codes/enum.LDPCCode.html) for more details.

// We have a couple of expressions with +0 for clarity of where the 0 comes from
#![cfg_attr(feature="cargo-clippy", allow(identity_op))]

use core::slice;

use codes::LDPCCode;

/// Trait for the types of codeword we can encode into.
///
/// We implement this for u8 (the standard but slow option), and u32 and u64 which give speedups.
pub trait EncodeInto {
    /// Given `codeword` which has the first k bits set to the data to transmit,
    /// sets the remaining n-k parity bits.
    ///
    /// Returns a `&mut [u8]` view on `codeword`.
    fn encode<'a>(code: &LDPCCode, codeword: &'a mut [Self]) -> &'a mut [u8]
    where
        Self: Sized;

    /// First copies `data` into the first k bits of `codeword`, then calls `encode`.
    fn copy_encode<'a>(code: &LDPCCode, data: &[u8], codeword: &'a mut [Self]) -> &'a mut [u8]
    where
        Self: Sized;

    /// Returns the bit length for this type
    fn bitlength() -> usize;
}

impl EncodeInto for u8 {
    fn encode<'a>(code: &LDPCCode, codeword: &'a mut [Self]) -> &'a mut [u8] {
        let k = code.k();
        let r = code.n() - code.k();
        let b = code.circulant_size();
        let gc = code.compact_generator();
        let row_len = r / 64;

        // Scope the split of codeword into (data, parity)
        {
            // Split codeword into data and parity sections and then zero the parity bits
            let (data, parity) = codeword.split_at_mut(k / 8);
            for x in parity.iter_mut() {
                *x = 0;
            }

            // For each rotation of the generator circulants
            for offset in 0..b {
                // For each row of circulants
                for crow in 0..k / b {
                    // Data bit (row of full generator matrix)
                    let bit = crow * b + offset;
                    if data[bit / 8] >> (7 - (bit % 8)) & 1 == 1 {
                        // If bit is set, XOR the generator constant in
                        for (idx, circ) in
                            gc[crow * row_len..(crow + 1) * row_len].iter().enumerate()
                        {
                            parity[idx * 8 + 7] ^= (*circ >> 0) as u8;
                            parity[idx * 8 + 6] ^= (*circ >> 8) as u8;
                            parity[idx * 8 + 5] ^= (*circ >> 16) as u8;
                            parity[idx * 8 + 4] ^= (*circ >> 24) as u8;
                            parity[idx * 8 + 3] ^= (*circ >> 32) as u8;
                            parity[idx * 8 + 2] ^= (*circ >> 40) as u8;
                            parity[idx * 8 + 1] ^= (*circ >> 48) as u8;
                            parity[idx * 8 + 0] ^= (*circ >> 56) as u8;
                        }
                    }
                }
                // Now simulate the right-rotation of the generator by left-rotating the parity
                for block in 0..r / b {
                    let parityblock = &mut parity[block * b / 8..(block + 1) * b / 8];
                    let mut carry = parityblock[0] >> 7;
                    for x in parityblock.iter_mut().rev() {
                        let c = *x >> 7;
                        *x = (*x << 1) | carry;
                        carry = c;
                    }
                }
            }
        }

        // Return a &mut [u8] view on the codeword
        codeword
    }

    fn copy_encode<'a>(code: &LDPCCode, data: &[u8], codeword: &'a mut [Self]) -> &'a mut [u8] {
        codeword[..data.len()].copy_from_slice(data);
        Self::encode(code, codeword)
    }

    fn bitlength() -> usize {
        8
    }
}

impl EncodeInto for u32 {
    fn encode<'a>(code: &LDPCCode, codeword: &'a mut [Self]) -> &'a mut [u8] {
        let k = code.k();
        let r = code.n() - code.k();
        let b = code.circulant_size();
        let gc = code.compact_generator();
        let row_len = r / 64;

        // Scope the split of codeword into (data, parity)
        {
            // Split codeword into data and parity sections and then zero the parity bits
            let (data, parity) = codeword.split_at_mut(k / 32);
            for x in parity.iter_mut() {
                *x = 0;
            }

            // We treat data as a &[u8] so we bit-index it correctly despite endianness
            let data = unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };

            // For each rotation of the generator circulants
            for offset in 0..b {
                // For each row of circulants
                for crow in 0..k / b {
                    // Data bit (row of full generator matrix)
                    let bit = crow * b + offset;
                    if data[bit / 8] >> (7 - (bit % 8)) & 1 == 1 {
                        // If bit is set, XOR the generator constant in
                        for (idx, circ) in
                            gc[crow * row_len..(crow + 1) * row_len].iter().enumerate()
                        {
                            parity[idx * 2 + 1] ^= (*circ >> 0) as u32;
                            parity[idx * 2 + 0] ^= (*circ >> 32) as u32;
                        }
                    }
                }
                // Now simulate the right-rotation of the generator by left-rotating the parity
                if b >= 32 {
                    for block in 0..r / b {
                        let parityblock = &mut parity[block * b / 32..(block + 1) * b / 32];
                        let mut carry = parityblock[0] >> 31;
                        for x in parityblock.iter_mut().rev() {
                            let c = *x >> 31;
                            *x = (*x << 1) | carry;
                            carry = c;
                        }
                    }
                } else if b == 16 {
                    // For small blocks we must rotate inside each parity word instead
                    for x in parity.iter_mut() {
                        let block1 = *x & 0xFFFF_0000;
                        let block2 = *x & 0x0000_FFFF;
                        *x = (((block1 << 1) | (block1 >> 15)) & 0xFFFF_0000)
                            | (((block2 << 1) | (block2 >> 15)) & 0x0000_FFFF);
                    }
                }
            }

            // Need to compensate for endianness
            for x in parity.iter_mut() {
                *x = x.to_be();
            }
        }

        // Return a &mut [u8] view on the codeword
        unsafe {
            slice::from_raw_parts_mut::<'a>(codeword.as_mut_ptr() as *mut u8, codeword.len() * 4)
        }
    }

    fn copy_encode<'a>(code: &LDPCCode, data: &[u8], codeword: &'a mut [Self]) -> &'a mut [u8] {
        let codeword_u8 = unsafe {
            slice::from_raw_parts_mut::<'a>(codeword.as_mut_ptr() as *mut u8, codeword.len() * 4)
        };
        codeword_u8[..data.len()].copy_from_slice(data);
        Self::encode(code, codeword)
    }

    fn bitlength() -> usize {
        32
    }
}

impl EncodeInto for u64 {
    fn encode<'a>(code: &LDPCCode, codeword: &'a mut [Self]) -> &'a mut [u8] {
        let k = code.k();
        let r = code.n() - code.k();
        let b = code.circulant_size();
        let gc = code.compact_generator();
        let row_len = r / 64;

        // Scope the split of codeword into (data, parity)
        {
            // Split codeword into data and parity sections and then zero the parity bits
            let (data, parity) = codeword.split_at_mut(k / 64);
            for x in parity.iter_mut() {
                *x = 0;
            }

            // We treat data as a &[u8] so we bit-index it correctly despite endianness
            let data = unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

            // For each rotation of the generator circulants
            for offset in 0..b {
                // For each row of circulants
                for crow in 0..k / b {
                    // Data bit (row of full generator matrix)
                    let bit = crow * b + offset;
                    if data[bit / 8] >> (7 - (bit % 8)) & 1 == 1 {
                        // If bit is set, XOR the generator constant in
                        for (idx, circ) in
                            gc[crow * row_len..(crow + 1) * row_len].iter().enumerate()
                        {
                            parity[idx] ^= *circ;
                        }
                    }
                }
                // Now simulate the right-rotation of the generator by left-rotating the parity
                if b >= 64 {
                    for block in 0..r / b {
                        let parityblock = &mut parity[block * b / 64..(block + 1) * b / 64];
                        let mut carry = parityblock[0] >> 63;
                        for x in parityblock.iter_mut().rev() {
                            let c = *x >> 63;
                            *x = (*x << 1) | carry;
                            carry = c;
                        }
                    }
                } else if b == 32 {
                    // For small blocks we must rotate inside each parity word instead
                    for x in parity.iter_mut() {
                        let block1 = *x & 0xFFFFFFFF_00000000;
                        let block2 = *x & 0x00000000_FFFFFFFF;
                        *x = (((block1 << 1) | (block1 >> 31)) & 0xFFFFFFFF_00000000)
                            | (((block2 << 1) | (block2 >> 31)) & 0x00000000_FFFFFFFF);
                    }
                } else if b == 16 {
                    for x in parity.iter_mut() {
                        let block1 = *x & 0xFFFF_0000_0000_0000;
                        let block2 = *x & 0x0000_FFFF_0000_0000;
                        let block3 = *x & 0x0000_0000_FFFF_0000;
                        let block4 = *x & 0x0000_0000_0000_FFFF;
                        *x = (((block1 << 1) | (block1 >> 15)) & 0xFFFF_0000_0000_0000)
                            | (((block2 << 1) | (block2 >> 15)) & 0x0000_FFFF_0000_0000)
                            | (((block3 << 1) | (block3 >> 15)) & 0x0000_0000_FFFF_0000)
                            | (((block4 << 1) | (block4 >> 15)) & 0x0000_0000_0000_FFFF);
                    }
                }
            }

            // Need to compensate for endianness
            for x in parity.iter_mut() {
                *x = x.to_be();
            }
        }

        // Return a &mut [u8] view on the codeword
        unsafe {
            slice::from_raw_parts_mut::<'a>(codeword.as_mut_ptr() as *mut u8, codeword.len() * 8)
        }
    }

    fn copy_encode<'a>(code: &LDPCCode, data: &[u8], codeword: &'a mut [Self]) -> &'a mut [u8] {
        let codeword_u8 = unsafe {
            slice::from_raw_parts_mut::<'a>(codeword.as_mut_ptr() as *mut u8, codeword.len() * 8)
        };
        codeword_u8[..data.len()].copy_from_slice(data);
        Self::encode(code, codeword)
    }

    fn bitlength() -> usize {
        64
    }
}

impl LDPCCode {
    /// Encode a codeword. This function assumes the first k bits of `codeword` have already
    /// been set to your data, and will set the remaining n-k bits appropriately.
    ///
    /// `codeword` must be exactly n bits long.
    ///
    /// You can give `codeword` in `u8`, `u32`, or `u64`.
    /// The larger types are faster and are interpreted as packed bytes in little endian.
    ///
    /// Returns a view of `codeword` in &mut [u8] which may be convenient if you
    /// passed in a larger type but want to use the output as bytes. You can just
    /// not use the return value if you wish to keep your original view on `codeword`.
    pub fn encode<'a, T>(&self, codeword: &'a mut [T]) -> &'a mut [u8]
    where
        T: EncodeInto,
    {
        assert_eq!(
            codeword.len() * T::bitlength(),
            self.n(),
            "codeword must be n bits long"
        );
        EncodeInto::encode(self, codeword)
    }

    /// Encode a codeword, first copying in the data.
    ///
    /// This is the same as `encode` except you can pass the data which must be k bits long in as
    /// `&[u8]` and it will be copied into the first part of `codeword`, which must be n bits long.
    ///
    /// Returns a view of `codeword` in &mut [u8] which may be convenient if you
    /// passed in a larger type but want to use the output as bytes. You can just
    /// not use the return value if you wish to keep your original view on `codeword`.
    pub fn copy_encode<'a, T>(&self, data: &[u8], codeword: &'a mut [T]) -> &'a mut [u8]
    where
        T: EncodeInto,
    {
        assert_eq!(data.len() * 8, self.k(), "data must be k bits long");
        assert_eq!(
            codeword.len() * T::bitlength(),
            self.n(),
            "codeword must be n bits long"
        );
        EncodeInto::copy_encode(self, data, codeword)
    }
}

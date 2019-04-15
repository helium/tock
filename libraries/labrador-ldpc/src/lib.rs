// Copyright 2017 Adam Greig
// Licensed under the MIT license, see LICENSE for details.

#![no_std]
#![warn(missing_docs)]

//! Labrador-LDPC implements a selection of LDPC error correcting codes,
//! including encoders and decoders.
//!
//! It is designed for use with other Labrador components but does not have any dependencies
//! on anything (including `std`) and thus may be used totally standalone. It is reasonably
//! efficient on both serious computers and on small embedded systems. Considerations have
//! been made to accommodate both use cases.
//!
//! No memory allocations are made inside this crate so most methods require you to pass in an
//! allocated block of memory for them to use. Check individual method documentation for further
//! details.
//!
//! ## Example
//!
//! ```
//! use labrador_ldpc::LDPCCode;
//!
//! // Pick the TC128 code, n=128 k=64
//! // (that's 8 bytes of user data encoded into 16 bytes)
//! let code = LDPCCode::TC128;
//!
//! // Generate some data to encode
//! let txdata: Vec<u8> = (0..8).collect();
//!
//! // Allocate memory for the encoded data
//! let mut txcode = vec![0u8; code.n()/8];
//!
//! // Encode, copying `txdata` into the start of `txcode` then computing the parity bits
//! code.copy_encode(&txdata, &mut txcode);
//!
//! // Copy the transmitted data and corrupt a few bits
//! let mut rxcode = txcode.clone();
//! rxcode[0] ^= 0x55;
//!
//! // Allocate some memory for the decoder's working area and output
//! let mut working = vec![0u8; code.decode_bf_working_len()];
//! let mut rxdata = vec![0u8; code.output_len()];
//!
//! // Decode for at most 20 iterations
//! code.decode_bf(&rxcode, &mut rxdata, &mut working, 20);
//!
//! // Check the errors got corrected
//! assert_eq!(&rxdata[..8], &txdata[..8]);
//! ```
//!
//! ## Codes
//!
//! *Nomenclature:* we use n to represent the code length (number of bits you have to
//! transmit per codeword), k to represent the code dimension (number of useful information bits
//! per codeword), and r to represent the *rate* k/n, the number of useful information bits per
//! bit transmitted.
//!
//! Several codes are available in a range of lengths and rates. Current codes come from two
//! sets of CCSDS recommendations, their TC (telecommand) short-length low-rate codes, and their
//! TM (telemetry) higher-length various-rates codes. These are all published and standardised
//! codes which have good performance.
//!
//! The TC codes are available in rate r=1/2 and dimensions k=128, k=256, and k=512.
//! They are the same codes defined in CCSDS document 231.1-O-1 and subsequent revisions (although
//! the n=256 code is eventually removed, it lives on here as it's quite useful).
//!
//! The TM codes are available in r=1/2, r=2/3, and r=4/5, for dimensions k=1024 and k=4096.
//! They are the same codes defined in CCSDS document 131.0-B-2 and subsequent revisions.
//!
//! For more information on the codes themselves please see the CCSDS publications:
//! https://public.ccsds.org/
//!
//! The available codes are the variants of the `LDPCCode` enum, and pretty much everything
//! else (encoders, decoders, utility methods) are implemented as methods on this enum.
//!
//! *Which code should I pick?*: for short and highly-reliable messages, the TC codes make sense,
//! especially if they need to be decoded on a constrained system such as an embedded platform.
//! For most other data transfer, the TM codes are more flexible and generally better suited.
//!
//! The very large k=16384 TM codes have not been included due to the complexity in generating
//! their generator matrices and the very long constants involved, but it would be theoretically
//! possible to include them. The relevant parity check constants are already included.
//!
//! ### Generator Matrices
//!
//! To encode a codeword, we need a generator matrix, which is a large binary matrix of shape
//! k rows by n columns. For each bit set in the data to encode, we sum the corresponding row
//! of the generator matrix to find the output codeword. Because all our codes are *systematic*,
//! the first k bits of our codewords are exactly the input data, which means we only need to
//! encode the final n-k parity bits at the end of the codeword.
//!
//! These final n-k columns of the generator are stored in a compact form, where only a small
//! number of the final rows are stored, and the rest can be inferred from those at runtime. Our
//! encoder methods just use this compact form directly, so it doesn't ever need to be expanded.
//!
//! The relevant constants are in the `codes.compact_generators` module, with names like `TC128_G`.
//!
//! ### Parity Check Matrices
//!
//! These are the counterpart to the generator matrices of the previous section. They are used by
//! the decoders to work out which bits are wrong and need to be changed. When fully expanded,
//! they are a large matrix with n-k rows (one per parity check) of n columns (one per input data
//! bit, or variable). We can store and use them in an extremely compact form due to the way these
//! specific codes have been constructed.
//!
//! The constants are in `codes.compact_parity_checks` and reflect the construction defined
//! in the CCSDS documents.
//!
//! ## Encoders
//!
//! There are two encoder methods implemented on `LDPCCode`: `encode` and `copy_encode`.
//!
//! `copy_encode` is a convenience wrapper that copies your data to encode into the codeword
//! memory first, and then performs the encode as usual. In comparison, `encode` requires that
//! your data is already at the start of the codeword memory, and just fills in the parity bits
//! at the end. It doesn't take very much time to do the copy, so use whichever is more convenient.
//!
//! The encode methods require you to pass in a slice of allocated codeword memory, `&mut [T]`,
//! which must be `n` bits long exactly. You can pass this as slices of `u8`, `u32`, or `u64`. In
//! general the larger types will encode up to three times faster, so it's usually worth using
//! them. They are interpreted as containing your data in little-endian, so you can directly
//! cast between the `&[u8]` and larger interpretations on all little-endian systems (which is to
//! say, most systems).
//!
//! The encode methods always return an `&mut [u8]` view on the codeword memory, which you
//! can use if you need this type for further use (such as transmission out of a radio), or if you
//! ignore the return value you can continue using your original slice of codeword memory.
//!
//! ```
//! # use labrador_ldpc::LDPCCode;
//! let code = LDPCCode::TC128;
//!
//! // Encode into u32, but then access results as u8
//! let mut codeword: [u32; 4] = [0x03020100, 0x07060504, 0x00000000, 0x00000000];
//! let txcode = code.encode(&mut codeword);
//! assert_eq!(txcode, [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
//!                     0x34, 0x99, 0x98, 0x87, 0x94, 0xE1, 0x62, 0x56]);
//!
//! // Encode into u64, but maintain access as a u64 afterwards
//! let mut codeword: [u64; 2] = [0x0706050403020100, 0x0000000000000000];
//! code.encode(&mut codeword);
//! assert_eq!(codeword, [0x0706050403020100, 0x5662E19487989934]);
//! ```
//!
//! The required memory (in bytes) to encode with each code is:
//!
//! Code   | Input (RAM) | Output (RAM)    | Generator const (text)
//! -------|-------------|-----------------|-----------------------
//!        | =k/8        | =n/8            |
//! TC128  |           8 |              16 |              32
//! TC256  |          16 |              32 |              64
//! TC512  |          32 |              64 |             128
//! TM1280 |         128 |             160 |            1024
//! TM1536 |         128 |             192 |            1024
//! TM2048 |         128 |             256 |            1024
//! TM5120 |         512 |             640 |            4096
//! TM6144 |         512 |             768 |            4096
//! TM8192 |         512 |            1024 |            4096
//!
//! ## Decoders
//!
//! There are two decoders available:
//!
//! * The low-memory decoder, `decode_bf`, uses a bit flipping algorithm with hard information.
//!   This is maybe 1 or 2dB from optimal for decoding, but requires much less RAM and is usually
//!   a few times faster. It's only really useful on something very slow or with very little memory
//!   available.
//! * The high-performance decoder, `decode_ms`, uses a modified min-sum decoding algorithm with
//!   soft information to perform near-optimal decoding albeit slower and with much higher memory
//!   overhead.  This decoder can operate on a variety of types for the soft information, with
//!   corresponding differences in the memory overhead.
//!
//! The required memory (in bytes) to decode with each code is:
//!
//! Code   | Hard input  | Soft input  |   Output | Parity const | `bf` overhead | `mp` overhead
//! -------|-------------|-------------|----------|--------------|---------------|---------------
//!        | (`bf`, RAM) | (`mp`, RAM) | (RAM)    | (text)       | (RAM)         | (RAM)
//!        | =n/8        | =n*T        | =(n+p)/8 |              |               |
//! TC128  |          16 |        128T |       16 |          132 |           128 | 1280T    +   8
//! TC256  |          32 |        256T |       32 |          132 |           256 | 2560T    +  16
//! TC512  |          64 |        512T |       64 |          132 |           512 | 5120T    +  32
//! TM1280 |         160 |       1280T |      176 |          366 |          1408 | 12160T   +  48
//! TM1536 |         192 |       1536T |      224 |          366 |          1792 | 15104T   +  96
//! TM2048 |         256 |       2048T |      320 |          366 |          2560 | 20992T   + 192
//! TM5120 |         640 |       5120T |      704 |          366 |          5632 | 48640T   + 192
//! TM6144 |         768 |       6144T |      896 |          366 |          7168 | 60416T   + 384
//! TM8192 |        1024 |       8192T |     1280 |          366 |         10240 | 83968T   + 768
//!
//! `T` reflects the size of the type for your soft information: for `i8` this is 1, for `i16` 2,
//! for `i32` and `f32` it's 4, and for `f64` it is 8. You should use a type commensurate with
//! the quality of your soft information; usually `i16` would suffice for instance.
//!
//! Both decoders require the same output storage and parity constants. The `bf` decoder takes
//! smaller hard inputs and has a much smaller working area, while the `mp` decoder requires
//! soft inputs and uses soft information internally, requiring a larger working area.
//!
//! The required sizes are available both at compile-time in the `CodeParams` consts, and at
//! runtime with methods on `LDPCCode` such as `decode_ms_working_len()`. You can therefore
//! allocate the required memory either statically or dynamically at runtime.
//!
//! Please see the individual decoder methods for more details on their requirements.
//!
//! ### Bit Flipping Decoder
//! This decoder is based on the original Gallagher decoder. It is not very optimal but is fast.
//! The idea is to see which bits are connected to the highest number of parity checks that are not
//! currently satisfied, and flip those bits, and iterate until things get better.
//! However, this routine cannot correct erasures (it only knows about bit flips). All of the TM
//! codes are *punctured*, which means some parity bits are not transmitted and so are unknown at
//! the receiver. We use a separate algorithm to decode the erasures first, based on a paper by
//! Archonta, Kanistras and Paliouras, doi:10.1109/MOCAST.2016.7495161.
//!
//! ### Message Passing Decoder
//! This is a modified min-sum decoder that computes the probability of each bit being set given
//! the other bits connected to it via the parity check matrix. It takes soft information in,
//! so inherently covers the punctured codes as well. This implementation is based on one described
//! by Savin, arXiv:0803.1090. It is both reasonably efficient (no `atahn` required), and
//! performs very close to optimal sum-product decoding.

#[cfg(test)]
#[macro_use]
extern crate std;

pub mod codes;
pub mod decoder;
pub mod encoder;
pub use codes::LDPCCode;

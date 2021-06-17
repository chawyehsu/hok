// references:
//   [SHA-2](https://en.wikipedia.org/wiki/SHA-2)
//   [golang crypto sha512](https://github.com/golang/go/blob/master/src/crypto/sha512/sha512.go)

use core::{cmp::min, convert::TryInto};

static INIT_STATE: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];
static K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

#[derive(Debug)]
pub struct Sha512 {
    // State A, B, C, D, E, F, G, H
    state: [u64; 8],
    // Hold total length of input data
    total_length: u128,
    // Store the last part of input data to be consumed
    buffer: [u8; 128],
    // Hold the length of the last part of input data that aren't consumed yet,
    // the `buflen` can only be from 0 to 128
    buflen: usize,
}

impl Sha512 {
    /// Create a new [`Sha512`] instance to consume data and get digest.
    #[inline]
    pub fn new() -> Self {
        Sha512 {
            state: INIT_STATE,
            total_length: 0,
            buffer: [0; 128],
            buflen: 0,
        }
    }

    /// Reset this [`Sha512`] instance's status.
    #[inline]
    pub fn reset(&mut self) {
        self.state = INIT_STATE;
        self.total_length = 0;
        self.buffer = [0; 128];
        self.buflen = 0;
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `[u8; 64]` array format.
    #[inline]
    pub fn result(&mut self) -> [u8; 64] {
        let len_mod = self.total_length % 128;
        let pad_idx = if 111 < len_mod {
            111 + 128 - len_mod
        } else {
            111 - len_mod
        };
        // transform the total length of all data to a 128-bit representation,
        // note that the length itself needs to be represented as `bits`, which
        // means a length of 1 needs to be tranformed to a 8 bits representation,
        // then extend the 8bits representation to the 128-bit long.
        //
        // To transform the length to `bits`, simply multiply it by 8. We use
        // left-shift operation here for a better perfermance.
        let total: [u8; 16] = (self.total_length << 3).to_be_bytes();

        self.consume([&[0x80u8], &[0x00; 127][..pad_idx as usize], &total].concat());

        self.state
            .iter()
            .map(|i| i.to_be_bytes())
            .collect::<Vec<_>>()
            .concat()
            .try_into()
            .unwrap()
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a [`String`] format.
    #[inline]
    pub fn result_string(&mut self) -> String {
        self.result()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Consume the input data, but not finalize the calculation. This
    /// method returns `&self` to make itself chainable, so that callers
    /// can continuously consume data by chaining function calls. for example:
    ///
    /// ```
    /// use scoop_hash::Sha512;
    /// let data1 = "hello".as_bytes();
    /// let data2 = "world".as_bytes();
    /// let hex_str = Sha512::new().consume(data1).consume(data2).result_string();
    /// assert_eq!(hex_str, "1594244d52f2d8c12b142bb61f47bc2eaf503d6d9ca8480cae9fcf112f66e4967dc5e8fa98285e36db8af1b8ffa8b84cb15e0fbcf836c3deb803c13f37659a60");
    /// ```
    pub fn consume<D: AsRef<[u8]>>(&mut self, data: D) -> &mut Self {
        let mut data = data.as_ref();
        let mut length = data.len();

        self.total_length += length as u128;

        if self.buflen > 0 {
            // The max `copied_idx` is 127
            let copied_idx = min(128 - self.buflen, length);

            // `copy_from_slice` requires extact the same length of two slices,
            // here we use `[self.buflen..self.buflen + copied_idx]` to narrow
            // down the length of slice `self.buffer`, and use `copied_idx` to
            // narrow down the length of slice `data`.
            self.buffer[self.buflen..self.buflen + copied_idx].copy_from_slice(&data[..copied_idx]);
            self.buflen += copied_idx;

            if self.buflen == 128 {
                self.compress(self.buffer);

                // clear buffer
                self.buflen = 0
            }

            // keep the remaining untransformed data
            data = &data[copied_idx..];
            length = data.len();
        }

        if length >= 128 {
            let split_idx = length & !127;
            data[..split_idx].chunks_exact(128).for_each(|block| {
                self.compress(block.try_into().unwrap());
            });

            // keep the remaining untransformed data
            data = &data[split_idx..];
            length = data.len();
        } // The `length` out here must be 0 <= length < 128

        if length > 0 {
            self.buffer[..length].copy_from_slice(data);
            self.buflen = length;
        }

        self
    }

    /// The [`compression function`]: transform a 1024 bits input block, which
    /// is split into 16 `words` (64 bits per word). Then each word is compressed
    /// into the state.
    ///
    /// [`compression function`]: https://en.wikipedia.org/wiki/One-way_compression_function
    #[inline]
    fn compress(&mut self, block: [u8; 128]) {
        // Create temp state variables for compression
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        let mut words = [0u64; 80];

        // Break block into the first sixteen 64-bit `big-endian` words
        block.chunks_exact(8).enumerate().for_each(|(i, s)| {
            words[i] = u64::from_be_bytes(s.try_into().unwrap());
        });

        // Extend the first 16 words into the remaining 64 words words[16..79]
        for i in 16..80 {
            let s0 = words[i - 15].rotate_right(1)
                ^ words[i - 15].rotate_right(8)
                ^ (words[i - 15] >> 7);
            let s1 =
                words[i - 2].rotate_right(19) ^ words[i - 2].rotate_right(61) ^ (words[i - 2] >> 6);

            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }

        // Compression function main loop
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(words[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // Update state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

#[cfg(test)]
mod tests {
    use super::Sha512;

    #[test]
    fn rfc_test_suite() {
        let inputs = [
            "",
            "a",
            "abc",
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "0123456701234567012345670123456701234567012345670123456701234567",
            "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        ];
        let outputs = [
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
            "1f40fc92da241694750979ee6cf582f2d5d7d28e18335de05abc54d0560e0f5302860c652bf08d560252aa5e74210546f369fbbbce8c12cfc7957b2652fe9a75",
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            "204a8fc6dda82f0a0ced7beb8e08a41657c16ef468b228a8279be331a703c33596fd15c13b1b07f9aa1d3bea57789ca031ad85c7a71dd70354ec631238ca3445",
            "846e0ef73436438a4acb0ba7078cfe381f10a0f5edebcb985b3790086ef5e7ac5992ac9c23c77761c764bb3b1c25702d06b99955eb197d45b82fb3d124699d78",
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
        ];

        for (input, &output) in inputs.iter().zip(outputs.iter()) {
            let computed = Sha512::new().consume(input.as_bytes()).result_string();
            assert_eq!(output, computed);
        }
    }

    #[test]
    fn chaining_consume() {
        let data1 = "hello".as_bytes();
        let data2 = "world".as_bytes();
        let hex_str = Sha512::new().consume(data1).consume(data2).result_string();
        // equal to `helloworld`
        assert_eq!(hex_str, "1594244d52f2d8c12b142bb61f47bc2eaf503d6d9ca8480cae9fcf112f66e4967dc5e8fa98285e36db8af1b8ffa8b84cb15e0fbcf836c3deb803c13f37659a60");
    }

    #[test]
    fn result() {
        let hex = Sha512::new().consume("".as_bytes()).result();
        assert_eq!(
            hex,
            [
                207, 131, 225, 53, 126, 239, 184, 189, 241, 84, 40, 80, 214, 109, 128, 7, 214, 32,
                228, 5, 11, 87, 21, 220, 131, 244, 169, 33, 211, 108, 233, 206, 71, 208, 209, 60,
                93, 133, 242, 176, 255, 131, 24, 210, 135, 126, 236, 47, 99, 185, 49, 189, 71, 65,
                122, 129, 165, 56, 50, 122, 249, 39, 218, 62
            ]
        );
    }

    #[test]
    fn reset() {
        let mut sha1 = Sha512::new();
        sha1.consume("".as_bytes());
        sha1.reset();
        let hex_str = sha1.consume("abc".as_bytes()).result_string();
        // equal to `abc`
        assert_eq!(hex_str, "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    }
}

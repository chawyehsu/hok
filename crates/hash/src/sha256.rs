// references:
//   [SHA-2](https://en.wikipedia.org/wiki/SHA-2)
//   [golang crypto sha256](https://github.com/golang/go/blob/master/src/crypto/sha256/sha256.go)

use core::{cmp::min, convert::TryInto};

static INIT_STATE: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];
static K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[derive(Debug)]
pub struct Sha256 {
    // State A, B, C, D, E, F, G, H
    state: [u32; 8],
    // Hold total length of input data
    total_length: u64,
    // Store the last part of input data to be consumed
    buffer: [u8; 64],
    // Hold the length of the last part of input data that aren't consumed yet,
    // the `buflen` can only be from 0 to 64
    buflen: usize,
}

impl Sha256 {
    /// Create a new [`Sha256`] instance to consume data and get digest.
    #[inline]
    pub fn new() -> Self {
        Sha256 {
            state: INIT_STATE,
            total_length: 0,
            buffer: [0; 64],
            buflen: 0,
        }
    }

    /// Reset this [`Sha256`] instance's status.
    #[inline]
    pub fn reset(&mut self) {
        self.state = INIT_STATE;
        self.total_length = 0;
        self.buffer = [0; 64];
        self.buflen = 0;
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `[u8; 32]` array format.
    #[inline]
    pub fn result(&mut self) -> [u8; 32] {
        let len_mod = self.total_length % 64;
        let pad_idx = if 55 < len_mod {
            55 + 64 - len_mod
        } else {
            55 - len_mod
        };
        // transform the total length of all data to a 64-bit representation,
        // note that the length itself needs to be represented as `bits`, which
        // means a length of 1 needs to be tranformed to a 8 bits representation,
        // then extend the 8bits representation to the 64-bit long.
        //
        // To transform the length to `bits`, simply multiply it by 8. We use
        // left-shift operation here for a better perfermance.
        let total: [u8; 8] = (self.total_length << 3).to_be_bytes();

        self.consume([&[0x80u8], &[0x00; 63][..pad_idx as usize], &total].concat());

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
    /// use scoop_hash::Sha256;
    /// let data1 = "hello".as_bytes();
    /// let data2 = "world".as_bytes();
    /// let hex_str = Sha256::new().consume(data1).consume(data2).result_string();
    /// assert_eq!(hex_str, "936a185caaa266bb9cbe981e9e05cb78cd732b0b3280eb944412bb6f8f8f07af");
    /// ```
    pub fn consume<D: AsRef<[u8]>>(&mut self, data: D) -> &mut Self {
        let mut data = data.as_ref();
        let mut length = data.len();

        self.total_length += length as u64;

        if self.buflen > 0 {
            // The max `copied_idx` is 63
            let copied_idx = min(64 - self.buflen, length);

            // `copy_from_slice` requires extact the same length of two slices,
            // here we use `[self.buflen..self.buflen + copied_idx]` to narrow
            // down the length of slice `self.buffer`, and use `copied_idx` to
            // narrow down the length of slice `data`.
            self.buffer[self.buflen..self.buflen + copied_idx].copy_from_slice(&data[..copied_idx]);
            self.buflen += copied_idx;

            if self.buflen == 64 {
                self.compress(self.buffer);

                // clear buffer
                self.buflen = 0
            }

            // keep the remaining untransformed data
            data = &data[copied_idx..];
            length = data.len();
        }

        if length >= 64 {
            let split_idx = length & !63;
            data[..split_idx].chunks_exact(64).for_each(|block| {
                self.compress(block.try_into().unwrap());
            });

            // keep the remaining untransformed data
            data = &data[split_idx..];
            length = data.len();
        } // The `length` out here must be 0 <= length < 64

        if length > 0 {
            self.buffer[..length].copy_from_slice(data);
            self.buflen = length;
        }

        self
    }

    /// The [`compression function`]: transform a 512 bits input block, which is
    /// split into 16 `words` (32 bits per word). Then each word is compressed
    /// into the state.
    ///
    /// [`compression function`]: https://en.wikipedia.org/wiki/One-way_compression_function
    #[inline]
    fn compress(&mut self, block: [u8; 64]) {
        // Create temp state variables for compression
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        let mut words = [0u32; 64];

        // Break block into the first sixteen 32-bit `big-endian` words
        block.chunks_exact(4).enumerate().for_each(|(i, s)| {
            words[i] = u32::from_be_bytes(s.try_into().unwrap());
        });

        // Extend the first 16 words into the remaining 48 words words[16..63]
        for i in 16..64 {
            let s0 = words[i - 15].rotate_right(7)
                ^ words[i - 15].rotate_right(18)
                ^ (words[i - 15] >> 3);
            let s1 = words[i - 2].rotate_right(17)
                ^ words[i - 2].rotate_right(19)
                ^ (words[i - 2] >> 10);

            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }

        // Compression function main loop
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(words[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
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
    use super::Sha256;

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
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            "8182cadb21af0e37c06414ece08e19c65bdb22c396d48ba7341012eea9ffdfdd",
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
        ];

        for (input, &output) in inputs.iter().zip(outputs.iter()) {
            let computed = Sha256::new().consume(input.as_bytes()).result_string();
            assert_eq!(output, computed);
        }
    }

    #[test]
    fn chaining_consume() {
        let data1 = "hello".as_bytes();
        let data2 = "world".as_bytes();
        let hex_str = Sha256::new().consume(data1).consume(data2).result_string();
        // equal to `helloworld`
        assert_eq!(
            hex_str,
            "936a185caaa266bb9cbe981e9e05cb78cd732b0b3280eb944412bb6f8f8f07af"
        );
    }

    #[test]
    fn result() {
        let hex = Sha256::new().consume("".as_bytes()).result();
        assert_eq!(
            hex,
            [
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85
            ]
        );
    }

    #[test]
    fn reset() {
        let mut sha1 = Sha256::new();
        sha1.consume("".as_bytes());
        sha1.reset();
        let hex_str = sha1.consume("abc".as_bytes()).result_string();
        // equal to `abc`
        assert_eq!(
            hex_str,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

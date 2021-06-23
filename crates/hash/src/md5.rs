// references:
//   [MD5](https://en.wikipedia.org/wiki/MD5)
//   [golang crypto md5](https://github.com/golang/go/blob/master/src/crypto/md5/md5.go)
//   [md-5](https://github.com/RustCrypto/hashes/)

use core::{cmp::min, convert::TryInto};

static INIT_STATE: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
static ROUND_TABLE: [u32; 64] = [
    // round 1
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    // round 2
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    // round 3
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    // round 4
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[derive(Debug)]
pub struct Md5 {
    // State A, B, C, D
    state: [u32; 4],
    // Hold total length of input data
    total_length: u64,
    // Store the last part of input data to be consumed
    buffer: [u8; 64],
    // Hold the length of the last part of input data that aren't consumed yet,
    // the `buflen` can only be from 0 to 64
    buflen: usize,
    finished: bool,
}

impl Md5 {
    /// Create a new [`Md5`] instance to consume data and get digest.
    #[inline]
    pub fn new() -> Self {
        Md5 {
            state: INIT_STATE,
            total_length: 0,
            buffer: [0; 64],
            buflen: 0,
            finished: false,
        }
    }

    /// Reset this [`Md5`] instance's status.
    #[inline]
    pub fn reset(&mut self) {
        self.state = INIT_STATE;
        self.total_length = 0;
        self.buffer = [0; 64];
        self.buflen = 0;
        self.finished = false;
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `[u8; 16]` array format.
    #[inline]
    pub fn result(&mut self) -> [u8; 16] {
        if !self.finished {
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
            let total: [u8; 8] = (self.total_length << 3).to_le_bytes();

            self.consume([&[0x80u8], &[0x00; 63][..pad_idx as usize], &total].concat());
            self.finished = true;
        }

        self.state
            .iter()
            .map(|i| i.to_le_bytes())
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
    /// use scoop_hash::Md5;
    /// let data1 = "hello".as_bytes();
    /// let data2 = "world".as_bytes();
    /// let hex_str = Md5::new().consume(data1).consume(data2).result_string();
    /// assert_eq!(hex_str, "fc5e038d38a57032085441e7fe7010b0");
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
        let [mut a, mut b, mut c, mut d] = self.state;

        // Break block into sixteen 32-bit `little-endian` words
        let mut words = [0u32; 16];
        for (o, s) in words.iter_mut().zip(block.chunks_exact(4)) {
            *o = u32::from_le_bytes(s.try_into().unwrap());
        }

        // round 1
        a = f(a, b, c, d, words[0], ROUND_TABLE[0], 7);
        d = f(d, a, b, c, words[1], ROUND_TABLE[1], 12);
        c = f(c, d, a, b, words[2], ROUND_TABLE[2], 17);
        b = f(b, c, d, a, words[3], ROUND_TABLE[3], 22);
        a = f(a, b, c, d, words[4], ROUND_TABLE[4], 7);
        d = f(d, a, b, c, words[5], ROUND_TABLE[5], 12);
        c = f(c, d, a, b, words[6], ROUND_TABLE[6], 17);
        b = f(b, c, d, a, words[7], ROUND_TABLE[7], 22);
        a = f(a, b, c, d, words[8], ROUND_TABLE[8], 7);
        d = f(d, a, b, c, words[9], ROUND_TABLE[9], 12);
        c = f(c, d, a, b, words[10], ROUND_TABLE[10], 17);
        b = f(b, c, d, a, words[11], ROUND_TABLE[11], 22);
        a = f(a, b, c, d, words[12], ROUND_TABLE[12], 7);
        d = f(d, a, b, c, words[13], ROUND_TABLE[13], 12);
        c = f(c, d, a, b, words[14], ROUND_TABLE[14], 17);
        b = f(b, c, d, a, words[15], ROUND_TABLE[15], 22);

        // round 2
        a = g(a, b, c, d, words[1], ROUND_TABLE[16], 5);
        d = g(d, a, b, c, words[6], ROUND_TABLE[17], 9);
        c = g(c, d, a, b, words[11], ROUND_TABLE[18], 14);
        b = g(b, c, d, a, words[0], ROUND_TABLE[19], 20);
        a = g(a, b, c, d, words[5], ROUND_TABLE[20], 5);
        d = g(d, a, b, c, words[10], ROUND_TABLE[21], 9);
        c = g(c, d, a, b, words[15], ROUND_TABLE[22], 14);
        b = g(b, c, d, a, words[4], ROUND_TABLE[23], 20);
        a = g(a, b, c, d, words[9], ROUND_TABLE[24], 5);
        d = g(d, a, b, c, words[14], ROUND_TABLE[25], 9);
        c = g(c, d, a, b, words[3], ROUND_TABLE[26], 14);
        b = g(b, c, d, a, words[8], ROUND_TABLE[27], 20);
        a = g(a, b, c, d, words[13], ROUND_TABLE[28], 5);
        d = g(d, a, b, c, words[2], ROUND_TABLE[29], 9);
        c = g(c, d, a, b, words[7], ROUND_TABLE[30], 14);
        b = g(b, c, d, a, words[12], ROUND_TABLE[31], 20);

        // round 3
        a = h(a, b, c, d, words[5], ROUND_TABLE[32], 4);
        d = h(d, a, b, c, words[8], ROUND_TABLE[33], 11);
        c = h(c, d, a, b, words[11], ROUND_TABLE[34], 16);
        b = h(b, c, d, a, words[14], ROUND_TABLE[35], 23);
        a = h(a, b, c, d, words[1], ROUND_TABLE[36], 4);
        d = h(d, a, b, c, words[4], ROUND_TABLE[37], 11);
        c = h(c, d, a, b, words[7], ROUND_TABLE[38], 16);
        b = h(b, c, d, a, words[10], ROUND_TABLE[39], 23);
        a = h(a, b, c, d, words[13], ROUND_TABLE[40], 4);
        d = h(d, a, b, c, words[0], ROUND_TABLE[41], 11);
        c = h(c, d, a, b, words[3], ROUND_TABLE[42], 16);
        b = h(b, c, d, a, words[6], ROUND_TABLE[43], 23);
        a = h(a, b, c, d, words[9], ROUND_TABLE[44], 4);
        d = h(d, a, b, c, words[12], ROUND_TABLE[45], 11);
        c = h(c, d, a, b, words[15], ROUND_TABLE[46], 16);
        b = h(b, c, d, a, words[2], ROUND_TABLE[47], 23);

        // round 4
        a = i(a, b, c, d, words[0], ROUND_TABLE[48], 6);
        d = i(d, a, b, c, words[7], ROUND_TABLE[49], 10);
        c = i(c, d, a, b, words[14], ROUND_TABLE[50], 15);
        b = i(b, c, d, a, words[5], ROUND_TABLE[51], 21);
        a = i(a, b, c, d, words[12], ROUND_TABLE[52], 6);
        d = i(d, a, b, c, words[3], ROUND_TABLE[53], 10);
        c = i(c, d, a, b, words[10], ROUND_TABLE[54], 15);
        b = i(b, c, d, a, words[1], ROUND_TABLE[55], 21);
        a = i(a, b, c, d, words[8], ROUND_TABLE[56], 6);
        d = i(d, a, b, c, words[15], ROUND_TABLE[57], 10);
        c = i(c, d, a, b, words[6], ROUND_TABLE[58], 15);
        b = i(b, c, d, a, words[13], ROUND_TABLE[59], 21);
        a = i(a, b, c, d, words[4], ROUND_TABLE[60], 6);
        d = i(d, a, b, c, words[11], ROUND_TABLE[61], 10);
        c = i(c, d, a, b, words[2], ROUND_TABLE[62], 15);
        b = i(b, c, d, a, words[9], ROUND_TABLE[63], 21);

        // Update state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

#[inline(always)]
fn f(w: u32, x: u32, y: u32, z: u32, m: u32, c: u32, s: u32) -> u32 {
    ((x & y) | (!x & z))
        .wrapping_add(w)
        .wrapping_add(m)
        .wrapping_add(c)
        .rotate_left(s)
        .wrapping_add(x)
}
#[inline(always)]
fn g(w: u32, x: u32, y: u32, z: u32, m: u32, c: u32, s: u32) -> u32 {
    ((x & z) | (y & !z))
        .wrapping_add(w)
        .wrapping_add(m)
        .wrapping_add(c)
        .rotate_left(s)
        .wrapping_add(x)
}

#[inline(always)]
fn h(w: u32, x: u32, y: u32, z: u32, m: u32, c: u32, s: u32) -> u32 {
    (x ^ y ^ z)
        .wrapping_add(w)
        .wrapping_add(m)
        .wrapping_add(c)
        .rotate_left(s)
        .wrapping_add(x)
}

#[inline(always)]
fn i(w: u32, x: u32, y: u32, z: u32, m: u32, c: u32, s: u32) -> u32 {
    (y ^ (x | !z))
        .wrapping_add(w)
        .wrapping_add(m)
        .wrapping_add(c)
        .rotate_left(s)
        .wrapping_add(x)
}

#[cfg(test)]
mod tests {
    use super::Md5;

    #[test]
    fn rfc_test_suite() {
        let inputs = [
            "",
            "a",
            "abc",
            "message digest",
            "abcdefghijklmnopqrstuvwxyz",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            "0123456789012345678901234567890123456789012345678901234567890123",
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        ];
        let outputs = [
            "d41d8cd98f00b204e9800998ecf8427e",
            "0cc175b9c0f1b6a831c399e269772661",
            "900150983cd24fb0d6963f7d28e17f72",
            "f96b697d7cb7938d525a2f31aaf161d0",
            "c3fcd3d76192e4007dfb496cca67e13b",
            "d174ab98d277d9f5a5611c2c9f419d9f",
            "7f7bfd348709deeaace19e3f535f8c54",
            "57edf4a22be3c955ac49da2e2107b67a",
        ];

        for (input, &output) in inputs.iter().zip(outputs.iter()) {
            let computed = Md5::new().consume(input.as_bytes()).result_string();
            assert_eq!(output, computed);
        }
    }

    #[test]
    fn chaining_consume() {
        let data1 = "hello".as_bytes();
        let data2 = "world".as_bytes();
        let hex_str = Md5::new().consume(data1).consume(data2).result_string();
        // equal to `helloworld`
        assert_eq!(hex_str, "fc5e038d38a57032085441e7fe7010b0");
    }

    #[test]
    fn result() {
        let hex = Md5::new().consume("".as_bytes()).result();
        assert_eq!(
            hex,
            [212, 29, 140, 217, 143, 0, 178, 4, 233, 128, 9, 152, 236, 248, 66, 126]
        );
    }

    #[test]
    fn reset() {
        let mut md5 = Md5::new();
        md5.consume("".as_bytes());
        md5.reset();
        let hex_str = md5.consume("a".as_bytes()).result_string();
        // equal to `a`
        assert_eq!(hex_str, "0cc175b9c0f1b6a831c399e269772661");
    }
}

// references:
//   [SHA-1](https://en.wikipedia.org/wiki/SHA-1)
//   [golang crypto sha1](https://github.com/golang/go/blob/master/src/crypto/sha1/sha1.go)

use core::{cmp::min, convert::TryInto};

static INIT_STATE: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
static K: [u32; 4] = [0x5a827999, 0x6ed9eba1, 0x8f1bbcdc, 0xca62c1d6];

#[derive(Debug)]
pub struct Sha1 {
    // State A, B, C, D, E
    state: [u32; 5],
    // Hold total length of input data
    total_length: u64,
    // Store the last part of input data to be consumed
    buffer: [u8; 64],
    // Hold the length of the last part of input data that aren't consumed yet,
    // the `buflen` can only be from 0 to 64
    buflen: usize,
}

impl Sha1 {
    /// Create a new [`Sha1`] instance to consume data and get digest.
    #[inline]
    pub fn new() -> Self {
        Sha1 {
            state: INIT_STATE,
            total_length: 0,
            buffer: [0; 64],
            buflen: 0,
        }
    }

    /// Reset this [`Sha1`] instance's status.
    #[inline]
    pub fn reset(&mut self) {
        self.state = INIT_STATE;
        self.total_length = 0;
        self.buffer = [0; 64];
        self.buflen = 0;
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `[u8; 20]` array format.
    #[inline]
    pub fn result(&mut self) -> [u8; 20] {
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
    /// use scoop_hash::Sha1;
    /// let data1 = "hello".as_bytes();
    /// let data2 = "world".as_bytes();
    /// let hex_str = Sha1::new().consume(data1).consume(data2).result_string();
    /// assert_eq!(hex_str, "6adfb183a4a2c94a2f92dab5ade762a47889a5a1");
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
        let [mut a, mut b, mut c, mut d, mut e] = self.state;

        // Break block into sixteen 32-bit `big-endian` words
        let mut words = [0u32; 16];
        for (o, s) in words.iter_mut().zip(block.chunks_exact(4)) {
            *o = u32::from_be_bytes(s.try_into().unwrap());
        }

        for i in 0..16 {
            let f = (b & c) | ((!b) & d);
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(words[i & 0xf])
                .wrapping_add(K[0]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }

        for i in 16..20 {
            let tmp = words[(i - 3) & 0xf]
                ^ words[(i - 8) & 0xf]
                ^ words[(i - 14) & 0xf]
                ^ words[i & 0xf];
            words[i & 0xf] = tmp << 1 | tmp >> (32 - 1);

            let f = (b & c) | ((!b) & d);
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(words[i & 0xf])
                .wrapping_add(K[0]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }

        for i in 20..40 {
            let tmp = words[(i - 3) & 0xf]
                ^ words[(i - 8) & 0xf]
                ^ words[(i - 14) & 0xf]
                ^ words[i & 0xf];
            words[i & 0xf] = tmp << 1 | tmp >> (32 - 1);

            let f = b ^ c ^ d;
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(words[i & 0xf])
                .wrapping_add(K[1]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }

        for i in 40..60 {
            let tmp = words[(i - 3) & 0xf]
                ^ words[(i - 8) & 0xf]
                ^ words[(i - 14) & 0xf]
                ^ words[i & 0xf];
            words[i & 0xf] = tmp << 1 | tmp >> (32 - 1);

            let f = ((b | c) & d) | (b & c);
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(words[i & 0xf])
                .wrapping_add(K[2]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }

        for i in 60..80 {
            let tmp = words[(i - 3) & 0xf]
                ^ words[(i - 8) & 0xf]
                ^ words[(i - 14) & 0xf]
                ^ words[i & 0xf];
            words[i & 0xf] = tmp << 1 | tmp >> (32 - 1);

            let f = b ^ c ^ d;
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(words[i & 0xf])
                .wrapping_add(K[3]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }

        // Update state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

#[cfg(test)]
mod tests {
    use super::Sha1;

    #[test]
    fn rfc_test_suite() {
        let inputs = [
            "",
            "a",
            "abc",
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "0123456701234567012345670123456701234567012345670123456701234567",
        ];
        let outputs = [
            "da39a3ee5e6b4b0d3255bfef95601890afd80709",
            "86f7e437faa5a7fce15d1ddcb9eaeaea377667b8",
            "a9993e364706816aba3e25717850c26c9cd0d89d",
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            "e0c094e867ef46c350ef54a7f59dd60bed92ae83",
        ];

        for (input, &output) in inputs.iter().zip(outputs.iter()) {
            let computed = Sha1::new().consume(input.as_bytes()).result_string();
            assert_eq!(output, computed);
        }
    }

    #[test]
    fn chaining_consume() {
        let data1 = "hello".as_bytes();
        let data2 = "world".as_bytes();
        let hex_str = Sha1::new().consume(data1).consume(data2).result_string();
        // equal to `helloworld`
        assert_eq!(hex_str, "6adfb183a4a2c94a2f92dab5ade762a47889a5a1");
    }

    #[test]
    fn result() {
        let hex = Sha1::new().consume("".as_bytes()).result();
        assert_eq!(
            hex,
            [
                218, 57, 163, 238, 94, 107, 75, 13, 50, 85, 191, 239, 149, 96, 24, 144, 175, 216,
                7, 9
            ]
        );
    }

    #[test]
    fn reset() {
        let mut sha1 = Sha1::new();
        sha1.consume("".as_bytes());
        sha1.reset();
        let hex_str = sha1.consume("abc".as_bytes()).result_string();
        // equal to `abc`
        assert_eq!(hex_str, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}

use std::{cmp::min, convert::TryInto};

// Padding table
const PADDING: [u8; 63] = [0x00; 63];

macro_rules! add(
    ($a:expr, $b:expr) => ($a.wrapping_add($b));
);

#[derive(Debug)]
pub struct Md5 {
    // State A, B, C, D
    state: [u32; 4],
    // Hold total length of input data
    total_length: usize,
    // Store the last part of input data to be consumed
    buffer: [u8; 64],
    // Hold the length of the last part of input data that aren't consumed yet,
    // the `buflen` can only be from 0 to 64
    buflen: usize,
}

impl Md5 {
    pub fn new() -> Self {
        Md5 {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            total_length: 0,
            buffer: [0; 64],
            buflen: 0,
        }
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `[u8; 16]` array format.
    pub fn result(&mut self) -> [u8; 16] {
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

        let a: Vec<u8> = [&[0x80u8], &PADDING[..pad_idx], &total].concat();
        self.consume(&a);

        self.state
            .iter()
            .map(|i| i.to_le_bytes())
            .collect::<Vec<_>>()
            .concat()
            .try_into()
            .unwrap()
    }

    /// Consume the last buffer data, finalize the calculation and return
    /// the digest as a `String` format.
    pub fn result_string(&mut self) -> String {
        self.result()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Consume the input data, but not finalize the calculation. This
    /// method returns `&self` to make itself chainable, so that callers
    /// can continuously consume data by chaining functions calls. for example:
    ///
    /// ```
    /// use scoop_hash::md5::Md5;
    /// let data1 = "hello".as_bytes();
    /// let data2 = "world".as_bytes();
    /// let hex_str = Md5::new().consume(data1).consume(data2).result();
    /// assert_eq!(hex_str, "fc5e038d38a57032085441e7fe7010b0");
    /// ```
    pub fn consume(&mut self, data: &[u8]) -> &mut Self {
        let mut data = data;
        let mut length = data.len();

        self.total_length += length;

        if self.buflen > 0 {
            // The max `copied_len` is 63
            let copied_len = min(64 - self.buflen, length);

            // `copy_from_slice` requires extact the same length of two slices,
            // here we use `[self.buflen..self.buflen + copied_len]` to narrow
            // down the length of slice `self.buffer`, and use `copied_len` to
            // narrow down the length of slice `data`.
            self.buffer[self.buflen..self.buflen + copied_len].copy_from_slice(&data[..copied_len]);
            self.buflen += copied_len;

            if self.buflen == 64 {
                let buf_block = self.buffer;
                self.transform(&buf_block);

                // clear buflen
                self.buflen = 0
            }

            // keep the remaining untransformed data
            data = &data[copied_len..];
            length = data.len();
        }

        if length >= 64 {
            let split_idx = length & !63;
            self.transform(&data[..split_idx]);

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

    /// Transform input blocks, which has a 0 modulo 64 length and can be split
    /// into multiple 64 bytes (i.e. 512 bits) chunks. And each 512 bits chunk
    /// will then be split into 16 `words`, which is a 32 bits long operand
    /// defined in the RFC1321.
    fn transform(&mut self, blocks: &[u8]) {
        blocks.chunks(64).for_each(|chunk| {
            // Create temp state variables for transformation
            let [mut a, mut b, mut c, mut d] = self.state;

            let words: [u32; 16] = chunk
                .chunks(4)
                // This is safe because a word must be 4 bytes (i.e. 32 bits) long.
                .map(|s| u32::from_ne_bytes(s.try_into().unwrap()).to_le())
                .collect::<Vec<_>>()
                // And this is safe too, because a chunk must contains 16 words.
                .try_into()
                .ok()
                .unwrap();

            // Round 1
            a = add!(b, (add!(add!(add!(((c ^ d) & b) ^ d, a), words[0]), 0xd76aa478)).rotate_left(7));
            d = add!(a, (add!(add!(add!(((b ^ c) & a) ^ c, d), words[1]), 0xe8c7b756)).rotate_left(12));
            c = add!(d, (add!(add!(add!(((a ^ b) & d) ^ b, c), words[2]), 0x242070db)).rotate_left(17));
            b = add!(c, (add!(add!(add!(((d ^ a) & c) ^ a, b), words[3]), 0xc1bdceee)).rotate_left(22));
            a = add!(b, (add!(add!(add!(((c ^ d) & b) ^ d, a), words[4]), 0xf57c0faf)).rotate_left(7));
            d = add!(a, (add!(add!(add!(((b ^ c) & a) ^ c, d), words[5]), 0x4787c62a)).rotate_left(12));
            c = add!(d, (add!(add!(add!(((a ^ b) & d) ^ b, c), words[6]), 0xa8304613)).rotate_left(17));
            b = add!(c, (add!(add!(add!(((d ^ a) & c) ^ a, b), words[7]), 0xfd469501)).rotate_left(22));
            a = add!(b, (add!(add!(add!(((c ^ d) & b) ^ d, a), words[8]), 0x698098d8)).rotate_left(7));
            d = add!(a, (add!(add!(add!(((b ^ c) & a) ^ c, d), words[9]), 0x8b44f7af)).rotate_left(12));
            c = add!(d, (add!(add!(add!(((a ^ b) & d) ^ b, c), words[10]), 0xffff5bb1)).rotate_left(17));
            b = add!(c, (add!(add!(add!(((d ^ a) & c) ^ a, b), words[11]), 0x895cd7be)).rotate_left(22));
            a = add!(b, (add!(add!(add!(((c ^ d) & b) ^ d, a), words[12]), 0x6b901122)).rotate_left(7));
            d = add!(a, (add!(add!(add!(((b ^ c) & a) ^ c, d), words[13]), 0xfd987193)).rotate_left(12));
            c = add!(d, (add!(add!(add!(((a ^ b) & d) ^ b, c), words[14]), 0xa679438e)).rotate_left(17));
            b = add!(c, (add!(add!(add!(((d ^ a) & c) ^ a, b), words[15]), 0x49b40821)).rotate_left(22));

            // Round 2
            a = add!(b, (add!(add!(add!(((b ^ c) & d) ^ c, a), words[1]), 0xf61e2562)).rotate_left(5));
            d = add!(a, (add!(add!(add!(((a ^ b) & c) ^ b, d), words[6]), 0xc040b340)).rotate_left(9));
            c = add!(d, (add!(add!(add!(((d ^ a) & b) ^ a, c), words[11]), 0x265e5a51)).rotate_left(14));
            b = add!(c, (add!(add!(add!(((c ^ d) & a) ^ d, b), words[0]), 0xe9b6c7aa)).rotate_left(20));
            a = add!(b, (add!(add!(add!(((b ^ c) & d) ^ c, a), words[5]), 0xd62f105d)).rotate_left(5));
            d = add!(a, (add!(add!(add!(((a ^ b) & c) ^ b, d), words[10]), 0x02441453)).rotate_left(9));
            c = add!(d, (add!(add!(add!(((d ^ a) & b) ^ a, c), words[15]), 0xd8a1e681)).rotate_left(14));
            b = add!(c, (add!(add!(add!(((c ^ d) & a) ^ d, b), words[4]), 0xe7d3fbc8)).rotate_left(20));
            a = add!(b, (add!(add!(add!(((b ^ c) & d) ^ c, a), words[9]), 0x21e1cde6)).rotate_left(5));
            d = add!(a, (add!(add!(add!(((a ^ b) & c) ^ b, d), words[14]), 0xc33707d6)).rotate_left(9));
            c = add!(d, (add!(add!(add!(((d ^ a) & b) ^ a, c), words[3]), 0xf4d50d87)).rotate_left(14));
            b = add!(c, (add!(add!(add!(((c ^ d) & a) ^ d, b), words[8]), 0x455a14ed)).rotate_left(20));
            a = add!(b, (add!(add!(add!(((b ^ c) & d) ^ c, a), words[13]), 0xa9e3e905)).rotate_left(5));
            d = add!(a, (add!(add!(add!(((a ^ b) & c) ^ b, d), words[2]), 0xfcefa3f8)).rotate_left(9));
            c = add!(d, (add!(add!(add!(((d ^ a) & b) ^ a, c), words[7]), 0x676f02d9)).rotate_left(14));
            b = add!(c, (add!(add!(add!(((c ^ d) & a) ^ d, b), words[12]), 0x8d2a4c8a)).rotate_left(20));

            // Round 3
            a = add!(b, (add!(add!(add!(b ^ c ^ d, a), words[5]), 0xfffa3942)).rotate_left(4));
            d = add!(a, (add!(add!(add!(a ^ b ^ c, d), words[8]), 0x8771f681)).rotate_left(11));
            c = add!(d, (add!(add!(add!(d ^ a ^ b, c), words[11]), 0x6d9d6122)).rotate_left(16));
            b = add!(c, (add!(add!(add!(c ^ d ^ a, b), words[14]), 0xfde5380c)).rotate_left(23));
            a = add!(b, (add!(add!(add!(b ^ c ^ d, a), words[1]), 0xa4beea44)).rotate_left(4));
            d = add!(a, (add!(add!(add!(a ^ b ^ c, d), words[4]), 0x4bdecfa9)).rotate_left(11));
            c = add!(d, (add!(add!(add!(d ^ a ^ b, c), words[7]), 0xf6bb4b60)).rotate_left(16));
            b = add!(c, (add!(add!(add!(c ^ d ^ a, b), words[10]), 0xbebfbc70)).rotate_left(23));
            a = add!(b, (add!(add!(add!(b ^ c ^ d, a), words[13]), 0x289b7ec6)).rotate_left(4));
            d = add!(a, (add!(add!(add!(a ^ b ^ c, d), words[0]), 0xeaa127fa)).rotate_left(11));
            c = add!(d, (add!(add!(add!(d ^ a ^ b, c), words[3]), 0xd4ef3085)).rotate_left(16));
            b = add!(c, (add!(add!(add!(c ^ d ^ a, b), words[6]), 0x04881d05)).rotate_left(23));
            a = add!(b, (add!(add!(add!(b ^ c ^ d, a), words[9]), 0xd9d4d039)).rotate_left(4));
            d = add!(a, (add!(add!(add!(a ^ b ^ c, d), words[12]), 0xe6db99e5)).rotate_left(11));
            c = add!(d, (add!(add!(add!(d ^ a ^ b, c), words[15]), 0x1fa27cf8)).rotate_left(16));
            b = add!(c, (add!(add!(add!(c ^ d ^ a, b), words[2]), 0xc4ac5665)).rotate_left(23));

            // Round 4
            a = add!(b, (add!(add!(add!(c ^ (b | !d), a), words[0]), 0xf4292244)).rotate_left(6));
            d = add!(a, (add!(add!(add!(b ^ (a | !c), d), words[7]), 0x432aff97)).rotate_left(10));
            c = add!(d, (add!(add!(add!(a ^ (d | !b), c), words[14]), 0xab9423a7)).rotate_left(15));
            b = add!(c, (add!(add!(add!(d ^ (c | !a), b), words[5]), 0xfc93a039)).rotate_left(21));
            a = add!(b, (add!(add!(add!(c ^ (b | !d), a), words[12]), 0x655b59c3)).rotate_left(6));
            d = add!(a, (add!(add!(add!(b ^ (a | !c), d), words[3]), 0x8f0ccc92)).rotate_left(10));
            c = add!(d, (add!(add!(add!(a ^ (d | !b), c), words[10]), 0xffeff47d)).rotate_left(15));
            b = add!(c, (add!(add!(add!(d ^ (c | !a), b), words[1]), 0x85845dd1)).rotate_left(21));
            a = add!(b, (add!(add!(add!(c ^ (b | !d), a), words[8]), 0x6fa87e4f)).rotate_left(6));
            d = add!(a, (add!(add!(add!(b ^ (a | !c), d), words[15]), 0xfe2ce6e0)).rotate_left(10));
            c = add!(d, (add!(add!(add!(a ^ (d | !b), c), words[6]), 0xa3014314)).rotate_left(15));
            b = add!(c, (add!(add!(add!(d ^ (c | !a), b), words[13]), 0x4e0811a1)).rotate_left(21));
            a = add!(b, (add!(add!(add!(c ^ (b | !d), a), words[4]), 0xf7537e82)).rotate_left(6));
            d = add!(a, (add!(add!(add!(b ^ (a | !c), d), words[11]), 0xbd3af235)).rotate_left(10));
            c = add!(d, (add!(add!(add!(a ^ (d | !b), c), words[2]), 0x2ad7d2bb)).rotate_left(15));
            b = add!(c, (add!(add!(add!(d ^ (c | !a), b), words[9]), 0xeb86d391)).rotate_left(21));

            // Update state
            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
        });
    }
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
    fn chaining_comsume() {
        let data1 = "hello".as_bytes();
        let data2 = "world".as_bytes();
        let hex_str = Md5::new().consume(data1).consume(data2).result_string();
        // equal to `helloworld`
        assert_eq!(hex_str, "fc5e038d38a57032085441e7fe7010b0");
    }
}

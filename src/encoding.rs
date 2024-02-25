pub fn encode(state: &[[bool; 6]; 5]) -> [u8; 4] {
    let mut encoded = [0; 4];
    let mut flattened = [false; 6 * 5];
    for ri in 0..5 {
        for ci in 0..6 {
            flattened[ri + ci * 5] = state[ri][ci];
        }
    }
    for (outidx, chunk) in flattened.chunks(8).enumerate() {
        let mut packed = 0;
        for i in 0..chunk.len() {
            packed += (chunk[i] as u8) << (i as u8);
        }
        encoded[outidx] = packed;
    }
    encoded
}

pub fn decode(encoded: &[u8; 4], state: &mut [[bool; 6]; 5]) {
    let mut flattened = [false; 6 * 5];
    for (i, byte) in encoded.iter().enumerate() {
        let mut byte = *byte;
        let mut offset = 0;
        while byte != 0 {
            if (byte & 1) == 1 {
                flattened[i * 8 + offset] = true;
            }
            offset += 1;
            byte >>= 1;
        }
    }
    for ri in 0..5 {
        for ci in 0..6 {
            state[ri][ci] = flattened[ri + ci * 5];
        }
    }
}

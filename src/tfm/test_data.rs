#[cfg(test)]
#[rustfmt::skip]
pub static BASIC_TFM: [u8; 160] = [
    // File length,
    0x00, 0x28,
    // Header length
    0x00, 0x12,
    // First character ('a')
    0x00, 0x61,
    // Last character ('a')
    0x00, 0x61,
    // Number of widths
    0x00, 0x02,
    // Number of heights,
    0x00, 0x02,
    // Number of depths
    0x00, 0x02,
    // Number of italic correctionx
    0x00, 0x02,
    // Number of Lig/Kern program steps
    0x00, 0x00,
    // Number of Kerns
    0x00, 0x00,
    // Number of extensible character recipes
    0x00, 0x00,
    // Number of font params
    0x00, 0x07,

    // Header
    0xAB, 0xCD, 0xEF, 0xAB,
    0b0000_0000, 0b1010_0000, 0b0000_0000, 0b0000_0000,
    0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6E, 0x67, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x07, 0x68, 0x69, 0x20, 0x70, 0x61, 0x72, 0x63, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0b1000_0000, 0x00, 0x00, 0xab,

    // The single character
    0b0000_0001, 0b0001_0001, 0b0000_0100, 0b0000_0000,

    // 2 widths
    0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0011_1000, 0b0000_0000, 0b0000_0000,

    // 2 heights
    0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0101_1000, 0b0000_0000, 0b0000_0000,

    // 2 depths
    0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0000_1000, 0b0000_0000, 0b0000_0000,

    // 2 italic corrections
    0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0000_0100, 0b0000_0000, 0b0000_0000,

    // font params (note: no lig/kern, kerns, or ext recipes)
    0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0100_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0001_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0010_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0101_1000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0100_0000, 0b0000_0000, 0b0000_0000,
    0b0000_0000, 0b0001_0000, 0b0000_0000, 0b0000_0000,
];

//Contains utility functions, such as byte manipulation functions

//constructs word from 2 bytes
#[inline(always)]
pub fn word(high: u8, low: u8) -> u16 {
    ((high as u16) << 8) | (low as u16)
}

//gets most significant byte of a word
#[inline(always)]
pub fn hb(word: u16) -> u8 {
    (word >> 8) as u8
}

//gets least significant byte of a word
#[inline(always)]
pub fn lb(word: u16) -> u8 {
    word as u8
}


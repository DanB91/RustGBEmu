/*
use std::ops::*;
use std::cmp::*;
use std::num::Int;
pub struct WrappingInt<T: Int>(pub T);

pub type Byte = WrappingInt<u8>;
pub type Word = WrappingInt<u16>;

#[inline(always)]
pub fn Word(val: u16) -> Word {
    WrappingInt(val)
}

#[inline(always)]
pub fn Byte(val: u8) -> Byte {
    WrappingInt(val)
}

*/
#[inline(always)]
pub fn makeWord(high: u8, low: u8) -> u16 {
    ((high as u16) << 8) | (low as u16)
}

#[inline(always)]
pub fn hb(word: u16) -> u8 {
    (word >> 8) as u8
}

#[inline(always)]
pub fn lb(word: u16) -> u8 {
    word as u8
}
/*
impl<T: Int> Sub<T> for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn sub(self, rhs: T) -> Self {
        self.0 = self.0.wrapping_sub(rhs);
        self
    }
}


impl<T: Int> Sub for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn sub(self, rhs: Self) -> Self {
        self.0 = self.0.wrapping_sub(rhs.0);
        self
    }
}

impl<T: Int> Add<T> for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn add(self, rhs: T) -> Self {
        self.0 = self.0.wrapping_add(rhs);
        self
    }
}


impl<T: Int> Add for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn add(self, rhs: Self) -> Self {
        self.0 = self.0.wrapping_add(rhs.0);
        self
    }
}

impl<T: Int> Shl<usize> for WrappingInt<T> {
    
    type Output = WrappingInt<T>;

    pub fn shl(self, rhs: usize) -> Self {
        self.0 = self.0 << rhs;
        self
    }
}

impl<T: Int> Shr<usize> for WrappingInt<T> {
    
    type Output = WrappingInt<T>;

    pub fn shr(self, rhs: usize) -> Self {
        self.0 = self.0 >> rhs;
        self
    }
}

impl<T: Int> BitOr for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn bitor(self, rhs: Self) -> Self {
        self.0 = self.0 | rhs.0;
        self
    }
}

impl<T: Int> BitOr<T> for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn bitor(self, rhs: T) -> Self {
        self.0 = self.0 | rhs;
        self
    }
}
impl<T: Int> BitAnd for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn bitand(self, rhs: Self) -> Self {
        self.0 = self.0 & rhs.0;
        self
    }
}

impl<T: Int> BitAnd<T> for WrappingInt<T> {

    type Output = WrappingInt<T>;

    pub fn bitand(self, rhs: T) -> Self {
        self.0 = self.0 & rhs;
        self
    }
}

impl<T: Int> PartialEq<T> for WrappingInt<T> {
    fn eq(&self, rhs: &T) -> bool {
        self.0 == *rhs
    }
}

impl<T: Int> PartialEq for WrappingInt<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.0 == rhs.0
    }
}
impl<T: Int> Eq for WrappingInt<T> {

}

impl<T: Int> PartialOrd<T> for WrappingInt<T> {

    fn partial_cmp(&self, other: &T) -> Option<Ordering> {

        if self.0 < *other {
            Some(Ordering::Less)
        }
        else if self.0 > *other {
            Some(Ordering::Greater)
        }
        else {
            Some(Ordering::Equal)
        }
    }
}
*/

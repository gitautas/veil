use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelScale(pub u32, pub u32);

impl PixelScale {
    pub const ONE: Self = Self(1, 1);

    pub fn is_fractional(&self) -> bool {
        (self.0 % self.1) != 0
    }

    pub fn ceil(self) -> Self {
        Self(self.0.next_multiple_of(self.1), self.1)
    }
}

impl std::fmt::Display for PixelScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0 as f64 / self.1 as f64)
    }
}

impl Default for PixelScale {
    fn default() -> Self {
        PixelScale::ONE
    }
}

#[derive(Debug, Clone)]
pub struct FractionalScaleError;

impl fmt::Display for FractionalScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scale is fractional")
    }
}

impl From<PixelScale> for f64 {
    fn from(value: PixelScale) -> Self {
        value.0 as f64 / value.1 as f64
    }
}

impl TryFrom<PixelScale> for u32 {
    type Error = FractionalScaleError;

    fn try_from(value: PixelScale) -> Result<Self, Self::Error> {
        if value.is_fractional() {
            return Err(FractionalScaleError);
        }

        Ok(value.0 / value.1)
    }
}


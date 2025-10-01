/// Some way to generate an OTP.
pub enum OneTimePasswordGenerator {
    Static(u32),
}

impl OneTimePasswordGenerator {
    /// Returns the current OTP (as a decimal number), or
    /// `None` if and only if the current OTP has been returned before.
    ///
    /// This means that the same OTP will not be returned twice.
    /// After calling this function, until the OTP algorithm generates a new OTP,
    /// there will be no OTP.
    ///
    /// However, this function may still return `Some(x)` and `Some(y)`
    /// back to back where `x == y`, as the OTP algorithm may simply
    /// generate the same OTP twice for two different times.
    pub fn get_current_otp(&mut self) -> Option<u32> {
        match self {
            Self::Static(pin) => Some(*pin),
        }
    }
}

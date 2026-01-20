use anyhow::{Result, anyhow};

pub fn bcd_to_decimal(value: u8) -> Result<u8> {
    let tens = value >> 4;
    let units = value & 0xF;
    if tens >= 10 || units >= 10 {
        Err(anyhow!("Could not unpack BCD value: {:#02x}", value))
    } else {
        Ok(tens * 10 + units)
    }
}

pub fn decimal_to_bcd(value: u8) -> Result<u8> {
    if value > 99 {
        Err(anyhow!("Could not pack value to BCD: {}", value))
    } else {
        let tens = value / 10;
        let units = value % 10;
        Ok(tens << 4 | units)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_to_decimal() {
        assert_eq!(bcd_to_decimal(0).unwrap(), 0);
        assert_eq!(bcd_to_decimal(0x15).unwrap(), 15);
        assert_eq!(bcd_to_decimal(0x99).unwrap(), 99);
    }

    #[test]
    fn test_decimal_to_bcd() {
        assert_eq!(decimal_to_bcd(0).unwrap(), 0);
        assert_eq!(decimal_to_bcd(15).unwrap(), 0x15);
        assert_eq!(decimal_to_bcd(99).unwrap(), 0x99);
    }
}

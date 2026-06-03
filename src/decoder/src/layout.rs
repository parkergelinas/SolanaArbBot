//! Fixed-layout byte readers for account decoders.

use common::{Error, Pubkey, Result};

pub(crate) fn read_u16(data: &[u8], offset: usize, decoder: &'static str) -> Result<u16> {
    let bytes = read_array::<2>(data, offset, decoder)?;
    Ok(u16::from_le_bytes(bytes))
}

pub(crate) fn read_i32(data: &[u8], offset: usize, decoder: &'static str) -> Result<i32> {
    let bytes = read_array::<4>(data, offset, decoder)?;
    Ok(i32::from_le_bytes(bytes))
}

pub(crate) fn read_u64(data: &[u8], offset: usize, decoder: &'static str) -> Result<u64> {
    let bytes = read_array::<8>(data, offset, decoder)?;
    Ok(u64::from_le_bytes(bytes))
}

pub(crate) fn read_u128(data: &[u8], offset: usize, decoder: &'static str) -> Result<u128> {
    let bytes = read_array::<16>(data, offset, decoder)?;
    Ok(u128::from_le_bytes(bytes))
}

pub(crate) fn read_pubkey(data: &[u8], offset: usize, decoder: &'static str) -> Result<Pubkey> {
    let bytes = read_array::<32>(data, offset, decoder)?;
    Ok(Pubkey::new(bytes))
}

fn read_array<const N: usize>(
    data: &[u8],
    offset: usize,
    decoder: &'static str,
) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| Error::DecodeError(format!("{decoder} offset overflow")))?;
    let bytes = data.get(offset..end).ok_or_else(|| {
        Error::DecodeError(format!(
            "{decoder} input too short: expected at least {end} bytes, got {}",
            data.len()
        ))
    })?;

    let mut out = [0; N];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::read_u64;
    use common::Error;

    #[test]
    fn read_u64_rejects_short_input() {
        let err = read_u64(&[1, 2, 3], 0, "test").expect_err("short input");

        assert!(matches!(err, Error::DecodeError(_)));
    }
}

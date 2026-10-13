use crate::document::EditorError;

const MAX_DECIMALS: u32 = 8;
const FLOAT_FRACTION_BITS: u32 = 52;
const FLOAT_EXPONENT_MASK: u64 = 0x7ff;
const FLOAT_EXPONENT_BIAS: i32 = 1023;
const DECIMAL_BASE: u128 = 10;
const SCIENTIFIC_THRESHOLD: f64 = 1e21;
const RGB_SHORT_DIVISOR: u8 = 17;
const ALPHA_DECIMALS: u32 = 8;

pub(crate) fn number(value: f64, decimals: u32) -> Result<String, EditorError> {
    if !value.is_finite() {
        return Err(EditorError::Refused);
    }
    if decimals > MAX_DECIMALS {
        return Err(EditorError::Capacity);
    }
    let absolute = value.abs();
    if absolute >= SCIENTIFIC_THRESHOLD {
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e').ok_or(EditorError::Refused)?;
        let exponent = exponent.parse::<i32>().map_err(|_| EditorError::Refused)?;
        return Ok(format!("{mantissa}e{exponent:+}"));
    }
    let bits = absolute.to_bits();
    let exponent = ((bits >> FLOAT_FRACTION_BITS) & FLOAT_EXPONENT_MASK) as i32;
    let fraction_mask = (1u64 << FLOAT_FRACTION_BITS) - 1;
    let fraction = bits & fraction_mask;
    let (significand, exponent) = if exponent == 0 {
        (
            fraction,
            1 - FLOAT_EXPONENT_BIAS - FLOAT_FRACTION_BITS as i32,
        )
    } else {
        (
            fraction | 1u64 << FLOAT_FRACTION_BITS,
            exponent - FLOAT_EXPONENT_BIAS - FLOAT_FRACTION_BITS as i32,
        )
    };
    let numerator = u128::from(significand)
        .checked_mul(DECIMAL_BASE.pow(decimals))
        .ok_or(EditorError::Capacity)?;
    let rounded = if exponent >= 0 {
        numerator
            .checked_mul(
                1u128
                    .checked_shl(exponent as u32)
                    .ok_or(EditorError::Capacity)?,
            )
            .ok_or(EditorError::Capacity)?
    } else if exponent.unsigned_abs() >= u128::BITS {
        0
    } else {
        let denominator = 1u128 << exponent.unsigned_abs();
        numerator / denominator + u128::from(numerator % denominator >= denominator / 2)
    };
    let mut text = rounded.to_string();
    if decimals > 0 {
        let decimals = decimals as usize;
        if text.len() <= decimals {
            text = format!("{}{}", "0".repeat(decimals + 1 - text.len()), text);
        }
        text.insert(text.len() - decimals, '.');
        text = text.trim_end_matches('0').trim_end_matches('.').into();
    }
    if value < 0.0 {
        text.insert(0, '-');
    }
    Ok(text)
}

pub(crate) fn color(red: u8, green: u8, blue: u8, alpha: f64) -> Result<String, EditorError> {
    if !alpha.is_finite() {
        return Err(EditorError::Refused);
    }
    if red == 0 && green == 0 && blue == 0 && alpha == 0.0 {
        return Ok("transparent".into());
    }
    if alpha == 1.0 {
        if [red, green, blue]
            .iter()
            .all(|component| component % RGB_SHORT_DIVISOR == 0)
        {
            return Ok(format!(
                "#{:x}{:x}{:x}",
                red / RGB_SHORT_DIVISOR,
                green / RGB_SHORT_DIVISOR,
                blue / RGB_SHORT_DIVISOR
            ));
        }
        return Ok(format!("#{red:02x}{green:02x}{blue:02x}"));
    }
    Ok(format!(
        "rgba({red}, {green}, {blue}, {})",
        number(alpha, ALPHA_DECIMALS)?
    ))
}

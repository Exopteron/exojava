use super::{CharStream, LexErrorType, LexResult, LexingError, BaseToken};

#[derive(Clone, Copy, Debug)]
pub struct RawInputCharacter(pub char);
impl BaseToken for RawInputCharacter {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        Ok(Self(s.next()?))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct HexDigit(pub char);
impl BaseToken for HexDigit {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        match s.next()? {
            v if v.is_ascii_hexdigit() => Ok(Self(v)),
            v => Err(LexingError::new(
                LexErrorType::SyntaxError(format!("invalid hex character {}", v)),
                (s.position, s.position),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnicodeMarker;
impl BaseToken for UnicodeMarker {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        s.match_str("u")?;
        while s.match_str("u").is_ok() {}
        Ok(Self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnicodeEscape(pub char);
impl BaseToken for UnicodeEscape {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        s.match_str("\\")?;
        UnicodeMarker::parse(s)?;
        let digits = [
            HexDigit::parse(s)?.0,
            HexDigit::parse(s)?.0,
            HexDigit::parse(s)?.0,
            HexDigit::parse(s)?.0,
        ];
        let str = digits.iter().collect::<String>();
        let code = u16::from_str_radix(&str, 16).map_err(|v| {
            LexingError::new(LexErrorType::ParseIntError(v), (s.position, s.position))
        })?;
        let v = char::from_u32(code as u32).ok_or_else(|| LexingError::new(
            LexErrorType::InvalidCodePoint(code as u32),
            (s.position, s.position),
        ))?;
        Ok(Self(v))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnicodeInputCharacter(pub char);

impl BaseToken for UnicodeInputCharacter {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        if s.match_str("\\").is_ok() {
            if s.match_str("\\").is_ok() {
                Ok(Self('\\'))
            } else {
                s.position -= 1;
                Ok(Self(UnicodeEscape::parse(s)?.0))
            }
        } else {
            Ok(Self(s.next()?))
        }
    }
}
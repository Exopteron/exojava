use exo_parser::{Parseable, parse_err, error::ParsingErrorType, Lexer, LexerRef};

pub mod class;
pub mod field;
pub mod method;

/// Characters banned in identifiers.
pub const BANNED_IDENT_CHARS: [char; 4] = ['.', ';', '[', '/'];


/// Unqualified name.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnqualifiedName(pub String);

impl UnqualifiedName {
    pub fn new(s: String) -> Option<Self> {
        let lexer = Lexer::new();
        Lexer::stream(lexer, s).token().ok().map(|v| v.token)
    }
}

impl Parseable for UnqualifiedName {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        let mut str = String::new();
        while let Ok(c) = s.char() {
            if c.is_whitespace() { break }
            if BANNED_IDENT_CHARS.contains(&c) { break }
            str.push(c);
        }
        if str.is_empty() {
            return Err(parse_err!(s, "empty unqualified name"));
        }
        Ok(Self(str))
    }
}
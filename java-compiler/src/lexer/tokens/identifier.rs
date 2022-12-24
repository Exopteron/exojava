use std::{rc::Rc};

use crate::lexer::{LexingError, LexErrorType};

use super::{Tokenizable, early::FinalTerminalElement, keywords::Keyword};

#[derive(Clone, Debug)]
pub struct Identifier(pub Rc<String>);

impl Tokenizable for Identifier {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let mut str = String::new();
        let start = s.position;
        str.push(s.get::<JavaLetter>()?.v.0);
        while let Ok(v) = s.get::<JavaLetterOrDigit>() {
            str.push(v.v.0);
        }
        if Keyword::is_keyword(&str) {
            return Err(LexingError::new(LexErrorType::SyntaxError("Keyword found in identifier".to_string()), (start, s.position)));
        }
        Ok(Self(Rc::new(str)))
    }
}




#[derive(Clone, Copy, Debug)]
pub struct JavaLetter(pub char);

impl JavaLetter {
    pub fn is_java_letter(c: char) -> bool {
        c.is_alphabetic()
    }
}

impl Tokenizable for JavaLetter {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if let FinalTerminalElement::InputCharacter(v) = s.lookahead()? {
            if Self::is_java_letter(v.0) {
                s.next()?;
                Ok(Self(v.0))
            } else {
                Err(LexingError::new(LexErrorType::SyntaxError(format!("Bad character in JavaLetter {}", v.0)), (s.position, s.position)))
            }
        } else {
            Err(LexingError::new(LexErrorType::SyntaxError("Line termintor in JavaLetter".to_string()), (s.position, s.position)))
        }
    }
}



#[derive(Clone, Copy, Debug)]
pub struct JavaLetterOrDigit(pub char);

impl JavaLetterOrDigit {
    pub fn is_java_letter_or_digit(c: char) -> bool {
        JavaLetter::is_java_letter(c) || c.is_numeric()
    }
}

impl Tokenizable for JavaLetterOrDigit {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if let FinalTerminalElement::InputCharacter(v) = s.lookahead()? {
            if Self::is_java_letter_or_digit(v.0) {
                s.next()?;
                Ok(Self(v.0))
            } else {
                Err(LexingError::new(LexErrorType::SyntaxError(format!("Bad character in JavaLetterOrDigit {}", v.0)), (s.position, s.position)))
            }
        } else {
            Err(LexingError::new(LexErrorType::SyntaxError("Line termintor in JavaLetterOrDigit".to_string()), (s.position, s.position)))
        }
    }
}

use crate::lexer::tokens::{Token, JToken, Input, operators::Operator, separators::Separator, literal::{number::integer::IntegerLiteral, Literal, StringLiteral, CharacterLiteral}, identifier::Identifier, keywords::Keyword};

use super::{error::{ParseResult, ParseError, ParseErrorType}, SyntaxElement, CompilerState};


pub struct JavaTokenStream {
    chars: Vec<Token<JToken>>,
    pub position: usize,
}

impl JavaTokenStream {
    pub fn new(mut s: Input) -> ParseResult<Self> {
        Ok(Self {
            chars: s.elements,
            position: 0,
        })
    }
    pub fn is_finished(&self) -> bool {
        self.position >= self.chars.len()
    }

    pub fn get<T: SyntaxElement>(&mut self, compiler: &mut CompilerState) -> ParseResult<T> {
        T::parse(compiler, self)
    }

    pub fn match_operator(&mut self, op: Operator) -> ParseResult<()> {
        if let JToken::Operator(Token { v, .. }) = self.next()?.v {
            if v == op {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError(format!("could not match operator {:?}", op)), (self.position, self.position)))
    }

    pub fn match_separator(&mut self, op: Separator) -> ParseResult<()> {
        if let JToken::Separator(Token { v, .. }) = self.next()?.v {
            if v == op {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError(format!("could not match operator {:?}", op)), (self.position, self.position)))
    }

    pub fn match_integer_literal(&mut self) -> ParseResult<IntegerLiteral> {
        if let JToken::Literal(Token { v: Literal::IntegerLiteral(v), .. }) = self.next()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match integer literal".to_string()), (self.position, self.position)))
    }

    pub fn match_boolean_literal(&mut self) -> ParseResult<bool> {
        if let JToken::Literal(Token { v: Literal::BooleanLiteral(v), .. }) = self.next()?.v {
            return Ok(v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match boolean literal".to_string()), (self.position, self.position)))
    }

    pub fn match_string_literal(&mut self) -> ParseResult<StringLiteral> {
        if let JToken::Literal(Token { v: Literal::StringLiteral(v), .. }) = self.next()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match string literal".to_string()), (self.position, self.position)))
    }

    pub fn match_char_literal(&mut self) -> ParseResult<CharacterLiteral> {
        if let JToken::Literal(Token { v: Literal::CharacterLiteral(v), .. }) = self.next()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match char literal".to_string()), (self.position, self.position)))
    }

    pub fn match_identifier(&mut self) -> ParseResult<Identifier> {
        if let JToken::Identifier(Token { v, .. }) = self.next()?.v {
            return Ok(v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match identifier".to_string()), (self.position, self.position)))
    }

    pub fn match_keyword(&mut self, kw: Keyword) -> ParseResult<()> {
        if let JToken::Keyword(Token { v, .. }) = self.next()?.v {
            if v == kw {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match keyword".to_string()), (self.position, self.position)))
    }





    pub fn lookahead_match_operator(&mut self, op: Operator) -> ParseResult<()> {
        if let JToken::Operator(Token { v, .. }) = self.lookahead()?.v {
            if v == op {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError(format!("could not match operator {:?}", op)), (self.position, self.position)))
    }

    pub fn lookahead_match_separator(&mut self, op: Separator) -> ParseResult<()> {
        if let JToken::Separator(Token { v, .. }) = self.lookahead()?.v {
            if v == op {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError(format!("could not match operator {:?}", op)), (self.position, self.position)))
    }

    pub fn lookahead_match_integer_literal(&mut self) -> ParseResult<IntegerLiteral> {
        if let JToken::Literal(Token { v: Literal::IntegerLiteral(v), .. }) = self.lookahead()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match integer literal".to_string()), (self.position, self.position)))
    }

    pub fn lookahead_match_boolean_literal(&mut self) -> ParseResult<bool> {
        if let JToken::Literal(Token { v: Literal::BooleanLiteral(v), .. }) = self.lookahead()?.v {
            return Ok(v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match boolean literal".to_string()), (self.position, self.position)))
    }

    pub fn lookahead_match_string_literal(&mut self) -> ParseResult<StringLiteral> {
        if let JToken::Literal(Token { v: Literal::StringLiteral(v), .. }) = self.lookahead()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match string literal".to_string()), (self.position, self.position)))
    }

    pub fn lookahead_match_char_literal(&mut self) -> ParseResult<CharacterLiteral> {
        if let JToken::Literal(Token { v: Literal::CharacterLiteral(v), .. }) = self.lookahead()?.v {
            return Ok(v.v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match char literal".to_string()), (self.position, self.position)))
    }

    pub fn lookahead_match_identifier(&mut self) -> ParseResult<Identifier> {
        if let JToken::Identifier(Token { v, .. }) = self.lookahead()?.v {
            return Ok(v);
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match identifier".to_string()), (self.position, self.position)))
    }

    pub fn lookahead_match_keyword(&mut self, kw: Keyword) -> ParseResult<()> {
        if let JToken::Keyword(Token { v, .. }) = self.lookahead()?.v {
            if v == kw {
                return Ok(());
            }
        }
        Err(ParseError::new(ParseErrorType::SyntaxError("could not match keyword".to_string()), (self.position, self.position)))
    }




    // pub fn lookahead(&mut self) -> LexResult<FinalTerminalElement> {
    //     let v = self.chars.get(self.position).copied();
    //     v.ok_or(LexingError::new(
    //         LexErrorType::EOI,
    //         (self.position, self.position),
    //     ))
    // }

    pub fn next(&mut self) -> ParseResult<Token<JToken>> {
        let v = self.chars.get(self.position).cloned();
        self.position += 1;

        v.ok_or(ParseError::new(
            ParseErrorType::EOI,
            (self.position, self.position),
        ))
    }

    pub fn lookahead(&mut self) -> ParseResult<Token<JToken>> {
        let v = self.chars.get(self.position).cloned();

        v.ok_or(ParseError::new(
            ParseErrorType::EOI,
            (self.position, self.position),
        ))
    }

    // pub fn match_input_character(&mut self) -> LexResult<InputCharacter> {
    //     let v = self.chars.get(self.position).copied();
    //     self.position += 1;
    //     let c = v.ok_or(LexingError::new(
    //         LexErrorType::EOI,
    //         (self.position, self.position),
    //     ))?;
    //     match c {
    //         FinalTerminalElement::InputCharacter(v) => Ok(v),
    //         FinalTerminalElement::LineTerminator(_) => Err(LexingError::new(
    //             LexErrorType::SyntaxError(
    //                 "Got line terminator when looking for input character".to_string(),
    //             ),
    //             (self.position, self.position),
    //         )),
    //     }
    // }

    // pub fn match_line_terminator(&mut self) -> LexResult<LineTerminator> {
    //     let v = self.chars.get(self.position).copied();
    //     self.position += 1;
    //     let c = v.ok_or(LexingError::new(
    //         LexErrorType::EOI,
    //         (self.position, self.position),
    //     ))?;
    //     match c {
    //         FinalTerminalElement::LineTerminator(v) => Ok(v),
    //         FinalTerminalElement::InputCharacter(_) => Err(LexingError::new(
    //             LexErrorType::SyntaxError(
    //                 "Got input character when looking for line terminator".to_string(),
    //             ),
    //             (self.position, self.position),
    //         )),
    //     }
    // }

    // pub fn match_one_of(&mut self, strs: &[&str], eat: bool) -> LexResult<usize> {
    //     for (idx, s) in strs.iter().enumerate() {
    //         if self.match_str(s, eat).is_ok() {
    //             return Ok(idx);
    //         }
    //     }
    //     return Err(LexingError::new(LexErrorType::SyntaxError(format!("Could not match one of {:?}", strs)), (self.position, self.position)))
    // }

    // pub fn match_str(&mut self, str: &str, eat: bool) -> LexResult<()> {
    //     let mut cursor = 0;
    //     for c in str.chars() {
    //         let v = self.chars[self.position + cursor];
    //         match v {
    //             FinalTerminalElement::LineTerminator(_) => {
    //                 return Err(LexingError::new(
    //                     LexErrorType::WrongChar('\n', c),
    //                     (cursor, cursor),
    //                 ));
    //             }
    //             FinalTerminalElement::InputCharacter(InputCharacter(v)) => {
    //                 if v != c {
    //                     return Err(LexingError::new(
    //                         LexErrorType::WrongChar(v, c),
    //                         (cursor, cursor),
    //                     ));
    //                 }
    //             }
    //         }
    //         cursor += 1;
    //     }
    //     if eat {
    //         for _ in 0..cursor {
    //             self.next()?;
    //         }
    //     }
    //     Ok(())
    // }
}
// macro_rules! match_one_of {
//     ($name:ident, {$(
//         $f_name:ident($token_ty:ty) => $block:block
//     )*}) => {
//         #[derive(Debug, Clone)]
//         pub enum $name {
//             $(
//                 $f_name($token_ty)
//             )*
//         }

//         impl 
//     };
// }
// match_one_of!(Balls, {
//     A(a) => {}
// });
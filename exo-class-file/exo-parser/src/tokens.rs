use super::error::Result;
use crate::{error::{ParsingError, ParsingErrorType}, parse_err};
use std::{cell::{RefCell, Ref}, fmt::Debug, rc::Rc, ops::{Range, Deref}, any::type_name};

/// Generic `Tokenable` trait.
/// Implemented by all tokens.
pub trait Parseable: Sized + Debug {
    fn parse(s: &mut LexerStream) -> Result<Self>;
    fn name() -> &'static str {
        type_name::<Self>()
    }
}

/// Represents a token.
///
/// Stores the start/end indices in
/// the parse stream.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Token<T: Parseable> {
    pub token: T,
    pub start: usize,
    pub end: usize,
}

impl<T: Parseable> Deref for Token<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

impl<T: Parseable> Token<T> {
    pub fn new(token: T, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }
}

/// A `Lexer`. Parses characters
/// into tokens.
pub struct Lexer {
    successful_parsed_tokens: Vec<usize>,
    currently_parsing: Vec<String>,
}

pub type LexerRef = Rc<RefCell<Lexer>>;
impl Lexer {
    pub fn currently_parsing_list(&self) -> Vec<String> {
        self.currently_parsing.clone()
    }

    pub fn currently_parsing(&self) -> Option<&str> {
        self.currently_parsing.last().map(|v| v.as_str())
    }



    pub fn new() -> LexerRef {
        let s = Self {
            successful_parsed_tokens: Vec::new(),
            currently_parsing: Vec::new(),
        };
        Rc::new(RefCell::new(s))
    }


    pub fn stream(this: LexerRef, string: String) -> LexerStream {
        LexerStream::new(string.chars().collect(), 0, this)
    }


    // /// Grab the next character from
    // /// the lexer. Returns `None` if
    // /// there are no more characters.
    // pub fn char(&mut self) -> Result<char> {
    //     if self.ended() {
    //         Err(ParsingErrorType::EOSError.to(self))
    //     } else {
    //         let c = self.chars[self.position];
    //         self.position += 1;
    //         Ok(c)
    //     }
    // }

    // /// Attempts to parse a parseable item. On failure,
    // /// returns the error and how many tokens were successfully
    // /// parsed.
    // pub fn token<T: Parseable>(&mut self) -> std::result::Result<Token<T>, (ParsingError, usize)> {
    //     self.currently_parsing.push(T::name());
    //     let start = self.position;
    //     self.successful_parsed_tokens.push(0);
    //     match T::parse(self) {
    //         Ok(v) => {
    //             self.currently_parsing.pop();
    //             self.successful_parsed_tokens.pop();
    //             if let Some(v) = self.successful_parsed_tokens.last_mut() {
    //                 *v += 1;
    //             }
    //             let end = self.position;
    //             Ok(Token::new(v, start, end))
    //         }
    //         Err(e) => {
    //             self.currently_parsing.pop();
    //             self.position = start;
    //             Err((e, self.successful_parsed_tokens.pop().unwrap()))
    //         }
    //     }
    // }
}

pub struct LexerStream {
    pub chars: Vec<char>,
    pub position: usize,
    lexer: LexerRef,
}

impl LexerStream {
    fn new(chars: Vec<char>, position: usize, lexer: LexerRef) -> Self {
        Self { chars, position, lexer }
    }

    pub fn split(&mut self, range: Range<usize>) -> Result<LexerStream> {
        if self.position > range.start {
            parse_err!(self, "range outside of eaten characters");
        }

        let chars = self.chars.drain(range).collect();
        Ok(Self::new(chars, 0, self.lexer.clone()))        
    }


    pub fn lexer(&self) -> Ref<Lexer> {
        self.lexer.borrow()
    }

    pub fn chars(&self) -> &[char] {
        &self.chars[self.position..]
    }

    pub fn ended(&self) -> bool {
        self.position >= self.chars.len()
    }
    /// Grab the next character from
    /// the lexer. Returns `None` if
    /// there are no more characters.
    pub fn char(&mut self) -> Result<char> {
        if self.ended() {
            Err(ParsingErrorType::EOSError.to(self))
        } else {
            let c = self.chars[self.position];
            self.position += 1;
            Ok(c)
        }
    }

    /// Attempts to parse a parseable item. On failure,
    /// returns the error and how many tokens were successfully
    /// parsed.
    pub fn token<T: Parseable>(&mut self) -> std::result::Result<Token<T>, (ParsingError, usize)> {
        self.lexer.borrow_mut().currently_parsing.push(T::name().to_string());
        let start = self.position;
        self.lexer.borrow_mut().successful_parsed_tokens.push(0);
        match T::parse(self) {
            Ok(v) => {
                self.lexer.borrow_mut().currently_parsing.pop();
                self.lexer.borrow_mut().successful_parsed_tokens.pop();
                if let Some(v) = self.lexer.borrow_mut().successful_parsed_tokens.last_mut() {
                    *v += 1;
                }
                let end = self.position;
                Ok(Token::new(v, start, end))
            }
            Err(e) => {
                self.lexer.borrow_mut().currently_parsing.pop();
                self.position = start;
                Err((e, self.lexer.borrow_mut().successful_parsed_tokens.pop().unwrap()))
            }
        }
    }
}

pub fn enclosed<A: Parseable, B: Parseable>(stream: &mut LexerStream) -> Result<LexerStream> {

    let start_pos = stream.position;
    stream.token::<A>()?;
    // println!("Chazzrs: {:?}", stream.chars());

    let mut amount = 1;
    let mut stack = 1;



    loop {
        let mut success = false;
        match stream.token::<A>() {
            Ok(_) => {
                success = true;
                // println!("pushing");
                stack += 1;
            },
            Err(e) => {
                if matches!(e.0.error_type, ParsingErrorType::EOSError) {
                    return Err(e.0);
                }
                // println!("A error: {:?}", e);
            },
        }
        if !success {
            match stream.token::<B>() {
                Ok(_) => { 
                    success = true;
                    // println!("Popping");
                    stack -= 1;
                 },
                Err(e) => {
                    if matches!(e.0.error_type, ParsingErrorType::EOSError) {
                        return Err(e.0);
                    }
                    // println!("B error: {:?}", e);
                },
            }
        }
        if !success {
            stream.position += 1;
        }
        amount += 1;
        if stack == 0 {
            stream.position = start_pos + amount;
            //println!("Cha3rs: {:?}", &stream.chars()[..amount]);
            let mut v = stream.chars[start_pos..start_pos + (amount)].to_vec();
            v.remove(0);
            v.pop();
            // println!("VCHAR Z {:?}", v);
            //stream.chars.remove(0);
            return Ok(LexerStream::new(v, 0, stream.lexer.clone()));
            
        }

    }


}

#[cfg(test)]
mod tests {
    use crate::{Lexer, enclosed, tokenimpl::Char};

    #[test]
    fn epic() {
        let lexer = Lexer::new();
        let mut stream = Lexer::stream(lexer, "{{{Balls}}} 1234".to_string());

        let mut delim_1 = enclosed::<Char<'{'>, Char<'}'>>(&mut stream).unwrap();

        let mut delim_2 = enclosed::<Char<'{'>, Char<'}'>>(&mut delim_1).unwrap();

        let delim_3 = enclosed::<Char<'{'>, Char<'}'>>(&mut delim_2).unwrap();

        println!("Chars: {:?}", delim_3.chars());
        println!("Stream: {:?}", stream.chars());
    }
}


impl From<(ParsingError, usize)> for ParsingError {
    fn from(v: (ParsingError, usize)) -> Self {
        v.0
    }
}

pub struct TokenStream {}

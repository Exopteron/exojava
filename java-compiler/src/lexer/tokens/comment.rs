use std::rc::Rc;

use crate::lexer::{LexingError, LexErrorType};

use super::{Tokenizable, early::lines::{LineTerminator, InputCharacter}, Token};

#[derive(Clone, Debug)]
pub enum Comment {
    TraditionalComment(Token<TraditionalComment>),
    EndOfLineComment(Token<EndOfLineComment>)
}

impl Tokenizable for Comment {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("//", false).is_ok() {
            Ok(Self::EndOfLineComment(s.get::<EndOfLineComment>()?))
        } else {
            Ok(Self::TraditionalComment(s.get()?))
        }
    }
}


#[derive(Clone, Debug)]
pub struct TraditionalComment(pub Token<CommentTail>);
impl Tokenizable for TraditionalComment {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_str("/*", true)?;
        Ok(Self(s.get()?))
    }
}


#[derive(Clone, Debug)]
pub enum CommentTail {
    StarCommentTailStar(Rc<Token<CommentTailStar>>),
    NotStarCommentTail(Token<NotStar>, Rc<Token<CommentTail>>)
}
impl Tokenizable for CommentTail {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("*", true).is_ok() {
            Ok(Self::StarCommentTailStar(Rc::new(s.get()?)))
        } else {
            Ok(Self::NotStarCommentTail(s.get()?, Rc::new(s.get()?)))
        }
    }
}

#[derive(Clone, Debug)]
pub enum CommentTailStar {
    Slash,
    StarCommentTailStar(Rc<Token<CommentTailStar>>),
    NotStarNotSlashCommentTail(Token<NotStarNotSlash>, Token<CommentTail>)
}
impl Tokenizable for CommentTailStar {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("/", true).is_ok() {
            Ok(Self::Slash)
        } else if s.match_str("*", true).is_ok() {
            Ok(Self::StarCommentTailStar(Rc::new(s.get()?)))
        } else {
            Ok(Self::NotStarNotSlashCommentTail(s.get::<NotStarNotSlash>()?, s.get()?))
        }
    }
}



#[derive(Clone, Copy, Debug)]
pub enum NotStar {
    InputCharacter(InputCharacter),
    LineTerminator(LineTerminator)
}

impl Tokenizable for NotStar {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if let Ok(v) = s.match_line_terminator() {
            Ok(Self::LineTerminator(v))
        } else {
            let c = s.match_input_character()?;
            if c.0 == '*' {
                return Err(LexingError::new(LexErrorType::SyntaxError("Got * inside NotStar".to_string()), (s.position, s.position)));
            }
            Ok(Self::InputCharacter(c))
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub enum NotStarNotSlash {
    InputCharacter(InputCharacter),
    LineTerminator(LineTerminator)
}

impl Tokenizable for NotStarNotSlash {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if let Ok(v) = s.match_line_terminator() {
            Ok(Self::LineTerminator(v))
        } else {
            let c = s.match_input_character()?;
            if c.0 == '*' || c.0 == '/' {
                return Err(LexingError::new(LexErrorType::SyntaxError("Got * or / inside NotStarOrSlash".to_string()), (s.position, s.position)));
            }
            Ok(Self::InputCharacter(c))
        }
    }
}


#[derive(Clone, Debug)]
pub struct EndOfLineComment(pub Rc<String>);

impl Tokenizable for EndOfLineComment {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_str("//", true)?;
        let mut comment = String::new();
        while let Ok(v) = s.match_input_character() {
            comment.push(v.0);
        }
        Ok(Self(Rc::new(comment)))
    }
}
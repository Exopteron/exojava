// use thiserror::Error;

// use super::super::testlang::compile::{Inst, VarType};

// use super::compile::Compiler;

// #[derive(Error, Debug)]
// pub enum ParseError {
//     #[error("wrong character, got {0} but expected {1}")]
//     WrongChar(char, char),
//     #[error("wrong token, got {0:?} but expected {1:?}")]
//     WrongToken(TokenTy, TokenTy),
//     #[error("syntax error: {0}")]
//     SyntaxError(String),
//     #[error("EOI reached")]
//     EOI
// }

// type Result<T> = std::result::Result<T, ParseError>;

// pub struct CharStream {
//     chars: Vec<char>,
// }

// impl CharStream {
//     pub fn new(s: String) -> Self {
//         Self {
//             chars: s.chars().rev().collect(),
//         }
//     }
//     pub fn is_finished(&self) -> bool {
//         self.chars.is_empty()
//     }

//     pub fn lookahead(&mut self) -> Option<char> {
//         let v = self.chars.last().copied();
//         if let Some(b) = v.map(|v| v.is_whitespace()) {
//             if b {
//                 self.chars.pop();
//                 return self.lookahead();
//             }
//         }
//         v
//     }

//     pub fn next(&mut self) -> Option<char> {
//         println!("l {}", self.chars.len());
//         let v = self.chars.pop();
//         if let Some(b) = v.map(|v| v.is_whitespace()) {
//             if b {
//                 return self.next();
//             }
//         }
//         v
//     }

//     pub fn match_str(&mut self, str: &str) -> Result<()> {
//         let mut cursor = 0;
//         for c in str.chars() {
//             let v = self.chars[self.chars.len() - (cursor + 1)];
//             if v != c {
//                 return Err(ParseError::WrongChar(v, c));
//             }
//             cursor += 1;
//         } 
//         for _ in 0..cursor {
//             self.next();
//         }
//         Ok(())
//     }
// }

// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
// pub enum TokenTy {
//     Number,
//     Add,
//     Subtract,
//     Multiply,
//     Divide,
//     LParenthesis,
//     RParenthesis,
//     LCurlyBracket,
//     RCurlyBracket,
//     LSquareBracket,
//     RSquareBracket,
//     EqualsAssign,
//     EqualsCmp,
//     Comma,
//     Ident,
//     StringLit,
//     VarKw,
//     ExternKw,
//     ReturnKw,
//     NilKw,
// }

// pub enum Attribute {
//     Number(f64),
//     String(String)
// }

// impl Attribute {
//     pub fn as_number(&self) -> f64 {
//         match self {
//             Attribute::Number(v) => *v,
//             Self::String(_) => panic!()
//         }
//     }
//     pub fn as_string(&self) -> &str {
//         match self {
//             Attribute::Number(_v) => panic!(),
//             Self::String(v) => v
//         }
//     }
// }

// type Token = (TokenTy, Vec<Attribute>);

// pub struct TokenStream {
//     pub tokens: Vec<Token>,
//     cursor: usize,
// }
// impl TokenStream {
//     pub fn new(_compiler: &mut Compiler, mut s: CharStream) -> Self {
//         let mut tokens = Vec::new();

//         let mut not_nil = false;
//         let mut not_var = false;
//         let mut not_extern = false;
//         let mut not_return = false;

//         while !s.is_finished() {


//             match s.next().unwrap() {
//                 v if v.is_whitespace() => continue,

//                 '=' => {
//                     if s.lookahead().is_some() && s.lookahead().unwrap() == '=' {
//                         s.next();
//                         tokens.push((TokenTy::EqualsCmp, vec![]))
//                     } else {
//                         tokens.push((TokenTy::EqualsAssign, vec![]))
//                     }
//                 }
//                 '+' => tokens.push((TokenTy::Add, vec![])),
//                 '-' => tokens.push((TokenTy::Subtract, vec![])),
//                 '*' => tokens.push((TokenTy::Multiply, vec![])),
//                 '/' => tokens.push((TokenTy::Divide, vec![])),
//                 '(' => tokens.push((TokenTy::LParenthesis, vec![])),
//                 ')' => tokens.push((TokenTy::RParenthesis, vec![])),
//                 '[' => tokens.push((TokenTy::LSquareBracket, vec![])),
//                 ']' => tokens.push((TokenTy::RSquareBracket, vec![])),
//                 '{' => tokens.push((TokenTy::LCurlyBracket, vec![])),
//                 '}' => tokens.push((TokenTy::RCurlyBracket, vec![])),
//                 ',' => tokens.push((TokenTy::Comma, vec![])),
//                 'e' if !not_extern => {
//                     if s.match_str("xtern").is_ok() {
//                         tokens.push((TokenTy::ExternKw, vec![]));
//                     } else {
//                         not_extern = true;
//                         s.chars.push('e');
//                         continue;
//                     }
//                 }
//                 'n' if !not_nil => {
//                     if s.match_str("il").is_ok() {
//                         tokens.push((TokenTy::NilKw, vec![]));
//                     } else {
//                         not_nil = true;
//                         s.chars.push('n');
//                         continue;
//                     }
//                 }
//                 'v' if !not_var => {
//                     if s.match_str("ar").is_ok() {
//                         tokens.push((TokenTy::VarKw, vec![]));
//                     } else {
//                         not_var = true;
//                         s.chars.push('v');
//                         continue;
//                     }
//                 }
//                 'r' if !not_return => {
//                     if s.match_str("eturn").is_ok() {
//                         tokens.push((TokenTy::ReturnKw, vec![]));
//                     } else {
//                         not_return = true;
//                         s.chars.push('r');
//                         continue;
//                     }
//                 }


//                 v if v.is_numeric() || (v == '-' && s.lookahead().unwrap().is_numeric()) => {
//                     let mut number = vec![v];
//                     let mut did_dot = false;
//                     while s.lookahead().is_some() && (s.lookahead().unwrap().is_numeric()
//                         || (s.lookahead().unwrap() == '.' && !did_dot))
//                     {
//                         if s.lookahead().unwrap() == '.' {
//                             did_dot = true;
//                         }
//                         number.push(s.next().unwrap());
//                     }
//                     tokens.push((
//                         TokenTy::Number,
//                         vec![Attribute::Number(
//                             number.iter().collect::<String>().parse::<f64>().unwrap(),
//                         )],
//                     ));
//                 }
//                 v if v.is_alphabetic() => {
//                     let mut ident = vec![v];
//                     while s.lookahead().is_some() && (s.lookahead().unwrap().is_alphanumeric()) {
//                         ident.push(s.next().unwrap());
//                     }
//                     tokens.push((
//                         TokenTy::Ident,
//                         vec![Attribute::String(
//                             ident.iter().collect::<String>(),
//                         )],
//                     ));
//                 }
//                 v => panic!("Unknown char {}", v),
//             }
//             not_extern = false;
//             not_nil = false;
//             not_return = false;
//             not_var = false;
//         }
//         Self { tokens, cursor: 0 }
//     }

//     pub fn lookahead(&self, plus: usize) -> Option<&Token> {
//         self.tokens.get(self.cursor + plus)
//     }

//     pub fn match_lookahead(&mut self, plus: usize, t: TokenTy) -> bool {
//         let v = self.lookahead(plus).unwrap();
//         if v.0 != t {
//             return false;
//         }
//         true
//     }

//     pub fn match_token(&mut self, t: TokenTy) -> Result<Vec<Attribute>> {
//         let v = self.lookahead(0).unwrap();
//         if v.0 != t {
//             return Err(ParseError::WrongToken(v.0, t));
//         }
//         self.cursor += 1;
//         let v = std::mem::take(&mut self.tokens[self.cursor - 1].1);
//         Ok(v)
//     }
// }

// pub trait NonTerminal<ReturnV> {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<ReturnV>;
// }




// pub struct Factor;

// impl NonTerminal<()> for Factor {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<()> {
//         println!("Facktor {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         if stream.match_lookahead(0, TokenTy::Number) {
//             let la = stream.lookahead(0).unwrap().1.get(0).unwrap().as_number();
//             compiler.get_current_fn().insts.push(Inst::Push(la));
//             stream.match_token(TokenTy::Number)?;
//         } else if stream.match_lookahead(0, TokenTy::LParenthesis) {
//             stream.match_token(TokenTy::LParenthesis)?;
//             Expr::visit(compiler, stream)?;
//             stream.match_token(TokenTy::RParenthesis)?;
//         } else if stream.match_lookahead(0, TokenTy::Ident) {
//             println!("Found ident");
//             let ident = stream.match_token(TokenTy::Ident)?;
//             let ident = ident[0].as_string();

//             let addr = compiler.get_current_fn().var_alloc.find_var(ident).ok_or_else(|| ParseError::SyntaxError(format!("could not find var {}", ident)));
//             println!("Addr: {:?}", addr.is_ok());
//             let addr = addr?;
//             let inst = match addr {
//                 VarType::Local(v) => {
//                     Inst::LoadVar(v)
//                 },
//                 VarType::Global(v) => {
//                     Inst::LoadGlobal(v)
//                 }
//             };
//             println!("Found loc");
//             compiler.get_current_fn().insts.push(inst);
//         } else if stream.match_lookahead(0, TokenTy::LSquareBracket) {
//             stream.match_token(TokenTy::LSquareBracket)?;
//             let mut found_comma = false;
//             let mut num_entries = 0;
//             loop {
//                 if stream.lookahead(0).is_none() {
//                     return Err(ParseError::SyntaxError("".to_string()))
//                 }
//                 else if stream.match_lookahead(0, TokenTy::RSquareBracket) && !found_comma {
//                     stream.match_token(TokenTy::RSquareBracket)?;
//                     break;
//                 } else {
//                     found_comma = false;
//                     Expr::visit(compiler, stream)?;
//                     num_entries += 1;
//                     if stream.match_lookahead(0, TokenTy::Comma) {
//                         stream.match_token(TokenTy::Comma)?;
//                         found_comma = true;
//                     } else {
//                         stream.match_token(TokenTy::RSquareBracket)?;
//                         break;
//                     }
//                 }
//             }
//             compiler.get_current_fn().insts.push(Inst::MkArray(num_entries));


//         } else if stream.match_lookahead(0, TokenTy::NilKw) {
//             stream.match_token(TokenTy::NilKw)?;
//             compiler.get_current_fn().insts.push(Inst::PushNil);
//         } 
//         else {
//             return Err(ParseError::SyntaxError("".to_string()));
//         }
//         Ok(())
//     }
// }

// pub struct Term;

// impl NonTerminal<()> for Term {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<()> {
//         println!("Tern {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         Factor::visit(compiler, stream)?;
//         loop {
//             if stream.lookahead(0).is_none() {
//                 return Ok(());
//             } else if stream.match_lookahead(0, TokenTy::Multiply) {
//                 stream.match_token(TokenTy::Multiply)?;
//                 Factor::visit(compiler, stream)?;
//                 compiler.get_current_fn().insts.push(Inst::Mul);
//             } else if stream.match_lookahead(0, TokenTy::Divide) {
//                 stream.match_token(TokenTy::Divide)?;
//                 Factor::visit(compiler, stream)?;
//                 compiler.get_current_fn().insts.push(Inst::Divide);
//             } else { return Ok(()) };
//         }
//     }
// }

// pub struct Expr;

// impl NonTerminal<()> for Expr {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<()> {
//         println!("Expr {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         Term::visit(compiler, stream)?;
//         loop {
//             if stream.lookahead(0).is_none() {
//                 return Ok(());
//             } else if stream.match_lookahead(0, TokenTy::Add) {
//                 stream.match_token(TokenTy::Add)?;
//                 Term::visit(compiler, stream)?;
//                 compiler.get_current_fn().insts.push(Inst::Add);
//             } else if stream.match_lookahead(0, TokenTy::Subtract) {
//                 stream.match_token(TokenTy::Subtract)?;
//                 Term::visit(compiler, stream)?;
//                 compiler.get_current_fn().insts.push(Inst::Sub);
//             } else if stream.match_lookahead(0, TokenTy::LParenthesis) {
//                 println!("Found {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//                 stream.match_token(TokenTy::LParenthesis)?;
//                 let mut found_comma = false;
//                 let mut num_entries = 0;
//                 loop {
//                     if stream.lookahead(0).is_none() {
//                         return Err(ParseError::SyntaxError("".to_string()))
//                     }
//                     else if stream.match_lookahead(0, TokenTy::RParenthesis) && !found_comma {
//                         stream.match_token(TokenTy::RParenthesis)?;
//                         break;
//                     } else {
//                         found_comma = false;
//                         println!("Matching expr {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//                         Expr::visit(compiler, stream)?;
//                         num_entries += 1;
//                         if stream.match_lookahead(0, TokenTy::Comma) {
//                             stream.match_token(TokenTy::Comma)?;
//                             found_comma = true;
//                         } else {
//                             stream.match_token(TokenTy::RParenthesis)?;
//                             break;
//                         }
//                     }
//                 }
//                 println!("Call done {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//                 compiler.get_current_fn().insts.push(Inst::Call(num_entries));
//             } else if stream.match_lookahead(0, TokenTy::LSquareBracket) {
//                 stream.match_token(TokenTy::LSquareBracket)?;
//                 Expr::visit(compiler, stream)?;
//                 stream.match_token(TokenTy::RSquareBracket)?;
//                 if stream.match_lookahead(0, TokenTy::EqualsAssign) {
//                     stream.match_token(TokenTy::EqualsAssign)?;
//                     Expr::visit(compiler, stream)?;

//                     compiler.get_current_fn().insts.push(Inst::SetIndex);
//                 } else {
//                     compiler.get_current_fn().insts.push(Inst::GetIndex);
//                 }
//             } 
//             else { return Ok(()) };
//         }
//     }
// }


// pub struct Block;

// impl NonTerminal<()> for Block {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<()> {
        
//         println!("Blockd: {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         stream.match_token(TokenTy::LCurlyBracket)?;
        
//         println!("Blockd: {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         compiler.get_current_fn().var_alloc.enter_new_scope();
//         while Stmt::visit(compiler, stream).is_ok() {}
//         compiler.get_current_fn().var_alloc.exit_scope();
//         stream.match_token(TokenTy::RCurlyBracket)?;
//         Ok(())
//     }
// }



// pub struct Stmt;

// impl NonTerminal<()> for Stmt {
//     fn visit(compiler: &mut Compiler, stream: &mut TokenStream) -> Result<()> {
//         println!("Stmt {:?}", stream.tokens.iter().skip(stream.cursor).map(|v| v.0).collect::<Vec<TokenTy>>());
//         if stream.lookahead(0).is_none() {
//             return Err(ParseError::EOI)
//         } else if stream.match_lookahead(0, TokenTy::LCurlyBracket) {
//             Block::visit(compiler, stream)?;
//         } else if stream.match_lookahead(0, TokenTy::VarKw) {
//             stream.match_token(TokenTy::VarKw)?;
//             let ident = stream.match_token(TokenTy::Ident)?;
//             let ident = ident[0].as_string();
//             stream.match_token(TokenTy::EqualsAssign)?;
//             let var = compiler.get_current_fn().var_alloc.declare_var(ident.to_string());
//             Expr::visit(compiler, stream)?;
//             let inst = match var {
//                 VarType::Local(v) => Inst::StoreVar(v),
//                 VarType::Global(_) => panic!("shouldnt")
//             };
//             compiler.get_current_fn().insts.push(inst);
//         } else if stream.match_lookahead(0, TokenTy::ExternKw) {
//             stream.match_token(TokenTy::ExternKw)?;
//             let ident = stream.match_token(TokenTy::Ident)?;
//             let ident = ident[0].as_string();
//             stream.match_token(TokenTy::EqualsAssign)?;

//             let la = stream.lookahead(0).unwrap().1.get(0).unwrap().as_number();

//             stream.match_token(TokenTy::Number)?;
//             compiler.get_current_fn().var_alloc.declare_general(ident.to_string(), VarType::Global(la as usize));
//         } else if stream.match_lookahead(0, TokenTy::ReturnKw) {
//             stream.match_token(TokenTy::ReturnKw)?;
//             Expr::visit(compiler, stream)?;
//             compiler.get_current_fn().insts.push(Inst::Return);
//         } else {
//             let mut done = false;
//             if stream.match_lookahead(0, TokenTy::Ident) {
//                 if stream.match_lookahead(1, TokenTy::EqualsAssign) {

//                     let ident = stream.match_token(TokenTy::Ident)?;
//                     let ident = ident[0].as_string();

//                     done = true;
//                     stream.match_token(TokenTy::EqualsAssign)?;
                

//                     let addr = compiler.get_current_fn().var_alloc.find_var(ident).ok_or_else(|| ParseError::SyntaxError(format!("could not find var {}", ident)));
//                     println!("2 Addr: {:?}", addr.is_ok());
//                     let addr = addr?;
//                     let inst = match addr {
//                         VarType::Local(v) => {
//                             Inst::StoreVar(v)
//                         },
//                         VarType::Global(_v) => {
//                             panic!("No global")
//                         }
//                     };
//                     println!("2 Found loc");
//                     Expr::visit(compiler, stream)?;
//                     compiler.get_current_fn().insts.push(inst);

//                 }
//             } 
//             if !done {

//                 println!("Ealse");
//                 Expr::visit(compiler, stream)?;
//                 compiler.get_current_fn().insts.push(Inst::Pop);
//             }
//         }
//         Ok(())
//     }
// }



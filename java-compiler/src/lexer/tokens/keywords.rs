use crate::lexer::tokens::Tokenizable;
use crate::lexer::{LexingError, LexErrorType};
macro_rules! def_keywords {
    (
        $(
            $name:ident($word:expr)
        ),*
    ) => {

        $(
            #[derive(Clone, Debug)]
            pub struct $name;
            impl Tokenizable for $name {
                fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
                    s.match_str($word, true)?;
                    Ok(Self)
                }
            }
        )*


        #[derive(Clone, Debug)]
        pub enum Keyword {
            $(
                $name($name)
            ),*
        }

        impl Keyword {
            pub fn is_keyword(s: &str) -> bool {
                match s {
                    $(
                        stringify!($name) => true,
                    )*
                    _ => false
                }
            }
        }

        impl Tokenizable for Keyword {
            fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
                if $(
                  $name::parse(s).is_ok() {
                    Ok(Self::$name($name))
                  } else if 
                )* true {
                    Err(LexingError::new(LexErrorType::SyntaxError("could not match keyword".to_string()), (s.position, s.position)))
                } else {
                    unreachable!()
                }
            }
        }
    };
}

def_keywords! {
    Abstract("abstract"),
    Assert("assert"),
    Boolean("boolean"),
    Break("break"),
    Byte("byte"),
    Case("case"),
    Catch("catch"),
    Char("char"),
    Class("class"),
    Const("const"),
    Continue("continue"),
    Default("default"),
    Double("double"),
    Do("do"),
    Else("else"),
    Enum("enum"),
    Extends("extends"),
    Finally("finally"),
    Final("final"),
    Float("float"),
    For("for"),
    If("if"),
    Goto("goto"),
    Implements("implements"),
    Import("import"),
    Instanceof("instanceof"),
    Int("int"),
    Interface("interface"),
    Long("long"),
    Native("native"),
    New("new"),
    Package("package"),
    Private("private"),
    Protected("protected"),
    Public("public"),
    Return("return"),
    Short("short"),
    Static("static"),
    Strictfp("strictfp"),
    Super("super"),
    Switch("switch"),
    Synchronized("synchronized"),
    This("this"),
    Throws("throws"),
    Throw("throw"),
    Transient("transient"),
    Try("try"),
    Void("void"),
    Volatile("volatile"),
    While("while")

}

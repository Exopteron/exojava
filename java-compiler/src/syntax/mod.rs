use fxhash::FxHashMap;

use self::{stream::JavaTokenStream, error::ParseResult};

pub mod stream;
pub mod error;
pub mod types;
pub mod values;

pub enum SymbolType {
    String(String)
}

pub type SymbolRef = usize;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SymbolStringRef(pub SymbolRef);

pub struct SymbolTable {
    symbols: Vec<SymbolType>,
    string_lookup: FxHashMap<String, usize>
}

impl SymbolTable {
    fn insert_item(&mut self, i: SymbolType) -> usize {
        let idx = self.symbols.len();
        self.symbols.push(i);
        idx
    }

    pub fn insert_string(&mut self, s: String) -> SymbolStringRef {
        if let Some(v) = self.string_lookup.get(&s) {
            SymbolStringRef(*v)
        } else {
            SymbolStringRef(self.insert_item(SymbolType::String(s)))
        }
    }
}




pub struct CompilerState {
    pub symbol_table: SymbolTable
}

pub trait SyntaxElement: Sized {

    fn parse(c: &mut CompilerState, s: &mut JavaTokenStream) -> ParseResult<Self>;
}
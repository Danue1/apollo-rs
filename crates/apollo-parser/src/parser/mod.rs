use std::cell::RefCell;
use std::rc::Rc;

use crate::lexer;
use crate::lexer::Lexer;
use crate::TokenKind;

pub use generated::syntax_kind::SyntaxKind;
pub use language::{SyntaxElement, SyntaxElementChildren, SyntaxNodeChildren, SyntaxToken};
pub use syntax_tree::SyntaxTree;

pub(crate) use language::{GraphQLLanguage, SyntaxNode};
pub(crate) use parse_directive::parse_directive;
pub(crate) use parse_directive_locations::parse_directive_locations;
pub(crate) use parse_fragment::parse_fragment;
pub(crate) use parse_fragment_name::parse_fragment_name;
pub(crate) use parse_input_value_definitions::parse_input_value_definitions;
pub(crate) use parse_name::parse_name;
pub(crate) use syntax_tree::SyntaxTreeBuilder;
pub(crate) use token_text::TokenText;

mod generated;
mod language;
mod parse_directive;
mod parse_directive_locations;
mod parse_fragment;
mod parse_fragment_name;
mod parse_input_value_definitions;
mod parse_name;
mod syntax_tree;
mod token_text;

/// Parse text into an AST.
#[derive(Debug)]
pub struct Parser {
    /// input tokens, including whitespace,
    /// in *reverse* order.
    tokens: Vec<lexer::Token>,
    /// the in-progress tree.
    builder: Rc<RefCell<SyntaxTreeBuilder>>,
    /// the list of syntax errors we've accumulated
    /// so far.
    errors: Vec<crate::Error>,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let lexer = Lexer::new(&input);

        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for s in lexer.tokens().to_owned() {
            match s {
                Ok(t) => tokens.push(t),
                Err(e) => errors.push(e),
            }
        }

        tokens.reverse();
        errors.reverse();

        Self {
            tokens,
            builder: Rc::new(RefCell::new(SyntaxTreeBuilder::new())),
            errors,
        }
    }

    pub fn parse(mut self) -> SyntaxTree {
        let guard = self.start_node(SyntaxKind::DOCUMENT);

        loop {
            match self.peek() {
                None => break,
                Some(TokenKind::Fragment) => {
                    parse_fragment(&mut self).unwrap_or_else(|e| self.errors.push(e));
                }
                Some(TokenKind::Directive) => {
                    parse_directive(&mut self).unwrap_or_else(|e| self.errors.push(e));
                }
                Some(_) => break,
            }
        }

        guard.finish_node();

        let builder = Rc::try_unwrap(self.builder)
            .expect("More than one reference to builder left")
            .into_inner();
        builder.finish(self.errors)
    }

    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        let token = self.tokens.pop().unwrap();
        self.builder.borrow_mut().token(kind, token.data());
    }

    pub(crate) fn start_node(&mut self, kind: SyntaxKind) -> NodeGuard {
        self.builder.borrow_mut().start_node(kind);
        NodeGuard::new(self.builder.clone())
    }

    pub(crate) fn peek(&self) -> Option<TokenKind> {
        self.tokens.last().map(|token| token.kind())
    }

    pub(crate) fn peek_data(&self) -> Option<String> {
        self.tokens.last().map(|token| token.data().to_string())
    }
}

#[must_use]
pub(crate) struct NodeGuard {
    builder: Rc<RefCell<SyntaxTreeBuilder>>,
}

impl NodeGuard {
    fn new(builder: Rc<RefCell<SyntaxTreeBuilder>>) -> Self {
        Self { builder }
    }

    pub(crate) fn finish_node(self) {
        drop(self);
    }
}

impl Drop for NodeGuard {
    fn drop(&mut self) {
        self.builder.borrow_mut().finish_node();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke_fragment() {
        let input = "fragment friendFields on User {
            id name profilePic(size: 5.0)
        }";
        let parser = Parser::new(input);
        println!("{:?}", parser.parse());
    }

    #[test]
    fn smoke_directive() {
        let input = "directive @example(isTreat: Boolean, treatKind: String) on FIELD | MUTATION";
        let parser = Parser::new(input);

        println!("{:?}", parser.parse());
    }

    #[test]
    fn smoke_directive_with_errors() {
        let input =
            "directive ø @example(isTreat: ø Boolean, treatKind: String) on FIELD | MUTATION";
        let parser = Parser::new(input);

        println!("{:?}", parser.parse());
    }
}

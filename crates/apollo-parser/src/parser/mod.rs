mod generated;
mod language;
mod syntax_tree;
mod token_text;

pub(crate) mod grammar;

use std::{cell::RefCell, rc::Rc};

use crate::{lexer::Lexer, Error, Token, TokenKind};

pub use generated::syntax_kind::SyntaxKind;
pub use language::{SyntaxElement, SyntaxNodeChildren, SyntaxToken};
pub use syntax_tree::SyntaxTree;

pub(crate) use language::{GraphQLLanguage, SyntaxNode};
pub(crate) use syntax_tree::SyntaxTreeBuilder;
pub(crate) use token_text::TokenText;

/// Parse GraphQL schemas or queries into a typed AST.
///
/// ## Example
///
/// The API to parse a query or a schema is the same, as the parser currently
/// accepts a `&str`. Here is an example of parsing a query:
/// ```rust
/// use apollo_parser::Parser;
///
/// let query = "
/// {
///     animal
///     ...snackSelection
///     ... on Pet {
///       playmates {
///         count
///       }
///     }
/// }
/// ";
/// // Create a new instance of a parser given a query above.
/// let parser = Parser::new(query);
/// // Parse the query, and return a SyntaxTree.
/// let ast = parser.parse();
/// // Check that are no errors. These are not part of the AST.
/// assert_eq!(0, ast.errors().len());
///
/// // Get the document root node
/// let doc = ast.document();
/// // ... continue
/// ```
///
/// Here is how you'd parse a schema:
/// ```rust
/// use apollo_parser::Parser;
/// let core_schema = r#"
/// schema @core(feature: "https://specs.apollo.dev/join/v0.1") {
///   query: Query
///   mutation: Mutation
/// }
///
/// enum join__Graph {
///   ACCOUNTS @join__graph(name: "accounts")
/// }
/// "#;
/// let parser = Parser::new(core_schema);
/// let ast = parser.parse();
///
/// assert_eq!(0, ast.errors().len());
///
/// let document = ast.document();
/// ```
#[derive(Debug)]
pub struct Parser {
    /// Input tokens, including whitespace, in *reverse* order.
    tokens: Vec<Token>,
    /// The in-progress tree.
    builder: Rc<RefCell<SyntaxTreeBuilder>>,
    /// The list of syntax errors we've accumulated so far.
    errors: Vec<crate::Error>,
}

impl Parser {
    /// Create a new instance of a parser given an input string.
    pub fn new(input: &str) -> Self {
        let lexer = Lexer::new(input);

        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for s in lexer.tokens().to_owned() {
            tokens.push(s);
        }

        for e in lexer.errors().cloned() {
            errors.push(e);
        }

        tokens.reverse();
        errors.reverse();

        Self {
            tokens,
            builder: Rc::new(RefCell::new(SyntaxTreeBuilder::new())),
            errors,
        }
    }

    /// Parse the current tokens.
    pub fn parse(mut self) -> SyntaxTree {
        grammar::document::document(&mut self);

        let builder = Rc::try_unwrap(self.builder)
            .expect("More than one reference to builder left")
            .into_inner();
        builder.finish(self.errors)
    }

    /// Check if the current token is `kind`.
    pub(crate) fn at(&mut self, token: TokenKind) -> bool {
        if let Some(t) = self.peek() {
            if t == token {
                return true;
            }
            return false;
        }

        false
    }

    /// Consume a token and any ignored tokens that follow, then add it to AST.
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        self.eat(kind);
        self.bump_ignored();
    }

    /// Consume ignored tokens and add them to the AST.
    fn bump_ignored(&mut self) {
        while let Some(TokenKind::Comment | TokenKind::Whitespace | TokenKind::Comma) = self.peek()
        {
            if let Some(TokenKind::Comment) = self.peek() {
                self.bump(SyntaxKind::COMMENT);
            }
            if let Some(TokenKind::Whitespace) = self.peek() {
                self.bump(SyntaxKind::WHITESPACE);
            }
            if let Some(TokenKind::Comma) = self.peek() {
                self.bump(SyntaxKind::COMMA);
            }
        }
    }

    /// Get current token's data.
    pub(crate) fn current(&mut self) -> &Token {
        self.peek_token()
            .expect("Could not peek at the current token")
    }

    /// Consume a token from the lexer and add it to the AST.
    fn eat(&mut self, kind: SyntaxKind) {
        let token = self
            .tokens
            .pop()
            .expect("Could not eat a token from the AST");
        self.builder.borrow_mut().token(kind, token.data());
    }

    /// Create a parser error and push it into the error vector.
    pub(crate) fn err(&mut self, message: &str) {
        let current = self.current().clone();
        // this needs to be the computed location
        let err = Error::with_loc(message, current.data().to_string(), current.index());
        self.push_err(err);
    }

    /// Create a parser error and push it into the error vector.
    pub(crate) fn err_and_pop(&mut self, message: &str) {
        let current = self.pop();
        // we usually bump ignored after we pop a token, so make sure we also do
        // this when we create an error and pop.
        self.bump_ignored();
        // this needs to be the computed location
        let err = Error::with_loc(message, current.data().to_string(), current.index());
        self.push_err(err);
    }

    /// Consume the next token if it is `kind` or emit an error
    /// otherwise.
    pub(crate) fn expect(&mut self, token: TokenKind, kind: SyntaxKind) {
        let current = self.current().clone();
        let data = current.data().to_string();

        if self.at(token) {
            self.bump(kind);
            return;
        }

        let err = Error::with_loc(
            format!("expected {:?}, got {}", kind, data),
            data,
            current.index(),
        );

        self.push_err(err);
    }

    /// Push an error to parser's error Vec.
    pub(crate) fn push_err(&mut self, err: crate::error::Error) {
        self.errors.push(err);
    }

    /// Consume a token from the lexer.
    pub(crate) fn pop(&mut self) -> Token {
        self.tokens
            .pop()
            .expect("Could not pop a token from the AST")
    }

    /// Insert a token into the AST.
    pub(crate) fn push_ast(&mut self, kind: SyntaxKind, token: Token) {
        self.builder.borrow_mut().token(kind, token.data())
    }

    /// Start a node and make it current.
    ///
    /// This also creates a NodeGuard under the hood that will automatically
    /// close the node(via Drop) when the guard goes out of scope.
    /// This allows for us to not have to always close nodes when we are parsing
    /// tokens.
    pub(crate) fn start_node(&mut self, kind: SyntaxKind) -> NodeGuard {
        self.builder.borrow_mut().start_node(kind);
        let guard = NodeGuard::new(self.builder.clone());
        self.bump_ignored();

        guard
    }

    /// Peek the next Token and return its TokenKind.
    pub(crate) fn peek(&self) -> Option<TokenKind> {
        self.tokens.last().map(|token| token.kind())
    }

    /// Peek the next Token and return it.
    pub(crate) fn peek_token(&self) -> Option<&Token> {
        self.tokens.last()
    }

    /// Peek Token `n` and return its TokenKind.
    pub(crate) fn peek_n(&self, n: usize) -> Option<TokenKind> {
        self.tokens
            .iter()
            .rev()
            .filter(|token| !matches!(token.kind(), TokenKind::Whitespace | TokenKind::Comment))
            .nth(n - 1)
            .map(|token| token.kind())
    }

    /// Peek next Token's `data` property.
    pub(crate) fn peek_data(&self) -> Option<String> {
        self.tokens.last().map(|token| token.data().to_string())
    }

    /// Peek `n` Token's `data` property.
    pub(crate) fn peek_data_n(&self, n: usize) -> Option<String> {
        self.tokens
            .iter()
            .rev()
            .filter(|token| !matches!(token.kind(), TokenKind::Whitespace | TokenKind::Comment))
            .nth(n - 1)
            .map(|token| token.data().to_string())
    }
}

/// A wrapper around the SyntaxTreeBuilder used to self-close nodes.
///
/// When the NodeGuard goes out of scope, it automatically runs `finish_node()`
/// on the SyntaxTreeBuilder. This ensures that nodes are not forgotten to be
/// closed.
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

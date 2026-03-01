use crate::ast::*;
use crate::error::AssembleError;
use crate::lexer::{Token, TokenKind};

/// Recursive-descent parser for the `.writil` text format.
///
/// Produces an `AsmModule` AST from a token sequence. Collects multiple
/// errors via synchronization rather than failing on the first error.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<AssembleError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    // ── Core navigation ─────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(self.tokens.last().unwrap())
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline) {
            self.pos += 1;
        }
    }

    fn expect_directive(&mut self, name: &str) -> bool {
        self.skip_newlines();
        if let TokenKind::Directive(d) = self.peek_kind() {
            if d == name {
                self.pos += 1;
                return true;
            }
        }
        let tok = self.peek();
        self.errors.push(AssembleError::new(
            format!("expected '.{}' directive", name),
            tok.line,
            tok.col,
        ));
        false
    }

    fn expect_string(&mut self) -> Option<String> {
        self.skip_newlines();
        if let TokenKind::StringLit(s) = self.peek_kind() {
            let s = s.clone();
            self.pos += 1;
            return Some(s);
        }
        let tok = self.peek();
        self.errors.push(AssembleError::new("expected string literal", tok.line, tok.col));
        None
    }

    fn expect_ident(&mut self) -> Option<String> {
        self.skip_newlines();
        if let TokenKind::Ident(s) = self.peek_kind() {
            let s = s.clone();
            self.pos += 1;
            return Some(s);
        }
        let tok = self.peek();
        self.errors.push(AssembleError::new("expected identifier", tok.line, tok.col));
        None
    }

    fn expect_int(&mut self) -> Option<i64> {
        self.skip_newlines();
        if let TokenKind::IntLit(v) = self.peek_kind() {
            let v = *v;
            self.pos += 1;
            return Some(v);
        }
        let tok = self.peek();
        self.errors.push(AssembleError::new("expected integer literal", tok.line, tok.col));
        None
    }

    fn expect_token(&mut self, expected: &TokenKind) -> bool {
        self.skip_newlines();
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(expected) {
            self.pos += 1;
            return true;
        }
        let tok = self.peek();
        self.errors.push(AssembleError::new(
            format!("expected {:?}", expected),
            tok.line,
            tok.col,
        ));
        false
    }

    /// Skip tokens until we reach a synchronization point: `}`, a directive at
    /// the start of a line, or EOF.
    fn synchronize(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::CloseBrace | TokenKind::Eof => break,
                TokenKind::Directive(_) => break,
                _ => { self.pos += 1; }
            }
        }
    }

    // ── Parse functions ─────────────────────────────────────────

    /// Parse the top-level `.module` block.
    pub fn parse_module(&mut self) -> AsmModule {
        self.skip_newlines();
        let mut module = AsmModule {
            name: String::new(),
            version: String::new(),
            externs: Vec::new(),
            types: Vec::new(),
            contracts: Vec::new(),
            impls: Vec::new(),
            globals: Vec::new(),
            extern_fns: Vec::new(),
            methods: Vec::new(),
        };

        if !self.expect_directive("module") {
            return module;
        }

        module.name = self.expect_string().unwrap_or_default();
        module.version = self.expect_string().unwrap_or_default();

        if !self.expect_token(&TokenKind::OpenBrace) {
            return module;
        }

        // Parse module items until closing brace
        loop {
            self.skip_newlines();
            if self.at_end() || matches!(self.peek_kind(), TokenKind::CloseBrace) {
                break;
            }

            // Clone the directive name to avoid borrow issues
            let directive_name = if let TokenKind::Directive(d) = self.peek_kind() {
                Some(d.clone())
            } else {
                None
            };

            if let Some(dir) = directive_name {
                match dir.as_str() {
                    "type" => {
                        if let Some(t) = self.parse_type() {
                            module.types.push(t);
                        }
                    }
                    "contract" => {
                        if let Some(c) = self.parse_contract() {
                            module.contracts.push(c);
                        }
                    }
                    "impl" => {
                        if let Some(i) = self.parse_impl() {
                            module.impls.push(i);
                        }
                    }
                    "method" => {
                        if let Some(m) = self.parse_method() {
                            module.methods.push(m);
                        }
                    }
                    "extern" => {
                        if let Some(ext) = self.parse_extern() {
                            module.externs.push(ext);
                        }
                    }
                    "global" => {
                        if let Some(g) = self.parse_global() {
                            module.globals.push(g);
                        }
                    }
                    other => {
                        let tok = self.peek();
                        self.errors.push(AssembleError::new(
                            format!("unknown directive '.{}'", other),
                            tok.line,
                            tok.col,
                        ));
                        self.synchronize();
                    }
                }
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new(
                    "expected directive or '}'",
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            }
        }

        self.expect_token(&TokenKind::CloseBrace);

        module
    }

    fn parse_type(&mut self) -> Option<AsmType> {
        self.pos += 1; // consume .type
        let name = self.expect_string()?;

        // Parse kind
        let kind_str = self.expect_ident()?;
        let kind = match kind_str.to_lowercase().as_str() {
            "struct" => AsmTypeKind::Struct,
            "enum" => AsmTypeKind::Enum,
            "entity" => AsmTypeKind::Entity,
            "component" => AsmTypeKind::Component,
            _ => {
                let tok = &self.tokens[self.pos - 1];
                self.errors.push(AssembleError::new(
                    format!("unknown type kind '{}'", kind_str),
                    tok.line,
                    tok.col,
                ));
                AsmTypeKind::Struct
            }
        };

        // Parse optional flags before the block
        let flags = self.parse_flags();

        if !self.expect_token(&TokenKind::OpenBrace) {
            self.synchronize();
            return Some(AsmType { name, kind, flags, fields: Vec::new() });
        }

        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() || matches!(self.peek_kind(), TokenKind::CloseBrace) {
                break;
            }
            let is_field = matches!(self.peek_kind(), TokenKind::Directive(d) if d == "field");
            let is_directive = matches!(self.peek_kind(), TokenKind::Directive(_));

            if is_field {
                if let Some(f) = self.parse_field() {
                    fields.push(f);
                }
            } else if is_directive {
                let tok = self.peek();
                let d = if let TokenKind::Directive(d) = self.peek_kind() { d.clone() } else { String::new() };
                self.errors.push(AssembleError::new(
                    format!("expected '.field' inside .type, got '.{}'", d),
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new(
                    "expected '.field' or '}'",
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            }
        }

        self.expect_token(&TokenKind::CloseBrace);
        Some(AsmType { name, kind, flags, fields })
    }

    fn parse_field(&mut self) -> Option<AsmField> {
        self.pos += 1; // consume .field
        let name = self.expect_string()?;
        let type_ref = self.parse_type_ref()?;
        let flags = self.parse_flags();
        Some(AsmField { name, type_ref, flags })
    }

    fn parse_contract(&mut self) -> Option<AsmContract> {
        self.pos += 1; // consume .contract
        let name = self.expect_string()?;

        // Parse optional generic params: <T, U>
        let mut generic_params = Vec::new();
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::LAngle) {
            self.pos += 1; // consume <
            loop {
                self.skip_newlines();
                if matches!(self.peek_kind(), TokenKind::RAngle) {
                    self.pos += 1;
                    break;
                }
                if let Some(param) = self.expect_ident() {
                    generic_params.push(param);
                }
                self.skip_newlines();
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.pos += 1;
                }
            }
        }

        if !self.expect_token(&TokenKind::OpenBrace) {
            self.synchronize();
            return Some(AsmContract { name, methods: Vec::new(), generic_params });
        }

        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() || matches!(self.peek_kind(), TokenKind::CloseBrace) {
                break;
            }
            let is_method = matches!(self.peek_kind(), TokenKind::Directive(d) if d == "method");
            let is_directive = matches!(self.peek_kind(), TokenKind::Directive(_));

            if is_method {
                if let Some(m) = self.parse_contract_method() {
                    methods.push(m);
                }
            } else if is_directive {
                let tok = self.peek();
                let d = if let TokenKind::Directive(d) = self.peek_kind() { d.clone() } else { String::new() };
                self.errors.push(AssembleError::new(
                    format!("expected '.method' inside .contract, got '.{}'", d),
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new(
                    "expected '.method' or '}'",
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            }
        }

        self.expect_token(&TokenKind::CloseBrace);
        Some(AsmContract { name, methods, generic_params })
    }

    fn parse_contract_method(&mut self) -> Option<AsmContractMethod> {
        self.pos += 1; // consume .method
        let name = self.expect_string()?;

        // Parse signature: (type, type) -> return_type
        let signature = self.parse_method_sig()?;

        // Parse slot keyword and number
        self.skip_newlines();
        if let TokenKind::Ident(s) = self.peek_kind() {
            if s.to_lowercase() == "slot" {
                self.pos += 1;
            }
        }
        let slot = self.expect_int()? as u16;

        Some(AsmContractMethod { name, signature, slot })
    }

    fn parse_method_sig(&mut self) -> Option<AsmMethodSig> {
        if !self.expect_token(&TokenKind::OpenParen) {
            return None;
        }

        let mut params = Vec::new();
        self.skip_newlines();
        if !matches!(self.peek_kind(), TokenKind::CloseParen) {
            if let Some(t) = self.parse_type_ref() {
                params.push(t);
            }
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.pos += 1; // consume comma
                if let Some(t) = self.parse_type_ref() {
                    params.push(t);
                }
            }
        }

        self.expect_token(&TokenKind::CloseParen);

        // Parse -> return_type
        self.skip_newlines();
        let return_type = if matches!(self.peek_kind(), TokenKind::Arrow) {
            self.pos += 1;
            self.parse_type_ref().unwrap_or(AsmTypeRef::Void)
        } else {
            AsmTypeRef::Void
        };

        Some(AsmMethodSig { params, return_type })
    }

    fn parse_impl(&mut self) -> Option<AsmImpl> {
        self.pos += 1; // consume .impl
        let type_name = self.expect_ident()?;

        self.expect_token(&TokenKind::Colon);
        let contract_name = self.expect_ident()?;

        if !self.expect_token(&TokenKind::OpenBrace) {
            self.synchronize();
            return Some(AsmImpl { type_name, contract_name, methods: Vec::new() });
        }

        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() || matches!(self.peek_kind(), TokenKind::CloseBrace) {
                break;
            }
            let is_method = matches!(self.peek_kind(), TokenKind::Directive(d) if d == "method");
            let is_directive = matches!(self.peek_kind(), TokenKind::Directive(_));

            if is_method {
                if let Some(m) = self.parse_method() {
                    methods.push(m);
                }
            } else if is_directive {
                let tok = self.peek();
                let d = if let TokenKind::Directive(d) = self.peek_kind() { d.clone() } else { String::new() };
                self.errors.push(AssembleError::new(
                    format!("expected '.method' inside .impl, got '.{}'", d),
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new(
                    "expected '.method' or '}'",
                    tok.line,
                    tok.col,
                ));
                self.synchronize();
            }
        }

        self.expect_token(&TokenKind::CloseBrace);
        Some(AsmImpl { type_name, contract_name, methods })
    }

    fn parse_method(&mut self) -> Option<AsmMethod> {
        self.pos += 1; // consume .method
        let name = self.expect_string()?;

        // Parse parameters: (name type, name type) or (type, type)
        if !self.expect_token(&TokenKind::OpenParen) {
            self.synchronize();
            return None;
        }

        let mut params = Vec::new();
        self.skip_newlines();
        if !matches!(self.peek_kind(), TokenKind::CloseParen) {
            if let Some(p) = self.parse_param() {
                params.push(p);
            }
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.pos += 1; // consume comma
                if let Some(p) = self.parse_param() {
                    params.push(p);
                }
            }
        }

        self.expect_token(&TokenKind::CloseParen);

        // Parse -> return_type
        self.skip_newlines();
        let return_type = if matches!(self.peek_kind(), TokenKind::Arrow) {
            self.pos += 1;
            self.parse_type_ref().unwrap_or(AsmTypeRef::Void)
        } else {
            AsmTypeRef::Void
        };

        // Parse optional flags
        let flags = self.parse_flags();

        if !self.expect_token(&TokenKind::OpenBrace) {
            self.synchronize();
            return Some(AsmMethod {
                name,
                params,
                return_type,
                registers: Vec::new(),
                body: Vec::new(),
                flags,
            });
        }

        // Parse register declarations and instructions
        let mut registers = Vec::new();
        let mut body = Vec::new();

        loop {
            self.skip_newlines();
            if self.at_end() || matches!(self.peek_kind(), TokenKind::CloseBrace) {
                break;
            }

            let is_reg = matches!(self.peek_kind(), TokenKind::Directive(d) if d == "reg");
            let is_label = matches!(self.peek_kind(), TokenKind::Label(_));
            let is_ident = matches!(self.peek_kind(), TokenKind::Ident(_));

            if is_reg {
                if let Some(r) = self.parse_reg_decl() {
                    registers.push(r);
                }
            } else if is_label {
                let label_name = if let TokenKind::Label(n) = self.peek_kind() {
                    n.clone()
                } else {
                    unreachable!()
                };
                self.pos += 1;
                body.push(AsmStatement::Label(label_name));
            } else if is_ident {
                if let Some(instr) = self.parse_instruction() {
                    body.push(AsmStatement::Instruction(instr));
                }
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new(
                    "expected instruction, label, '.reg', or '}'",
                    tok.line,
                    tok.col,
                ));
                self.pos += 1;
            }
        }

        self.expect_token(&TokenKind::CloseBrace);

        Some(AsmMethod {
            name,
            params,
            return_type,
            registers,
            body,
            flags,
        })
    }

    fn parse_param(&mut self) -> Option<AsmParam> {
        self.skip_newlines();

        // Special case: register-like param names (r0, r1)
        if let TokenKind::Register(n) = self.peek_kind() {
            let name = format!("r{}", n);
            self.pos += 1;
            let type_ref = self.parse_type_ref()?;
            return Some(AsmParam { name, type_ref });
        }

        // Check if this is a named param
        if let TokenKind::Ident(id) = self.peek_kind() {
            let id_clone = id.clone();
            // If this identifier is a type keyword, it's probably a type-only param
            if is_type_keyword(&id_clone) {
                let type_ref = self.parse_type_ref()?;
                return Some(AsmParam { name: String::new(), type_ref });
            }
            // Otherwise peek ahead: if followed by another type-like token, it's a named param
            if self.pos + 1 < self.tokens.len() {
                let next_kind = &self.tokens[self.pos + 1].kind;
                match next_kind {
                    TokenKind::Ident(_) | TokenKind::Directive(_) => {
                        // Named param
                        self.pos += 1;
                        let type_ref = self.parse_type_ref()?;
                        return Some(AsmParam { name: id_clone, type_ref });
                    }
                    _ => {}
                }
            }
            // Just a type
            let type_ref = self.parse_type_ref()?;
            return Some(AsmParam { name: String::new(), type_ref });
        }

        let type_ref = self.parse_type_ref()?;
        Some(AsmParam { name: String::new(), type_ref })
    }

    fn parse_reg_decl(&mut self) -> Option<AsmRegDecl> {
        self.pos += 1; // consume .reg
        self.skip_newlines();

        let index = if let TokenKind::Register(n) = self.peek_kind() {
            let n = *n;
            self.pos += 1;
            n
        } else {
            let tok = self.peek();
            self.errors.push(AssembleError::new("expected register (e.g., r0)", tok.line, tok.col));
            return None;
        };

        let type_ref = self.parse_type_ref()?;
        Some(AsmRegDecl { index, type_ref })
    }

    fn parse_instruction(&mut self) -> Option<AsmInstruction> {
        self.skip_newlines();
        let line = self.peek().line;
        let col = self.peek().col;

        let mnemonic = if let TokenKind::Ident(s) = self.peek_kind() {
            let m = s.clone();
            self.pos += 1;
            m
        } else {
            return None;
        };

        // Parse operands until newline, EOF, or closing brace
        let mut operands = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof | TokenKind::CloseBrace => break,
                _ => {}
            }

            if !operands.is_empty() {
                // Expect comma between operands
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.pos += 1;
                }
            }

            if let Some(op) = self.parse_operand() {
                operands.push(op);
            } else {
                // Failed to parse operand, skip to next line
                while !matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Eof | TokenKind::CloseBrace) {
                    self.pos += 1;
                }
                break;
            }
        }

        Some(AsmInstruction { mnemonic, operands, line, col })
    }

    fn parse_operand(&mut self) -> Option<AsmOperand> {
        match self.peek_kind() {
            TokenKind::Register(n) => {
                let n = *n;
                self.pos += 1;
                Some(AsmOperand::Register(n))
            }
            TokenKind::IntLit(v) => {
                let v = *v;
                self.pos += 1;
                Some(AsmOperand::IntLit(v))
            }
            TokenKind::FloatLit(v) => {
                let v = *v;
                self.pos += 1;
                Some(AsmOperand::FloatLit(v))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.pos += 1;
                Some(AsmOperand::StringLit(s))
            }
            TokenKind::LabelRef(name) => {
                let name = name.clone();
                self.pos += 1;
                Some(AsmOperand::LabelRef(name))
            }
            TokenKind::Ident(s) => {
                let s_clone = s.clone();

                // Check for token(...) syntax
                if s_clone == "token" && self.pos + 1 < self.tokens.len()
                    && matches!(self.tokens[self.pos + 1].kind, TokenKind::OpenParen)
                {
                    self.pos += 1; // consume "token"
                    self.pos += 1; // consume "("
                    if let Some(v) = self.expect_int() {
                        self.expect_token(&TokenKind::CloseParen);
                        return Some(AsmOperand::Token(v as u32));
                    }
                    return None;
                }

                // Check for qualified ref: TypeName::member
                if self.pos + 1 < self.tokens.len()
                    && matches!(self.tokens[self.pos + 1].kind, TokenKind::DoubleColon)
                {
                    self.pos += 1; // consume ident
                    self.pos += 1; // consume ::
                    if let TokenKind::Ident(member) = self.peek_kind() {
                        let member = member.clone();
                        self.pos += 1;
                        return Some(AsmOperand::MethodRef(AsmMethodRef {
                            type_name: Some(s_clone),
                            method_name: member,
                            module_name: None,
                        }));
                    }
                }

                // Just an identifier - use as type ref in instruction context
                self.pos += 1;
                if is_type_keyword(&s_clone) {
                    Some(AsmOperand::TypeRef(keyword_to_type_ref(&s_clone)))
                } else {
                    Some(AsmOperand::TypeRef(AsmTypeRef::Named(s_clone)))
                }
            }
            TokenKind::OpenBracket => {
                // Cross-module reference: [Module]Type::method
                self.pos += 1; // consume [
                let module_name = if let TokenKind::Ident(s) = self.peek_kind() {
                    let s = s.clone();
                    self.pos += 1;
                    s
                } else {
                    let tok = self.peek();
                    self.errors.push(AssembleError::new("expected module name", tok.line, tok.col));
                    return None;
                };
                self.expect_token(&TokenKind::CloseBracket);

                let type_name = if let TokenKind::Ident(s) = self.peek_kind() {
                    let s = s.clone();
                    self.pos += 1;
                    s
                } else {
                    let tok = self.peek();
                    self.errors.push(AssembleError::new("expected type name", tok.line, tok.col));
                    return None;
                };

                self.expect_token(&TokenKind::DoubleColon);

                let method_name = if let TokenKind::Ident(s) = self.peek_kind() {
                    let s = s.clone();
                    self.pos += 1;
                    s
                } else {
                    let tok = self.peek();
                    self.errors.push(AssembleError::new("expected method name", tok.line, tok.col));
                    return None;
                };

                Some(AsmOperand::MethodRef(AsmMethodRef {
                    type_name: Some(type_name),
                    method_name,
                    module_name: Some(module_name),
                }))
            }
            _ => None,
        }
    }

    fn parse_type_ref(&mut self) -> Option<AsmTypeRef> {
        self.skip_newlines();

        // Handle blob(...) as an identifier
        let is_blob_ident = matches!(self.peek_kind(), TokenKind::Ident(s) if s == "blob");

        if is_blob_ident {
            self.pos += 1;
            return self.parse_blob_literal();
        }

        match self.peek_kind() {
            TokenKind::Ident(s) => {
                let s_clone = s.clone();
                self.pos += 1;

                // Check for generic: Name<T>
                if matches!(self.peek_kind(), TokenKind::LAngle) {
                    self.pos += 1; // consume <
                    let mut args = Vec::new();
                    loop {
                        self.skip_newlines();
                        if matches!(self.peek_kind(), TokenKind::RAngle) {
                            self.pos += 1;
                            break;
                        }
                        if let Some(arg) = self.parse_type_ref() {
                            args.push(arg);
                        } else {
                            break;
                        }
                        if matches!(self.peek_kind(), TokenKind::Comma) {
                            self.pos += 1;
                        }
                    }

                    // Array<T> is a special case
                    let lower = s_clone.to_lowercase();
                    if lower == "array" && args.len() == 1 {
                        return Some(AsmTypeRef::Array(Box::new(args.into_iter().next().unwrap())));
                    }
                    return Some(AsmTypeRef::Generic(s_clone, args));
                }

                // Primitive type keywords
                if is_type_keyword(&s_clone) {
                    Some(keyword_to_type_ref(&s_clone))
                } else {
                    Some(AsmTypeRef::Named(s_clone))
                }
            }
            TokenKind::Directive(d) if d == "blob" => {
                self.pos += 1;
                self.parse_blob_literal()
            }
            _ => {
                let tok = self.peek();
                self.errors.push(AssembleError::new("expected type reference", tok.line, tok.col));
                None
            }
        }
    }

    fn parse_blob_literal(&mut self) -> Option<AsmTypeRef> {
        if !self.expect_token(&TokenKind::OpenParen) {
            return None;
        }
        let mut bytes = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::CloseParen) {
                self.pos += 1;
                break;
            }
            if let TokenKind::IntLit(v) = self.peek_kind() {
                bytes.push(*v as u8);
                self.pos += 1;
            } else {
                let tok = self.peek();
                self.errors.push(AssembleError::new("expected byte value in blob literal", tok.line, tok.col));
                break;
            }
        }
        Some(AsmTypeRef::RawBlob(bytes))
    }

    fn parse_flags(&mut self) -> u16 {
        let mut flags = 0u16;
        loop {
            self.skip_newlines();
            let flag_word = if let TokenKind::Ident(s) = self.peek_kind() {
                Some(s.to_lowercase())
            } else {
                None
            };

            if let Some(word) = flag_word {
                match word.as_str() {
                    "pub" => { flags |= 0x0001; self.pos += 1; }
                    "mut" => { flags |= 0x0002; self.pos += 1; }
                    "static" => { flags |= 0x0004; self.pos += 1; }
                    _ => break,
                }
            } else if let TokenKind::IntLit(v) = self.peek_kind() {
                // Numeric flag literal
                flags |= *v as u16;
                self.pos += 1;
                break;
            } else {
                break;
            }
        }
        flags
    }

    fn parse_extern(&mut self) -> Option<AsmExtern> {
        self.pos += 1; // consume .extern
        let name = self.expect_string()?;
        let min_version = self.expect_string()?;
        Some(AsmExtern { name, min_version })
    }

    fn parse_global(&mut self) -> Option<AsmGlobal> {
        self.pos += 1; // consume .global
        let name = self.expect_string()?;
        let type_ref = self.parse_type_ref()?;
        let flags = self.parse_flags();
        Some(AsmGlobal { name, type_ref, flags, init_value: None })
    }
}

/// Parse tokens into an AsmModule, collecting errors.
pub fn parse(tokens: &[Token]) -> Result<AsmModule, Vec<AssembleError>> {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    if parser.errors.is_empty() {
        Ok(module)
    } else {
        Err(parser.errors)
    }
}

/// Check if an identifier is a primitive type keyword.
fn is_type_keyword(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "int" | "float" | "bool" | "string" | "void")
}

/// Convert a type keyword to the corresponding AsmTypeRef.
fn keyword_to_type_ref(s: &str) -> AsmTypeRef {
    match s.to_lowercase().as_str() {
        "int" => AsmTypeRef::Int,
        "float" => AsmTypeRef::Float,
        "bool" => AsmTypeRef::Bool,
        "string" => AsmTypeRef::String_,
        "void" => AsmTypeRef::Void,
        _ => AsmTypeRef::Named(s.to_string()),
    }
}

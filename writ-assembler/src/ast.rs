/// Top-level module AST node.
#[derive(Debug, Clone)]
pub struct AsmModule {
    pub name: String,
    pub version: String,
    pub externs: Vec<AsmExtern>,
    pub types: Vec<AsmType>,
    pub contracts: Vec<AsmContract>,
    pub impls: Vec<AsmImpl>,
    pub globals: Vec<AsmGlobal>,
    pub extern_fns: Vec<AsmExternFn>,
    pub methods: Vec<AsmMethod>,
}

/// External module reference: `.extern "name" "min_version"`.
#[derive(Debug, Clone)]
pub struct AsmExtern {
    pub name: String,
    pub min_version: String,
}

/// Type definition: `.type "Name" kind { fields }`.
#[derive(Debug, Clone)]
pub struct AsmType {
    pub name: String,
    pub kind: AsmTypeKind,
    pub flags: u16,
    pub fields: Vec<AsmField>,
}

/// The kind of a type definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsmTypeKind {
    Struct,
    Enum,
    Entity,
    Component,
}

/// Field definition: `.field "name" type_ref flags`.
#[derive(Debug, Clone)]
pub struct AsmField {
    pub name: String,
    pub type_ref: AsmTypeRef,
    pub flags: u16,
}

/// Contract definition: `.contract "Name" { methods }`.
#[derive(Debug, Clone)]
pub struct AsmContract {
    pub name: String,
    pub methods: Vec<AsmContractMethod>,
    pub generic_params: Vec<String>,
}

/// Contract method slot: `.method "name" signature slot N`.
#[derive(Debug, Clone)]
pub struct AsmContractMethod {
    pub name: String,
    pub signature: AsmMethodSig,
    pub slot: u16,
}

/// Contract/impl block: `.impl TypeName : ContractName { methods }`.
#[derive(Debug, Clone)]
pub struct AsmImpl {
    pub type_name: String,
    pub contract_name: String,
    pub methods: Vec<AsmMethod>,
}

/// Method definition: `.method "name" (params) -> return_type { body }`.
#[derive(Debug, Clone)]
pub struct AsmMethod {
    pub name: String,
    pub params: Vec<AsmParam>,
    pub return_type: AsmTypeRef,
    pub registers: Vec<AsmRegDecl>,
    pub body: Vec<AsmStatement>,
    pub flags: u16,
}

/// Method parameter with name and type.
#[derive(Debug, Clone)]
pub struct AsmParam {
    pub name: String,
    pub type_ref: AsmTypeRef,
}

/// Register declaration: `.reg r0 int`.
#[derive(Debug, Clone)]
pub struct AsmRegDecl {
    pub index: u16,
    pub type_ref: AsmTypeRef,
}

/// A statement in a method body: either a label or an instruction.
#[derive(Debug, Clone)]
pub enum AsmStatement {
    Label(String),
    Instruction(AsmInstruction),
}

/// A single instruction with mnemonic and operands.
#[derive(Debug, Clone)]
pub struct AsmInstruction {
    pub mnemonic: String,
    pub operands: Vec<AsmOperand>,
    pub line: u32,
    pub col: u32,
}

/// Instruction operand types.
#[derive(Debug, Clone)]
pub enum AsmOperand {
    Register(u16),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    LabelRef(String),
    TypeRef(AsmTypeRef),
    MethodRef(AsmMethodRef),
    FieldRef(AsmFieldRef),
    Token(u32),
}

/// Type reference in text IL.
#[derive(Debug, Clone, PartialEq)]
pub enum AsmTypeRef {
    Void,
    Int,
    Float,
    Bool,
    String_,
    Named(String),
    Array(Box<AsmTypeRef>),
    Generic(String, Vec<AsmTypeRef>),
    RawBlob(Vec<u8>),
}

/// Method reference: `TypeName::method_name` or `[Module]Type::method`.
#[derive(Debug, Clone)]
pub struct AsmMethodRef {
    pub type_name: Option<String>,
    pub method_name: String,
    pub module_name: Option<String>,
}

/// Field reference: `TypeName::field_name` or `[Module]Type::field`.
#[derive(Debug, Clone)]
pub struct AsmFieldRef {
    pub type_name: String,
    pub field_name: String,
    pub module_name: Option<String>,
}

/// Global variable definition: `.global "name" type_ref flags`.
#[derive(Debug, Clone)]
pub struct AsmGlobal {
    pub name: String,
    pub type_ref: AsmTypeRef,
    pub flags: u16,
    pub init_value: Option<Vec<u8>>,
}

/// External function definition: `.extern fn "name" signature "import_name"`.
#[derive(Debug, Clone)]
pub struct AsmExternFn {
    pub name: String,
    pub signature: AsmMethodSig,
    pub import_name: String,
    pub flags: u16,
}

/// Method signature for contract methods and extern functions.
#[derive(Debug, Clone)]
pub struct AsmMethodSig {
    pub params: Vec<AsmTypeRef>,
    pub return_type: AsmTypeRef,
}

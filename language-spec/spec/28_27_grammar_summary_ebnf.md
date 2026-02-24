# 1. Writ Language Specification
## 27. Grammar Summary (EBNF)

A simplified EBNF sketch of the core grammar. Not exhaustive but captures key structural rules.

```ebnf
program        = { using_decl | namespace_decl | declaration } ;
namespace_decl = 'namespace' qualified_name ( ';'
               | '{' { using_decl | namespace_decl | declaration } '}' ) ;
using_decl     = 'using' ( IDENT '=' )? qualified_name ';' ;
qualified_name = IDENT { '::' IDENT } ;
rooted_name    = [ '::' ] qualified_name ;  /* leading :: = root namespace */

visibility     = 'pub' | 'priv' ;
declaration    = { attribute } [ visibility ] ( fn_decl | dlg_decl | struct_decl
               | enum_decl | contract_decl | impl_decl
               | entity_decl | extern_decl
               | const_decl | global_decl ) ;

attribute      = '[' attr_item { ',' attr_item } ']' ;
attr_item      = IDENT [ '(' [ attr_args ] ')' ] ;
attr_args      = attr_arg { ',' attr_arg } ;
attr_arg       = IDENT '=' expr       /* named argument */
               | expr ;               /* positional argument */

fn_decl        = 'fn' IDENT [ generic_params ] '(' [ params ] ')'
                 [ '->' type ] block ;
dlg_decl       = 'dlg' IDENT [ '(' [ params ] ')' ] dlg_block ;

struct_decl    = 'struct' IDENT [ generic_params ] '{'
                 { struct_member } '}' ;
struct_member  = [ visibility ] property | on_decl ;
enum_decl      = 'enum' IDENT [ generic_params ] '{'
                 { variant ',' } '}' ;
variant        = IDENT [ '(' { IDENT ':' type ',' } ')' ] ;

contract_decl  = 'contract' IDENT [ generic_params ] '{'
                 { fn_sig | op_sig } '}' ;
impl_decl      = 'impl' [ contract 'for' ] type '{'
                 { [ visibility ] ( fn_decl | op_decl ) } '}' ;

entity_decl    = 'entity' IDENT '{' { entity_member } '}' ;
entity_member  = [ visibility ] property | use_decl
               | [ visibility ] fn_decl | on_decl ;
property       = IDENT ':' type [ '=' expr ] ',' ;
use_decl       = 'use' IDENT [ '{' { IDENT ':' expr ',' } '}' ] ',' ;
on_decl        = 'on' IDENT [ '(' params ')' ] block ;

/* Components are always extern — see extern_decl */
component_decl = 'component' IDENT '{' { [ visibility ] property } '}' ;

extern_decl    = 'extern' ( fn_sig ';' | struct_decl
               | component_decl ) ;

const_decl     = 'const' IDENT ':' type '=' expr ';' ;
global_decl    = 'global' 'mut' IDENT ':' type '=' expr ';' ;

/* Construction expression */
new_expr       = 'new' rooted_name [ '<' type { ',' type } '>' ]
                 '{' { IDENT ':' expr ',' } '}' ;

/* Lambdas (anonymous functions) */
lambda         = 'fn' '(' [ lambda_params ] ')' [ '->' type ] block ;
lambda_params  = lambda_param { ',' lambda_param } ;
lambda_param   = IDENT [ ':' type ] ;

/* String literals */
string_literal = basic_string | formattable_string
               | raw_string | formattable_raw_string ;
basic_string   = '"' { char | escape } '"' ;
formattable_string = '$"' { char | escape | interpolation } '"' ;
raw_string     = QUOTES_N NEWLINE { raw_char } QUOTES_N ;
                 /* QUOTES_N = 3+ consecutive '"' chars; same count opens and closes */
formattable_raw_string = '$' QUOTES_N NEWLINE { raw_char | interpolation } QUOTES_N ;
interpolation  = '{' expr '}' ;

/* Range expressions */
range_expr     = [ expr ] ( '..' | '..=' ) [ expr ] ;
from_end_index = '^' expr ;    /* only valid inside [] */

/* Array literals */
array_literal  = '[' [ expr { ',' expr } ] ']' ;

/* Variables */
var_decl       = 'let' [ 'mut' ] IDENT [ ':' type ] '=' expr ';' ;

/* Generics */
generic_params = '<' IDENT [ ':' bound ]
                 { ',' IDENT [ ':' bound ] } '>' ;
bound          = IDENT [ '<' type { ',' type } '>' ]
                 { '+' IDENT [ '<' type { ',' type } '>' ] } ;

/* Dialogue blocks */
dlg_block      = '{' { dlg_line } '}' ;
dlg_line       = speaker_line | dlg_escape | transition | text_line ;
speaker_line   = '@' IDENT [ text_content [ '#' IDENT ] ] NEWLINE ;
text_line      = text_content [ '#' IDENT ] NEWLINE ;
dlg_escape     = '$' ( dlg_choice | dlg_if | dlg_match
               | block | statement ) ;
dlg_choice     = 'choice' '{' { STRING [ '#' IDENT ] dlg_block } '}' ;
dlg_if         = 'if' expr dlg_block [ 'else' ( dlg_if | dlg_block ) ] ;
dlg_match      = 'match' expr '{' { pattern '=>' dlg_block } '}' ;
transition     = '->' IDENT ;

/* Patterns (used in match arms and if-let) */
pattern        = literal_pat | wildcard_pat | enum_pat | or_pat
               | range_pat | binding_pat ;
literal_pat    = INT_LIT | STRING_LIT | 'true' | 'false' | 'null' ;
wildcard_pat   = '_' ;
binding_pat    = IDENT ;          /* matches anything, binds to name */
enum_pat       = rooted_name '(' [ pattern { ',' pattern } ] ')' ;
or_pat         = pattern '|' pattern { '|' pattern } ;
range_pat      = INT_LIT '..=' INT_LIT ;

/* Statements */
statement      = var_decl | expr_stmt | for_stmt | while_stmt
               | 'break' ';' | 'continue' ';' | 'return' [ expr ] ';' ;
expr_stmt      = expr ';' ;

/* Blocks and control flow */
block          = '{' { statement | block_expr } [ expr ] '}' ;
block_expr     = if_expr | match_expr ;   /* no trailing ; required */
if_expr        = 'if' expr block [ 'else' ( if_expr | block ) ] ;
match_expr     = 'match' expr '{' { pattern '=>' block } '}' ;
for_stmt       = 'for' IDENT 'in' expr block ;
while_stmt     = 'while' expr block ;

/* Expressions (simplified — see Section 17.1 for full precedence) */
expression     = literal | IDENT | unary_expr | binary_expr | call_expr
               | member_expr | index_expr | if_expr | match_expr
               | lambda | block | range_expr | array_literal
               | new_expr ;
call_expr      = expr [ '<' type { ',' type } '>' ] '(' [ args ] ')' ;
args           = arg { ',' arg } ;
arg            = [ IDENT ':' ] expr ;     /* positional or named */
member_expr    = expr '.' IDENT ;
index_expr     = expr '[' expr ']' ;
unary_expr     = ( '-' | '!' | 'try' ) expr ;
binary_expr    = expr BINARY_OP expr ;    /* see Section 17.1 for operators */

/* Types */
type           = IDENT [ '<' type { ',' type } '>' ] [ '[]' ] [ '?' ] ;
```

---


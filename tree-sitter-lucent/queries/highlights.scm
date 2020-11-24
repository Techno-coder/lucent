; Defines a base highlight for source code.
; The contents of this file is coupled to the
; language server semantic tokens query and
; should not be used on its own.

"module" @keyword
"when" @keyword
"if" @keyword
"fn" @keyword
"let" @keyword
"new" @keyword
"data" @keyword
"static" @keyword
"inline" @keyword
"return" @keyword
"while" @keyword
"load" @keyword
"with" @keyword
"use" @keyword
"as" @keyword

"!" @operator
"#" @operator
"&&" @operator
"||" @operator
"&" @operator
"|" @operator
"^" @operator
"==" @operator
"!=" @operator
"<" @operator
"<=" @operator
">" @operator
">=" @operator
"<<" @operator
">>" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator

"=" @operator
("+" "=") @operator
("-" "=") @operator
("*" "=") @operator
("/" "=") @operator
("%" "=") @operator
("&" "=") @operator
("|" "=") @operator
("^" "=") @operator
("<<" "=") @operator
(">>" "=") @operator

"." @punctuation.delimiter
"," @punctuation.delimiter
":" @punctuation.delimiter
";" @punctuation.delimiter
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket

(string) @string
(integral) @number
(register) @constant
(rune) @constant
"true" @constant
"false" @constant

(data name: (identifier) @type)
(static name: (identifier) @global)
(module name: (identifier) @module)
(load_string name: (identifier) @module)
(function name: (identifier) @function)
(signature name: (identifier) @function)
(variable name: (identifier) @variable)
(let name: (identifier) @variable)

(annotation "@" @attribute
	name: (identifier) @attribute)
(annotation "@!" @attribute
	name: (identifier) @attribute)
(global_annotation "@@" @attribute
	name: (identifier) @attribute)

(root) @keyword
(break) @keyword
(continue) @keyword
(comment) @comment

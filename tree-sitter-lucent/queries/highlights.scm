"module" @keyword
"when" @keyword
"fn" @keyword
"data" @keyword
"static" @keyword
"inline" @keyword
"return" @keyword
"while" @keyword
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

"." @delimiter
"," @delimiter
":" @delimiter
";" @delimiter
"(" @bracket
")" @bracket

(string) @string
(integral) @number
(register) @number
(rune) @number
"true" @number
"false" @number

(annotation "@" @property
	name: (identifier) @property)

(global_annotation "@@" @property
	name: (identifier) @property)

(root) @keyword
(break) @keyword
(identifier) @variable

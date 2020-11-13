# [Prototype] Lucent Language Reference

## Comments
Comment lines are prefixed with `//`:
```
// This is a comment
```

## Identifiers
Identifiers are a sequence of unicode characters.

They cannot: 
* Begin with a digit
* Begin with the prime character: `'`
* Contain punctuation characters other than underscores: `_` and primes: `'`

These are examples of valid identifiers:
```
variable
variable'
a_variable
variable_
_unused
a'123
😄
```

These are not valid identifiers:
```
'prime
0digit
a)b
```

## Variables
```
let identifier: type = value
```
The type can be omitted if it can be inferred:
```
let identifier = value
```
The variable can also be default initialized to zero:
```
let identifier
```

### Shadowing
Variables declared with existing names shadow the previous variables:
```
let a: u32 = 0
let a: u32 = 1
// a == 1
```

## Literals
### Integral
Integrals are composed of a series of digits. The digits can be prefixed to change the base:
* Hexadecimal: `0x`
* Binary: `0b`
* Octal: `0o`

Literals can be separated with primes: `'`.

### Rune
```
'rune'
```
Character or literals are enclosed within a pair of single quotes.

### Registers
```
$register
```
Register literals are prefixed with the `$` character. Registers can be assigned to and read from.

## Primitive types
```
rune
truth
never
void
```
### Integral types
```
u8
u16
u32
u64
i8
i16
i32
i64
```
If the type of an integer is ambiguous then it is assumed to be the smallest signed type that fits the value.

## Pointer type
```
*type
```
Pointer types are constructed by prefixing a type by an asterisk. The size of a pointer is dependent on the contextual architecture. Pointers can be constructed by taking the address of a variable with an ampersand:
```
let variable: i32 = 0
let pointer: *i32 = &variable
```
Addition and subtraction can be performed on pointers with any integer type:
```
let pointer: *i32 = &variable + 1
```
The offset is dependent on the size of the pointed type.

### Dereferencing
```
expression!
```
Pointers are dereferenced by appending an exclamation. This makes
chained dereferencing more ergonomic:
```
a!.b!.c!
```

## Conversions
```
value as type
```
You can omit the type if the target type can be implicitly derived:
```
let variable: type = value as _
```

### Integral 
* Conversions between integral types of the same width will not change the bit representation
* Reductions in width will take the lower (least significant) bits
* Increases in width sign extend the bit representation

### Integral to truth
* `0` is defined as `false`
* Any other number is defined as `true`

### Pointer to pointer
```
pointer as *type
```
Pointers can be casted to any other pointer including function pointers:
```
pointer as convention fn(argument: type) type
```

## Static variables
```
static identifier: type = value
```
Static variables can be default initialized to zero:
```
static identifier: type
```
The type of a static variable can be omitted if the value is specified:
```
static identifier = value
```
Values assigned to a static variable are evaluated at compilation time.

## If expressions
```
if condition: 
	statement.0
	statement.1
	...
```
If expressions can only have a single branch. Branches with a single statement can be collapsed into a single line:
```
if condition: statement.0
```

## When expressions
When expressions are similar to if expressions. Multiple branches are collated under a single expression:
```
when
	condition.0:
		statement.0
		...
	condition.1:
		statement.1
		...
	...
	true:
		...
```

If no branch exists with a `true` condition then the type of the expression is `void`. Otherwise, it is the type of the expression in each branch.

## While loops
```
while condition:
	statement.0
	...
```
Loops with a single statement can be collapsed into a single line:
```
while condition: statement.0
```

## Functions
```
fn identifier(parameter.0: type.0, ...) type.return
	statement.0
	statement.1
	...
	expression
```
Functions can also be defined on one line:
```
fn identifier(...) type.return = expression
```
The return type can be omitted if it is void:
```
fn identifier(...) = expression
```
Calling conventions can also be specified:
```
convention identifier(...) = expression
```
Marking a function as `root` will prevent the function from being removed from the final binary:
```
root identifier(...) = expression
```

### [Removed] Register parameters
```
fn identifier($register.0, $register.1, ...) type.return = ...
```
Registers may also be used as a parameter. The same register cannot appear twice. Calling code will move the parameters into the target registers before invocation.

## Sequence types
Arrays and slices have an element type.

### Arrays
```
let identifier: [type; number] = [value.0, value.1, ...]
```
Fixed size arrays must have the number of elements in the type.

### Slices
```
let identifier: [type;] = array[start:end]
```
Slices are constructed from slicing an array. They can also be created from an address and size:
```
let identifier = [type;] address, size
```

## Compilation time execution
```
let identifier = #expression
```
Prefixing an expression with `#` will evaluate it at compilation time.

## Inline values
```
inline value
```
Inline values are inserted into the function at the inline location.

### Inline byte sequences
```
inline [byte.0, byte.1, ...]
```
The contents of the byte sequence is spliced into the function body at compilation time.

### [Future] Inline nodes
```
inline node
```
The node is directly copied into the function body at the point of inlining.

## Modules
```
module identifier
	...
```
Modules contains functions and global symbols. 

## Structures
```
data identifier
	field.0: type.0
	field.1: type.1
	...
```
Structures are constructed by initialization:
```
new identifier field.0: value.0, field.1: value.1, ...
```
If there are variables in scope with the same name as the field then the assignment can be omitted:
```
let field.0 = value.0
new identifier field.0, ...
```

### [Future] Structure variants
Structures can also have variants:
```
data identifier
	...
	variant.0(type.0a, type.0b, ...)
	variant.1(type.1a, type.1b, ...)
	...
```
These variants can be destructured with a `match` expression.

## Annotations 
```
@annotation.0 parameter
@annotation.1 parameter
item
```
Annotations affect the subsequent item.
Global annotations affect the entire compilation unit and can only be declared in the root file.
```
@@annotation parameter
```
Annotation parameters can be any expression that can be evaluated at compilation time.

### Binary
Binary annotations describe the values that used in the headers of the output binary files.

* Type: `@@type`
* Architecture: `@@architecture`
* Entry point: `@@entry`

### Architecture
```
@architecture identifier
item
```
Architectures specify how intrinsic language structures should be translated including call and control flow instructions. This annotation can only be applied to modules and functions.

### Addresses
Address annotations change the location of a symbol in memory. They can be overridden by annotations in nested items. Annotations on modules offset all the items in the module by the same address.

```
@load address
item
```
The `load` annotation sets where the item is to be loaded into memory. Specifically, it sets the memory address where the loader should copy the binary contents. This also sets the virtual address for the item.

```
@virtual address
item
```
The `virtual` annotation defines what other instructions should treat the address as. For example, call instructions will invoke the virtual address instead of the actual (load) address.

### [Future] Admissions
Admissions are compiler warnings or notes. They can be suppressed by annotating the offending item:
```
@admit identifier
item
```

## Libraries
```
load "path" as identifier
```
Libraries can be imported as a namespaced name. Symbols from the library must be explicitly imported: 
```
load identifier.function as fn function(argument.0: type.0, ...)
```
Addresses from the library can be directly imported:
```
load identifier.address as fn function(argument.0: type.0, ...)
```
Calling conventions can be specified on the function signature:
```
load identifier.function as convention fn(argument.0: type.0, ...)
```
Symbols can also be imported as static variables:
```
load identifier.symbol as variable: type
```
### [Removed] Implicit symbol names
Named symbols can have the import name omitted:
```
load identifier.function as fn(type.0, ...)
```

### Address annotations
```
@load address.0
@virtual address.1
load "path" as identifier
```
Libraries may be relocated with address annotations but if the library type does not support relocation then an admission will be issued.

### [Future] C interoperability
```
load "path" as identifier with "path.h"
```
Symbols can be read automatically from a C header file. 

## File management
```
use "./path.lc"
```
Other source files can be included as modules at the place of usage. The module takes on the name of the included file without the extension. The name can also be manually defined:
```
use "./path.lc" as identifier
```

## Namespace management
```
use path
use path as identifier
use path.*
```
Namespace imports are effective after the position of import and only within the enclosing scope and file. Wildcards are allowed only as the last element in the path.

## Guarantees
### Function pruning
Any function not directly or indirectly called by a root function will be removed from the final binary. More strictly, functions that are not called will not attempt to be translated. This includes library functions (where possible) and functions that are only invoked at compilation time.

### Symbol pruning
Static variables will never be removed from the final binary.

## Verification
### Address overlaps
Addresses and regions are checked at compilation time to ensure no region overlaps with each other.

## Behaviour
### Register allocation
Registers explicitly used will never be used by the register allocator. If a used register conflicts with a required register (such as the parameter of a calling convention) then the contents will be moved into an unused register or otherwise spilled to stack.

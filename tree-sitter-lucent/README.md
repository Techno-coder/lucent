# tree-sitter-lucent
Parser for the Lucent language.

## Development
Lucent uses the `tree-sitter` library to read and parse source code. Generating the parser from the grammar specification requires the `tree-sitter-cli` tool from the `node` ecosystem. It is recommended you use the `yarn` package manager to install `tree-sitter`.

### Installation
1. `yarn`
2. Add to your shell profile if absent: `export PATH=$PATH:./node_modules/.bin`

### Generation
```tree-sitter generate```

### Testing
Run the testing corpus with:
```tree-sitter test```

A specific file can be parsed with:
```tree-sitter parse <file>```

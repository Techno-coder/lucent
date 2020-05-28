#include <tree_sitter/parser.h>
#include <string.h>

#define CONCATENATE(a, b) a ## _ ## b
#define EVALUATE(a, b) CONCATENATE(a, b)
#define PREFIX tree_sitter_lucent_external_scanner
#define FUNCTION(identifier) EVALUATE(PREFIX, identifier)

enum Token {
    OPEN,
    CLOSE,
    LEVEL,
};

struct Scanner {
    uint16_t current;
    uint16_t target;
};

void* FUNCTION(create)() {
    struct Scanner* scanner = malloc(sizeof(struct Scanner));
    scanner->current = 0;
    scanner->target = 0;
    return scanner;
}

void FUNCTION(destroy)(void* object) {
    free((struct Scanner*) object);
}

unsigned FUNCTION(serialize)(void* object, char* buffer) {
    size_t size = sizeof(struct Scanner);
    memcpy(buffer, object, size);
    return size;
}

void FUNCTION(deserialize)(void* object, const char* buffer, unsigned length) {
    memcpy(object, buffer, length);
}

bool step(struct Scanner* scanner, TSLexer* lexer, const bool* valid) {
    if (valid[OPEN] && scanner->current < scanner->target) {
        lexer->result_symbol = OPEN;
        ++scanner->current;
        return true;
    }

    if (valid[CLOSE] && scanner->current > scanner->target) {
        lexer->result_symbol = CLOSE;
        --scanner->current;
        return true;
    }

    return false;
}

bool FUNCTION(scan)(void* object, TSLexer* lexer, const bool* valid) {
    struct Scanner* scanner = object;
    if (step(scanner, lexer, valid))
        return true;

    lexer->mark_end(lexer);
    if (valid[OPEN] || valid[CLOSE] || valid[LEVEL]) {
        bool new_line = lexer->get_column(lexer) == 0;
        uint16_t length = 0;
        while (true) {
            if (lexer->lookahead == '\0') {
                length = 0;
                break;
            } else if (lexer->lookahead == '\n') {
                lexer->advance(lexer, true);
                new_line = true;
                length = 0;
            } else if (lexer->lookahead == '\t') {
                lexer->advance(lexer, true), ++length;
            } else if (lexer->lookahead == ' ') {
                lexer->advance(lexer, true);
            } else if (!new_line) {
                return false;
            } else break;
        }

        if (valid[LEVEL] && scanner->current == length) {
            lexer->result_symbol = LEVEL;
            return true;
        }

        scanner->target = length;
        return step(scanner, lexer, valid);
    }

    return false;
}

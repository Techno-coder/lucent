const PRECEDENCE = {
    call: 11,
    access: 10,
    unary: 9,
    multiplicative: 8,
    additive: 7,
    shift: 6,
    binary_and: 5,
    exclusive_or: 4,
    binary_or: 3,
    compare: 2,
    and: 1,
    or: 0,
}

module.exports = grammar({
    name: 'lucent',

    extras: $ => [
        $._comment,
        /\s/,
    ],

    externals: $ => [
        $._open,
        $._close,
        $._level,
    ],

    word: $ => $._identifier,

    rules: {
        source: $ => repeat(choice($._item,
            field('annotation', $.global_annotation))),

        _item: $ => choice(
            $.module,
            $.function,
            $.static,
            $.load,
            $.data,
            $.use,
        ),

        global_annotation: $ => seq('@@',
            field('name', $.identifier),
            field('value', $._value),
        ),

        annotation: $ => seq('@',
            field('name', $.identifier),
            field('value', $._value),
        ),

        module: $ => seq(annotations($), 'module',
            field('name', $.identifier),
            enclose($, $._item),
        ),

        load: $ => seq(annotations($), 'load', field('load',
            choice($.load_string, $.load_symbol)), '\n'),

        load_string: $ => seq(
            field('module', $.string),
            'as', field('name', $.identifier),
            optional(seq('with', field('with', $.string))),
        ),

        _load_path: $ => repeat1(seq($.identifier, '.')),
        load_symbol: $ => seq(
            field('module', alias($._load_path, $.path)),
            field('target', choice($.identifier, $.integral)),
            'as', field('as', choice(
                alias($.variable, $.static),
                $.signature,
            )),
        ),

        use: $ => seq('use', choice($._use_string, $._use_path)),
        _use_string: $ => seq(field('path', $.string),
            optional(seq('as', field('name', $.identifier)))),

        _use_path: $ => seq($.identifier,
            repeat(seq('.', $.identifier)),
            optional(seq('.', alias('*', $.wildcard))),
        ),

        data: $ => seq(
            'data', field('name', $.identifier),
            enclose($, $.variable),
        ),

        'function': $ => seq(annotations($),
            optional(field('root',
                alias('root', $.root))),
            signature($, true),
            choice(field('value', $.block),
                seq('=', field('value', $._value))),
        ),

        variable: $ => seq(
            field('name', $.identifier),
            ':', field('type', $._type)
        ),

        'static': $ => seq(annotations($),
            'static', field('name', $.identifier),
            optional(seq(':', field('type', $._type))),
            optional(seq('=', field('value', $._value))),
        ),

        _type: $ => choice(
            $.signature_type,
            $.array_type,
            $.slice_type,
            $.pointer,
            $.path,
        ),

        signature: $ => signature($, true),
        signature_type: $ => prec.right(signature($, false)),

        pointer: $ => seq('*', field('type', $._type)),
        slice_type: $ => seq('[', field('type', $._type), ';]'),
        array_type: $ => seq(
            '[', field('type', $._type),
            ';', field('size', $._value), ']',
        ),

        block: $ => enclose($, $._statement),

        _statement: $ => prec.right(choice(
            alias('break', $.break),
            alias('continue', $.continue),
            $.compound,
            $.return,
            $.while,
            $.let,
            $.set,
            $._value,
        )),

        'let': $ => prec.right(seq(
            'let', field('name', $.identifier),
            optional(seq(':', field('type', $._type))),
            optional(seq('=', field('value', $._value))),
        )),

        set: $ => prec.right(seq(
            field('target', $._value), '=',
            field('value', $._value),
        )),

        compound: $ => prec.right(seq(
            field('target', $._value),
            field('operator', choice(
                choice('+', '-', '*', '/', '%'),
                choice('&', '|', '^', '<<', '>>'),
            )), '=', field('value', $._value),
        )),

        'return': $ => prec.right(seq('return',
            optional(field('value', $._value)))),

        'while': $ => seq('while',
            field('condition', $._value),
            ':', field('value', $._statement),
        ),

        _value: $ => prec.right(choice(
            $.register,
            $.integral,
            $.string,
            $.rune,
            $.truth,
            $.unary,
            $.binary,
            $.dereference,
            $.cast,
            $.call,
            $.array,
            $.index,
            $.slice,
            $.access,
            $.group,
            $.block,
            $.when,
            $.path,
            $.new,
        )),

        when: $ => choice(
            seq('when', $._open, repeat1($.branch), $._close),
            seq('if', $.branch),
        ),

        branch: $ => seq(
            field('condition', $._value),
            ':', field('branch', $._statement),
        ),

        binary: $ => {
            const TABLE = [
                [PRECEDENCE.and, '&&'],
                [PRECEDENCE.or, '||'],
                [PRECEDENCE.binary_and, '&'],
                [PRECEDENCE.binary_or, '|'],
                [PRECEDENCE.exclusive_or, '^'],
                [PRECEDENCE.compare, choice('==', '!=', '<', '<=', '>', '>=')],
                [PRECEDENCE.shift, choice('<<', '>>')],
                [PRECEDENCE.additive, choice('+', '-')],
                [PRECEDENCE.multiplicative, choice('*', '/', '%')],
            ];

            return choice(...TABLE.map(([precedence, operator]) =>
                prec.left(precedence, seq(field('left', $._value),
                    field('operator', operator), field('right', $._value)))));
        },

        unary: $ => {
            const TABLE = ['-', '!', '&', '#', 'inline'];
            return prec(PRECEDENCE.unary, seq(
                field('operator', choice(...TABLE)),
                field('value', $._value),
            ));
        },

        dereference: $ => prec(PRECEDENCE.access,
            seq(field('value', $._value), '!')),

        call: $ => seq(
            field('function', choice($.path, $._value)),
            field('arguments', $.arguments),
        ),

        arguments: $ => seq('(',
            separated(',', $._value), ')'),

        cast: $ => prec.right(seq(
            field('value', $._value),
            'as', field('type', $._type),
        )),

        index: $ => seq(
            field('value', $._value),
            '[', field('index', $._value), ']',
        ),

        slice: $ => seq(
            field('value', $._value),
            '[', field('left', optional($._value)),
            ':', field('right', optional($._value)), ']',
        ),

        access: $ => prec(PRECEDENCE.access, seq(
            field('value', $._value), '.',
            field('name', $.identifier),
        )),

        new: $ => prec.right(seq('new',
            field('type', choice($.path, $.slice_type)),
            separated(',', $.field),
        )),

        field: $ => prec.right(seq(
            field('name', $.identifier),
            optional(seq(':', field('value', $._value))),
        )),

        group: $ => seq('(', field('value', $._value), ')'),
        array: $ => seq('[', separated(',', $._value), ']'),
        path: $ => prec.right(seq($.identifier,
            repeat(seq('.', $.identifier)))),

        integral: _ => choice(
            /0b[0-1]([0-1']*[0-1])?/,
            /0o[0-7]([0-7']*[0-7])?/,
            /0x[\da-f]([\da-f']*[\da-f])?/,
            /-?\d([\d']*\d)?/,
        ),

        truth: _ => choice('true', 'false'),
        register: $ => seq('$', $._identifier),
        string: _ => /"(\\.|[^"])*"/,
        rune: _ => /'(\\.|[^"])'/,

        identifier: $ => $._identifier,
        _identifier: _ => /[^\x00-@\[-^`{-~][^\x00-&(-/:-@\[-^`{-~]*/,
        _comment: _ => token(seq('//', /[^\n]*/)),
    }
});

function annotations($) {
    return repeat($.annotation);
}

function signature($, named) {
    return seq(
        field('convention', optional($.identifier)),
        'fn', (named ? field('name', $.identifier) : seq()),
        seq('(', separated(',', $.variable), ')'),
        field('return', optional($._type)),
    );
}

function enclose($, rule) {
    return seq($._open, rule, repeat(seq($._level, rule)), $._close);
}

function separated1(separator, rule) {
    return seq(rule, repeat(seq(separator, rule)), optional(separator));
}

function separated(separator, rule) {
    return optional(separated1(separator, rule));
}

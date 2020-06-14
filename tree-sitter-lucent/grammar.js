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
        source: $ => repeat(choice(
            field('annotation', $.global_annotation),
            field('item', $._item),
        )),

        _item: $ => choice(
            $.module,
            $.function,
            $.static,
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
            field('identifier', $.identifier),
            enclose($, field('item', $._item)),
        ),

        use: $ => seq(annotations($), 'use',
            choice($._use_string, $._use_identifier), '\n'),

        _use_string: $ => seq(
            field('path', $.string),
            optional(seq('with', field('with', $.string))),
            optional(seq('as', field('as', $.identifier))),
        ),

        _use_identifier: $ => seq(
            field('path', alias($._use_path, $.path)),
            optional(seq('as', field('as', choice(
                alias($.parameter, $.static),
                $.identifier,
                $.signature,
            )))),
        ),

        _use_path: $ => seq(
            $.identifier, repeat(seq('.', $.identifier)),
            optional(seq('.', choice(alias('*', $.wild), $.integral))),
        ),

        signature: $ => prec.right(seq(
            field('convention', optional($.identifier)),
            'fn', field('identifier', $.identifier),
            seq('(', separated(',', field('parameter', $._type)), ')'),
            field('return', optional($._type)),
        )),

        data: $ => seq(
            'data', field('identifier', $.identifier),
            enclose($, field('field', alias($.parameter, $.field))),
        ),

        'function': $ => seq(annotations($),
            field('root', optional(alias('root', $.root))),
            field('convention', optional($.identifier)),
            'fn', field('identifier', $.identifier),
            seq('(', separated(',', field('parameter',
                choice($.parameter, $.register))), ')'),
            field('return', optional($._root_type)),
            choice(field('block', $.block),
                seq('=', field('block', $._line_block))),
        ),

        parameter: $ => seq(
            field('identifier', $.identifier),
            ':', field('type', $._type)
        ),

        'static': $ => seq(annotations($),
            'static', field('identifier', $.identifier),
            choice(
                seq(':', field('type', $._type)),
                seq('=', field('value', $._value)),
                seq(
                    ':', field('type', $._type),
                    '=', field('value', $._value),
                ),
            ),
        ),

        _line_block: $ => alias($._value, $.block),
        block: $ => enclose($, $._statement),

        _root_type: $ => choice(
            $.register,
            $._type,
        ),

        _type: $ => choice(
            $.array_type,
            $.slice_type,
            $.pointer,
            $.path,
        ),

        pointer: $ => seq('*', $._type),
        slice_type: $ => seq('[', field('type', $._type), ';]'),
        array_type: $ => seq(
            '[', field('type', $._type),
            ';', field('size', $._value), ']',
        ),

        _statement: $ => choice(
            alias('break', $.break),
            $._expression,
            $.compound,
            $.return,
            $.while,
            $.let,
            $.set,
        ),

        _expression: $ => choice(
            $._value,
            $.block,
            $.when,
        ),

        'let': $ => seq(
            'let', field('identifier', $.identifier),
            optional(seq(':', field('type', $._type))),
            optional(seq('=', field('value', $._expression))),
        ),

        set: $ => seq(
            field('target', $._value), '=',
            field('value', $._expression),
        ),

        compound: $ => seq(
            field('target', $._value),
            field('operator', choice(
                choice('+', '-', '*', '/', '%'),
                choice('&', '|', '^', '<<', '>>'),
            )), '=', field('value', $._expression),
        ),

        'return': $ => seq('return',
            field('value', optional($._expression))),

        when: $ => seq('when', choice(
            seq(':', $._open, repeat1($.branch), $._close),
            $.branch,
        )),

        branch: $ => seq(
            field('condition', $._value),
            ':', field('branch', choice($.block, $._line_block)),
        ),

        'while': $ => seq('while',
            field('condition', $._value),
            ':', field('block', choice($.block, $._line_block)),
        ),

        _value: $ => prec.right(choice(
            $.register,
            $.integral,
            $.string,
            $.rune,
            $.truth,
            $.unary,
            $.binary,
            $.cast,
            $.call,
            $.array,
            $.index,
            $.slice,
            $.access,
            $.create,
            $.group,
            $.path,
        )),

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
            const TABLE = ['-', '!', '*', '&', '#', 'inline'];
            return prec(PRECEDENCE.unary, seq(
                field('operator', choice(...TABLE)),
                field('value', $._value),
            ));
        },

        call: $ => prec(PRECEDENCE.call, seq(
            field('function', $.path),
            seq('(', separated(',',
                field('argument', $._value)
            ), ')')
        )),

        cast: $ => seq(
            field('value', $._value),
            'as', field('type', $._type),
        ),

        index: $ => prec(PRECEDENCE.call, seq(
            field('value', $._value),
            '[', field('index', $._value), ']',
        )),

        slice: $ => prec(PRECEDENCE.call, seq(
            field('value', $._value),
            '[', field('left', optional($._value)),
            ':', field('right', optional($._value)), ']',
        )),

        access: $ => prec(PRECEDENCE.access, seq(
            field('value', $._value), '.',
            field('field', $.identifier),
        )),

        create: $ => prec.right(seq(
            field('path', $.path), '~',
            separated(',', prec.right(field('field',
                choice($.identifier, $.field)))),
        )),

        field: $ => prec.right(seq(
            field('name', $.identifier), '=',
            field('value', $._value),
        )),

        group: $ => seq('(', $._value, ')'),
        array: $ => seq('[', separated(',', $._value), ']'),
        path: $ => prec.right(seq($.identifier,
            repeat(seq('.', $.identifier)))),

        integral: $ => choice(
            /0b[0-1]([0-1']*[0-1])?/,
            /0o[0-7]([0-7']*[0-7])?/,
            /0x[\da-f]([\da-f']*[\da-f])?/,
            /\d([\d']*\d)?/,
        ),

        truth: $ => choice('true', 'false'),
        register: $ => seq('$', $._identifier),
        string: $ => /"(\\.|[^"])*"/,
        rune: $ => /'(\\.|[^"])'/,

        identifier: $ => $._identifier,
        _identifier: $ => /[^\x00-@\[-^`{-~][^\x00-&(-/:-@\[-^`{-~]*/,
        _comment: $ => token(seq('//', /[^\n]*/)),
    }
});

function annotations($) {
    return field('annotation', repeat($.annotation));
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

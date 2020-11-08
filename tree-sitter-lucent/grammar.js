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

        load: $ => seq(annotations($), 'load',
            choice($._load_string, $._load_symbol), '\n'),

        _load_string: $ => seq(
            field('module', $.string),
            choice(
                seq('as', field('name', $.identifier)),
                seq('with', field('with', $.string),
                    optional(seq('as', field('name', $.identifier))))
            )
        ),

        _load_symbol: $ => seq(
            field('module', $.identifier),
            '.', field('target', choice($.identifier, $.integral)),
            'as', field('as', choice(
                alias($.parameter, $.static),
                $.signature,
            )),
        ),

        signature: $ => seq(
            field('convention', optional($.identifier)),
            'fn', field('name', $.identifier),
            seq('(', separated(',', field('parameter', $._type)), ')'),
            field('return', optional($._type)),
        ),

        use: $ => seq(annotations($), 'use',
            field('use', choice($._use_string, $._use_path))),

        _use_string: $ => seq(
            field('path', $.string),
            optional(seq('as', $.identifier)),
        ),

        _use_path: $ => seq($.identifier,
            repeat(seq('.', $.identifier)),
            optional(seq('.', alias('*', $.wild)))
        ),

        data: $ => seq(
            'data', field('name', $.identifier),
            enclose($, field('field', alias($.parameter, $.field))),
        ),

        'function': $ => seq(annotations($),
            field('root', optional(alias('root', $.root))),
            field('convention', optional($.identifier)),
            'fn', field('name', $.identifier),
            seq('(', separated(',', field('parameter',
                choice($.parameter, $.register))), ')'),
            field('return', optional($._return_type)),
            choice(field('block', $.block),
                seq('=', field('block', $._value))),
        ),

        parameter: $ => seq(
            field('name', $.identifier),
            ':', field('type', $._type)
        ),

        'static': $ => seq(annotations($),
            'static', field('name', $.identifier),
            choice(
                seq(':', field('type', $._type)),
                seq('=', field('value', $._value)),
                seq(
                    ':', field('type', $._type),
                    '=', field('value', $._value),
                ),
            ),
        ),

        _return_type: $ => choice(
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

        block: $ => enclose($, $._statement),

        _statement: $ => prec.right(choice(
            alias('break', $.break),
            alias('continue', $.continue),
            $.compound,
            $.return,
            $.while,
            $.let,
            $.set,
            $._value
            ,
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
            field('target', $._value)
            ,
            field('operator', choice(
                choice('+', '-', '*', '/', '%'),
                choice('&', '|', '^', '<<', '>>'),
            )), '=', field('value', $._value),
        )),

        'return': $ => prec.right(seq('return',
            optional(field('value', $._value)))),

        'while': $ => seq('while',
            field('condition', $._value),
            ':', field('block', $._statement),
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
            $._group,
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
            seq($._value, '!')),

        call: $ => seq(
            field('function', $.path),
            seq('(', separated(',',
                field('argument', $._value)
            ), ')')
        ),

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
            field('field', $.identifier),
        )),

        new: $ => prec.right(seq('new',
            field('path', choice($.path, $.slice_type)),
            separated(',', prec.right(field('field',
                choice($.identifier, $.field)))),
        )),

        field: $ => prec.right(seq(
            field('name', $.identifier), ':',
            field('value', $._value),
        )),

        _group: $ => seq('(', $._value, ')'),
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

"""Render validation failures in the API's established human-readable format."""

from pydantic import ValidationError


def validation_message(error, *, title=None):
    if not isinstance(error, ValidationError):
        return str(error)
    lines = []
    for detail in error.errors(include_url=False):
        kind = detail['type']
        ctx = detail.get('ctx', {})
        message = detail['msg']
        code = 'value_error'
        if kind == 'value_error':
            message = str(ctx.get('error', message.removeprefix('Value error, ')))
        elif kind == 'missing':
            message, code = 'field required', 'value_error.missing'
        elif kind == 'extra_forbidden':
            message, code = 'extra fields not permitted', 'value_error.extra'
        elif kind == 'literal_error':
            values = ctx['expected'].replace(' or ', ', ')
            message = f"unexpected value; permitted: {values}"
            permitted = f"[{values}]" if ", " not in values else f"({values})"
            code = f"value_error.const; given={detail.get('input')}; permitted={permitted}"
        elif kind == 'email':
            code = 'value_error.email'
        elif kind == 'model_type':
            message, code = 'value is not a valid dict', 'type_error.dict'
        elif kind.startswith('datetime_'):
            if kind == 'datetime_type':
                message, code = 'invalid type; expected datetime, string, bytes, int or float', 'type_error'
            else:
                message, code = 'invalid datetime format', 'value_error.datetime'
        elif kind == 'enum':
            values = ctx['expected'].replace(' or ', ', ')
            message = f'value is not a valid enumeration member; permitted: {values}'
            from pydatalab.models import utils
            from pydatalab.models import people, relationships, cells, versions, tasks
            enum_type = None
            expected = values.replace(', and ', ', ').replace(' and ', ', ')
            for module in (utils, people, relationships, cells, versions, tasks):
                from enum import Enum
                for candidate in vars(module).values():
                    if isinstance(candidate, type) and issubclass(candidate, Enum):
                        if ', '.join(repr(e.value) for e in candidate) == expected:
                            enum_type = candidate
                            break
                if enum_type:
                    break
            code = 'type_error.enum' + (f'; enum_values={list(enum_type)!r}' if enum_type else '')
        elif kind in ('int_parsing', 'int_type', 'int_from_float'):
            message, code = 'value is not a valid integer', 'type_error.integer'
        elif kind in ('float_parsing', 'float_type'):
            message, code = 'value is not a valid float', 'type_error.float'
        elif kind in ('bool_parsing', 'bool_type'):
            message, code = 'value could not be parsed to a boolean', 'type_error.bool'
        elif kind == 'string_type':
            message, code = 'str type expected', 'type_error.str'
        elif kind == 'identifier_pattern':
            code = f'value_error.str.regex; pattern={ctx["pattern"]}'
        elif kind in ('string_too_short', 'string_too_long'):
            short = kind.endswith('short')
            limit = ctx['min_length' if short else 'max_length']
            message = f'ensure this value has at {"least" if short else "most"} {limit} characters'
            code = f'value_error.any_str.{"min" if short else "max"}_length; limit_value={limit}'
        elif kind in ('greater_than_equal', 'greater_than', 'less_than_equal', 'less_than'):
            name, word, bound = {'greater_than_equal': ('not_ge', 'greater than or equal to', 'ge'),
                                'greater_than': ('not_gt', 'greater than', 'gt'),
                                'less_than_equal': ('not_le', 'less than or equal to', 'le'),
                                'less_than': ('not_lt', 'less than', 'lt')}[kind]
            message = f'ensure this value is {word} {ctx[bound]}'
            code = f'value_error.number.{name}; limit_value={ctx[bound]}'
        elif kind in ('list_type', 'dict_type'):
            name = kind.split('_')[0]
            message, code = f'value is not a valid {name}', f'type_error.{name}'
        if detail.get('input', object()) is None and kind not in ('missing', 'value_error'):
            message, code = 'none is not an allowed value', 'type_error.none.not_allowed'
        loc = detail['loc'] or ('__root__',)
        # Pydantic 2 includes union branch names; the API historically omitted them.
        loc = tuple(part for part in loc if not (isinstance(part, str) and
                    (part.startswith('function-') or part in ('EntryReference', 'InlineSubstance'))))
        lines.append(' -> '.join(map(str, loc)) + f'\n  {message} (type={code})')
    count = len(lines)
    return f'{count} validation error{"s" if count != 1 else ""} for {title or error.title}\n' + '\n'.join(lines)


def validation_errors(error):
    """Structured validation details exposed by version request endpoints."""
    result = []
    for detail in error.errors(include_url=False, include_input=False):
        kind = detail['type']
        ctx = detail.get('ctx', {})
        if kind == 'missing':
            detail.update(msg='field required', type='value_error.missing')
        elif kind == 'extra_forbidden':
            detail.update(msg='extra fields not permitted', type='value_error.extra')
        elif kind == 'value_error':
            detail.update(msg=str(ctx['error']), type='value_error')
            detail.pop('ctx', None)
        result.append(detail)
    return result

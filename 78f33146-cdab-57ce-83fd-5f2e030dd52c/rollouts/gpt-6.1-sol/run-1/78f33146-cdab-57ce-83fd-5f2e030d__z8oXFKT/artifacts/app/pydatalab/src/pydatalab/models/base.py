"""Common validation and JSON conventions for datalab documents."""

from datetime import date, datetime, time
from enum import Enum
from pathlib import Path

from bson import ObjectId
from pint import Quantity
from pydantic import BaseModel, ConfigDict, model_serializer, model_validator


def json_values(value):
    """Encode database values using the API's established wire formats."""
    if isinstance(value, (datetime, date, time)):
        return value.isoformat()
    if isinstance(value, (ObjectId, Path, Quantity)):
        return str(value)
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, dict):
        return {str(k): json_values(v) for k, v in value.items()}
    if isinstance(value, (tuple, list, set, frozenset)):
        return [json_values(v) for v in value]
    return value


class DataLabModel(BaseModel):
    """Document models share coercion rules and lossless datetime JSON encoding."""

    model_config = ConfigDict(coerce_numbers_to_str=True, ser_json_inf_nan="constants")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        from pydantic_core import core_schema
        schema = handler(source)

        def coerce_string(value):
            return str(value) if isinstance(value, bool) else value

        def coerce_integer(value):
            if isinstance(value, float) and value == value and abs(value) != float('inf'):
                return int(value)
            return value

        def coerce_mapping(value):
            if not isinstance(value, dict):
                try:
                    return dict(value)
                except (TypeError, ValueError):
                    pass
            return value

        # Apply the document coercions to scalar/container schemas while retaining
        # Pydantic validation, field constraints, and nested model schemas.
        def visit(node):
            if isinstance(node, list):
                return [visit(v) for v in node]
            if not isinstance(node, dict):
                return node
            # Nested model schemas already carry their own validation conventions.
            if node.get('type') == 'model' and node.get('cls') is not cls:
                return node
            node = {k: visit(v) if k not in ('metadata', 'serialization', 'config') else v for k, v in node.items()}
            kind = node.get('type')
            if not isinstance(kind, str):
                return node
            if kind == 'union':
                node['mode'] = 'left_to_right'
            coerce = {'str': coerce_string, 'int': coerce_integer,
                      'dict': coerce_mapping,
                      'datetime': lambda v: int(v) if isinstance(v, bool) else v}.get(kind)
            if coerce and not node.get('strict'):
                return core_schema.no_info_before_validator_function(coerce, node)
            return node

        return visit(schema)

    @model_validator(mode='before')
    @classmethod
    def read_model_values(cls, value):
        if isinstance(value, BaseModel) and not isinstance(value, cls):
            return dict(value)
        return value

    @model_serializer(mode='wrap')
    def serialize_document(self, handler, info):
        if info.mode == 'python':
            return handler(self)
        # Python mode retains datetime offsets and BSON types, including in freeform metadata.
        data = self.model_dump(mode='python', include=info.include, exclude=info.exclude,
                               by_alias=info.by_alias, exclude_unset=info.exclude_unset,
                               exclude_defaults=info.exclude_defaults, exclude_none=info.exclude_none)
        return json_values(data)


# URL validation retains the supplied spelling, as used in deployment metadata.
from typing import Annotated
from pydantic import AnyUrl, PlainSerializer, WrapValidator


def validate_url(value, handler):
    handler(value)
    return str(value).strip()


MetadataUrl = Annotated[AnyUrl, WrapValidator(validate_url), PlainSerializer(str)]

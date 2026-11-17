from collections.abc import Callable
from enum import Enum
from functools import partial
from typing import TypeAlias
import datetime
import random
import string

from bson.objectid import ObjectId
from pydantic import ConfigDict
from pydantic import Field, field_validator, model_validator
import pint

from pydatalab.models.base import DataLabModel as BaseModel
from pydatalab.validation import validation_message


class ItemType(str, Enum):
    """An enumeration of the types of items known by this implementation, should be made dynamic in the future."""

    SAMPLES = "samples"
    STARTING_MATERIALS = "starting_materials"


class ItemStatus(str, Enum):
    """An enumeration of the status of items"""

    ACTIVE = "active"
    PLANNED = "planned"
    DISPOSED = "disposed"
    COMPLETED = "completed"
    FAILED = "failed"
    OTHER = "other"


SampleStatus = ItemStatus
"""An enumeration of the status of samples"""


class CellStatus(str, Enum):
    """An enumeration of the status of cells"""

    ACTIVE = "active"
    PLANNED = "planned"
    DISPOSED = "disposed"
    CYCLED = "cycled"
    SHORTED = "shorted"
    DISMANTLED = "dismantled"
    OTHER = "other"


class EquipmentStatus(str, Enum):
    """An enumeration of the status of equipments"""

    WORKING = "working"
    BROKEN = "broken"
    BEING_FIXED = "being_fixed"
    DEFUNCT = "defunct"
    NOT_BEING_FIXED = "not_being_fixed"
    OTHER = "other"


class StartingMaterialsStatus(str, Enum):
    """An enumeration of the status of starting materials"""

    ORDERED = "ordered"
    DISPOSED = "disposed"
    PLANNED = "planned"
    AVAILABLE = "available"
    UNAVAILABLE = "unavailable"
    EXHAUSTED = "exhausted"
    OTHER = "other"


class KnownType(str, Enum):
    """An enumeration of the types of entry known by this implementation, should be made dynamic in the future."""

    SAMPLES = "samples"
    STARTING_MATERIALS = "starting_materials"
    BLOCKS = "blocks"
    FILES = "files"
    PEOPLE = "people"
    COLLECTIONS = "collections"


IDENTIFIER_REGEX = r"^(?:[a-zA-Z0-9]+|[a-zA-Z0-9][a-zA-Z0-9._-]+[a-zA-Z0-9])$"
"""A regex that matches identifiers that are url-safe and do not contain
leading or trailing punctuation.
"""


class HumanReadableIdentifier(str):
    """A whitespace-trimmed, URL-safe human-readable identifier."""
    regex = IDENTIFIER_REGEX

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        from pydantic_core import core_schema
        return core_schema.no_info_after_validator_function(
            cls.check_identifier,
            core_schema.str_schema(coerce_numbers_to_str=True, strip_whitespace=True,
                                   min_length=1, max_length=40))

    @classmethod
    def check_identifier(cls, value):
        import re
        from pydantic_core import PydanticCustomError
        if not re.match(cls.regex, value):
            raise PydanticCustomError('identifier_pattern', 'string does not match regex "{pattern}"',
                                      {'pattern': cls.regex})
        return value

    def __new__(cls, value):
        from pydantic import TypeAdapter
        return str.__new__(cls, TypeAdapter(cls).validate_python(value))

    @property
    def value(self):
        return str(self)


class Refcode(HumanReadableIdentifier):
    regex = r"^[a-z]{2,10}:" + IDENTIFIER_REGEX[1:]

    @property
    def prefix(self):
        return self.split(":")[0]

    @property
    def identifier(self):
        return self.split(":")[1]


class UserRole(str, Enum):
    USER = "user"
    ADMIN = "admin"
    MANAGER = "manager"


class PintType(str):
    """A WIP attempt to create a custom pydantic field type for Pint quantities.
    The idea would eventually be to use TypeAlias to create physical/dimensionful pydantic fields.

    """

    Q = pint.Quantity

    def __init__(self, dimensions: str):
        self._dimensions = dimensions

    @classmethod
    def __get_pydantic_core_schema__(self, source, handler):
        from pydantic_core import core_schema
        return core_schema.no_info_plain_validator_function(self.validate, json_schema_input_schema=core_schema.str_schema())

    @classmethod
    def validate(self, v):
        q = self.Q(v)
        if not q.check(self._dimensions):
            raise ValueError("Value {v} must have dimensions of mass, not {v.dimensions}")
        return q

    @classmethod
    def __get_pydantic_json_schema__(cls, schema, handler):
        return {"type": "string"}


Mass: TypeAlias = PintType("[mass]")  # type: ignore # noqa
Volume: TypeAlias = PintType("[volume]")  # type: ignore # noqa


class PyObjectId(ObjectId):
    """A wrapper class for a BSON ObjectId that can be used as a Pydantic field type.

    Modified from "Getting started iwth MongoDB and FastAPI":
    https://www.mongodb.com/developer/languages/python/python-quickstart-fastapi/.

    """

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        from pydantic_core import core_schema
        return core_schema.no_info_plain_validator_function(cls.validate, json_schema_input_schema=core_schema.str_schema(), serialization=core_schema.to_string_ser_schema(when_used="json"))

    @classmethod
    def validate(cls, v):
        if isinstance(v, dict) and "$oid" in v:
            v = v["$oid"]

        if not ObjectId.is_valid(v):
            raise ValueError("Invalid ObjectId")

        return ObjectId(v)

    @classmethod
    def __get_pydantic_json_schema__(cls, schema, handler):
        return {"type": "string"}


class IsoformatDateTime(datetime.datetime):
    """A datetime container that is more flexible than the pydantic default."""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        from pydantic_core import core_schema
        return core_schema.no_info_plain_validator_function(cls.validate, json_schema_input_schema=core_schema.str_schema(), serialization=core_schema.plain_serializer_function_ser_schema(lambda v: v.isoformat() if v is not None else None, when_used="json"))

    @classmethod
    def validate(cls, v) -> datetime.datetime | None:
        """Cast isoformat strings to datetimes and enforce UTC if tzinfo is missing."""
        if isinstance(v, str):
            if v in ["0", " "]:
                return None
            v = datetime.datetime.fromisoformat(v)

        if v.tzinfo is None:
            v = v.replace(tzinfo=datetime.timezone.utc)

        return v


JSON_ENCODERS = {
    pint.Quantity: str,
    ObjectId: str,
}


class RefCodeFactory:
    refcode_generator: Callable

    @classmethod
    def generate(self):
        from pydatalab.config import CONFIG

        return f"{CONFIG.IDENTIFIER_PREFIX}:{self.refcode_generator()}"


def random_uppercase(length: int = 6):
    return "".join(random.choices(string.ascii_uppercase, k=length))  # noqa: S311


class RandomAlphabeticalRefcodeFactory(RefCodeFactory):
    refcode_generator = partial(random_uppercase, length=6)


def generate_unique_refcode():
    """Generates a unique refcode for an item using the configured convention."""
    from pydatalab.config import CONFIG
    from pydatalab.mongo import get_database

    refcode = f"{CONFIG.REFCODE_GENERATOR.generate()}"
    try:
        while get_database().items.find_one({"refcode": refcode}):
            refcode = f"{CONFIG.IDENTIFIER_PREFIX}:{CONFIG.REFCODE_GENERATOR.generate()}"
    except Exception as exc:
        raise RuntimeError(f"Cannot check refcode for uniqueness: {validation_message(exc)}")

    return refcode


class InlineSubstance(BaseModel):
    name: str
    chemform: str | None = None


class EntryReference(BaseModel):
    """A reference to a database entry by ID and type.

    Can include additional arbitarary metadata useful for
    inlining the item data.

    """

    type: str
    name: str | None = None
    immutable_id: PyObjectId | None = None
    item_id: HumanReadableIdentifier | None = None
    refcode: Refcode | None = None
    chemform: str | None = None

    @model_validator(mode="after")
    def check_id_fields(self):
        values = self.__dict__
        """Check that at least one of the possible identifier fields is provided."""
        id_fields = ("immutable_id", "item_id", "refcode")

        if all(values.get(f) is None for f in id_fields):
            raise ValueError(f"Must provide at least one of {id_fields!r}")

        return self

    model_config = ConfigDict(extra='allow')


class Constituent(BaseModel):
    """A constituent of a sample."""

    item: EntryReference | InlineSubstance = Field(..., validate_default=True, union_mode="left_to_right")
    """A reference to item (sample or starting material) entry for the constituent substance."""

    quantity: float | None = Field(..., ge=0)
    """The amount of the constituent material used to create the sample."""

    unit: str = Field('g')
    """The unit symbol for the value provided in `quantity`, default is mass
    in grams (g) but could also refer to volumes (mL, L, etc.) or moles (mol).
    """

    @field_validator("item")
    def check_itemhood(cls, v):
        """Check that the reference within the constituent is to an item type."""
        if "type" in (v.value for v in ItemType):
            raise ValueError(f"`type` must be one of {ItemType!r}")

        return v

    @field_validator("item", mode="before")
    def coerce_reference(cls, v):
        if isinstance(v, BaseModel):
            v = dict(v)
        if isinstance(v, dict):
            refcode = v.pop("refcode", None)
            item_id = v.pop("item_id", None)

            if refcode:
                return EntryReference(refcode=refcode, item_id=item_id, **v)
            elif item_id:
                return EntryReference(item_id=item_id, **v)
            else:
                name = v.pop("name", "")
                chemform = v.pop("chemform", None)
                if not name:
                    raise ValueError("Inline substance must have a name!")
                return InlineSubstance(name=name, chemform=chemform)
        return v

from typing import Literal

from pydantic import Field, model_validator

from pydatalab.models.entries import Entry
from pydatalab.models.traits import HasBlocks, HasOwner
from pydatalab.models.utils import HumanReadableIdentifier


class Collection(Entry, HasOwner, HasBlocks):
    """A model for representing a group of related items, for example all the samples
    belonging to one batch or project.

    """

    type: Literal['collections'] = Field('collections', json_schema_extra={'pattern': '^collections$'})

    collection_id: HumanReadableIdentifier | None = Field(None)
    """A short human-readable/usable name for the collection."""

    title: str | None = None
    """A descriptive title for the collection."""

    description: str | None = None
    """A description of the collection, either in plain-text or a markup language."""

    num_items: int | None = Field(None)
    """Inlined number of items associated with this collection."""

    @model_validator(mode="after")
    def check_ids(self):
        values = self.__dict__
        if not any(values.get(k) is not None for k in ("collection_id", "immutable_id")):
            raise ValueError("Collection must have at least collection_id or immutable_id")

        return self

from collections.abc import Mapping
from datetime import datetime, date
from dataclasses import is_dataclass, asdict
from email.utils import format_datetime
from datetime import timezone
import logging
from json.encoder import JSONEncoder
from uuid import UUID
from pydantic import BaseModel
from cassandra.util import OrderedMapSerializedKey
LOGGER = logging.getLogger(__name__)

class ArgusJSONEncoder(JSONEncoder):

    def default(self, o):
        match o:
            case UUID():
                return str(o)
            case BaseModel():
                return o.model_dump()
            case datetime():
                return o.strftime('%Y-%m-%dT%H:%M:%S.%fZ')[:-4] + 'Z'
            case bytes():
                return o.decode('utf-8', errors='replace')
            case _:
                return super().default(o)

class ArgusJSONProvider:

    @staticmethod
    def process_nested_dicts(o: dict):
        for k, v in o.items():
            if isinstance(v, Mapping):
                o[k] = {str(key): val for key, val in v.items()}
        return o

    @classmethod
    def default(cls, o):
        if is_dataclass(o):
            return asdict(o)
        if isinstance(o, date) and (not isinstance(o, datetime)):
            return format_datetime(datetime(o.year, o.month, o.day, tzinfo=timezone.utc), usegmt=True)
        if hasattr(o, '__html__'):
            return str(o.__html__())
        match o:
            case UUID():
                return str(o)
            case OrderedMapSerializedKey():
                return {str(k): v for k, v in o.items()}
            case BaseModel():
                o = {str(k): v for k, v in o.model_dump().items()}
                o = cls.process_nested_dicts(o)
                return o
            case dict():
                return {str(k): v for k, v in o.items()}
            case datetime():
                return o.strftime('%Y-%m-%dT%H:%M:%S.%fZ')[:-4] + 'Z'
            case bytes():
                return o.decode('utf-8', errors='replace')
            case _:
                return ArgusJSONEncoder().default(o)

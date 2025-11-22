"""
Data models for the LLM Schema Registry Python SDK.
"""

from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field, field_validator


class SchemaFormat(str, Enum):
    """Supported schema formats."""

    JSON_SCHEMA = "json_schema"
    AVRO = "avro"
    PROTOBUF = "protobuf"


class CompatibilityMode(str, Enum):
    """Schema compatibility checking modes."""

    BACKWARD = "backward"
    FORWARD = "forward"
    FULL = "full"
    BACKWARD_TRANSITIVE = "backward_transitive"
    FORWARD_TRANSITIVE = "forward_transitive"
    FULL_TRANSITIVE = "full_transitive"
    NONE = "none"


class SchemaMetadata(BaseModel):
    """Metadata for a schema."""

    description: Optional[str] = None
    tags: List[str] = Field(default_factory=list)
    owner: Optional[str] = None
    custom: Dict[str, Any] = Field(default_factory=dict)


class Schema(BaseModel):
    """Schema definition."""

    namespace: str = Field(..., description="Schema namespace (e.g., 'telemetry')")
    name: str = Field(..., description="Schema name (e.g., 'InferenceEvent')")
    version: str = Field(..., description="Semantic version (e.g., '1.0.0')")
    format: SchemaFormat = Field(..., description="Schema format")
    content: str = Field(..., description="Schema content (JSON/Avro/Protobuf)")
    metadata: Optional[SchemaMetadata] = None

    @field_validator("version")
    @classmethod
    def validate_version(cls, v: str) -> str:
        """Validate semantic versioning format."""
        parts = v.split(".")
        if len(parts) != 3:
            raise ValueError("Version must be in semver format (major.minor.patch)")
        for part in parts:
            if not part.isdigit():
                raise ValueError(f"Version part must be numeric: {part}")
        return v


class RegisterSchemaResponse(BaseModel):
    """Response from schema registration."""

    schema_id: str = Field(..., description="Unique schema ID (UUID)")
    version: str = Field(..., description="Registered version")
    created_at: datetime = Field(..., description="Creation timestamp")


class GetSchemaResponse(BaseModel):
    """Response from schema retrieval."""

    schema_id: str
    namespace: str
    name: str
    version: str
    format: SchemaFormat
    content: str
    metadata: Optional[SchemaMetadata] = None
    created_at: datetime
    updated_at: datetime


class ValidateResponse(BaseModel):
    """Response from data validation."""

    is_valid: bool = Field(..., description="Whether data is valid")
    errors: List[str] = Field(default_factory=list, description="Validation error messages")


class CompatibilityResult(BaseModel):
    """Result of compatibility checking."""

    is_compatible: bool = Field(..., description="Whether schemas are compatible")
    incompatibilities: List[str] = Field(
        default_factory=list, description="List of incompatibility issues"
    )
    mode: CompatibilityMode = Field(..., description="Compatibility mode used")


class SchemaVersion(BaseModel):
    """Schema version information."""

    version: str
    schema_id: str
    created_at: datetime


class SearchResult(BaseModel):
    """Schema search result."""

    schema_id: str
    namespace: str
    name: str
    version: str
    description: Optional[str] = None
    tags: List[str] = Field(default_factory=list)
    score: float = Field(..., description="Relevance score")

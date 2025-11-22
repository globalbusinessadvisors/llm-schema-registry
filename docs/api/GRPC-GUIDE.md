# LLM Schema Registry - gRPC API Guide

## Overview

The gRPC API provides high-performance access to schema registry operations with support for streaming and bidirectional communication.

## Connection Details

- **Endpoint:** `localhost:9090`
- **Protocol:** HTTP/2 with TLS (production)
- **Authentication:** Metadata-based token authentication

## Proto Definition

The complete protocol buffer definition is available at: `/proto/schema_registry.proto`

## Authentication

Include authentication token in gRPC metadata:

```python
# Python example
import grpc
from schema_registry_pb2_grpc import SchemaRegistryStub

channel = grpc.insecure_channel('localhost:9090')
stub = SchemaRegistryStub(channel)

metadata = [
    ('authorization', 'Bearer <your-jwt-token>')
]

# Make authenticated request
response = stub.GetSchema(
    GetSchemaRequest(schema_id='550e8400...'),
    metadata=metadata
)
```

```go
// Go example
import (
    "context"
    "google.golang.org/grpc"
    "google.golang.org/grpc/metadata"
)

conn, _ := grpc.Dial("localhost:9090", grpc.WithInsecure())
client := NewSchemaRegistryClient(conn)

md := metadata.New(map[string]string{
    "authorization": "Bearer <your-jwt-token>",
})
ctx := metadata.NewOutgoingContext(context.Background(), md)

response, _ := client.GetSchema(ctx, &GetSchemaRequest{
    SchemaId: "550e8400...",
})
```

## Key Operations

### 1. RegisterSchema

```protobuf
rpc RegisterSchema(RegisterSchemaRequest) returns (RegisterSchemaResponse);
```

**Example:**
```python
from schema_registry_pb2 import RegisterSchemaRequest, SchemaType

request = RegisterSchemaRequest(
    subject="com.example.user.created",
    schema_content=schema_bytes,
    schema_type=SchemaType.SCHEMA_TYPE_JSON,
    metadata={"owner": "team-a"},
    description="User creation event",
    tags=["user", "events"],
    auto_version=True
)

response = stub.RegisterSchema(request, metadata=metadata)
print(f"Schema ID: {response.schema_id}")
print(f"Version: {response.version}")
```

### 2. GetSchema

```protobuf
rpc GetSchema(GetSchemaRequest) returns (GetSchemaResponse);
```

**Example:**
```python
request = GetSchemaRequest(schema_id="550e8400-e29b-41d4-a716-446655440000")
response = stub.GetSchema(request, metadata=metadata)
print(f"Subject: {response.schema.subject}")
print(f"Version: {response.schema.version}")
```

### 3. ListSchemas (Server Streaming)

```protobuf
rpc ListSchemas(ListSchemasRequest) returns (stream SchemaInfo);
```

**Example:**
```python
request = ListSchemasRequest(
    subject_prefix="com.example",
    schema_type=SchemaType.SCHEMA_TYPE_JSON,
    limit=100
)

for schema in stub.ListSchemas(request, metadata=metadata):
    print(f"Schema: {schema.subject} v{schema.version}")
```

### 4. ValidateData

```protobuf
rpc ValidateData(ValidateDataRequest) returns (ValidationReport);
```

**Example:**
```python
import json

data = {"id": "user-123", "email": "user@example.com"}
request = ValidateDataRequest(
    schema_id="550e8400-e29b-41d4-a716-446655440000",
    data=json.dumps(data).encode('utf-8'),
    strict=True
)

report = stub.ValidateData(request, metadata=metadata)
if report.valid:
    print("Data is valid!")
else:
    for error in report.errors:
        print(f"Error at {error.path}: {error.message}")
```

### 5. CheckCompatibility

```protobuf
rpc CheckCompatibility(CompatibilityCheckRequest) returns (CompatibilityReport);
```

**Example:**
```python
request = CompatibilityCheckRequest(
    subject="com.example.user.created",
    new_schema=new_schema_bytes,
    level=CompatibilityLevel.COMPATIBILITY_LEVEL_BACKWARD
)

report = stub.CheckCompatibility(request, metadata=metadata)
if report.compatible:
    print("Schema is compatible!")
else:
    for violation in report.violations:
        print(f"{violation.rule}: {violation.message}")
```

### 6. BatchValidate (Bidirectional Streaming)

```protobuf
rpc BatchValidate(stream ValidateDataRequest) returns (stream ValidationReport);
```

**Example:**
```python
def request_generator():
    for data_item in data_batch:
        yield ValidateDataRequest(
            schema_id="550e8400-e29b-41d4-a716-446655440000",
            data=json.dumps(data_item).encode('utf-8'),
            strict=True
        )

responses = stub.BatchValidate(request_generator(), metadata=metadata)
for report in responses:
    print(f"Valid: {report.valid}, Time: {report.validation_time_ms}ms")
```

### 7. StreamSchemaChanges (Server Streaming)

```protobuf
rpc StreamSchemaChanges(StreamRequest) returns (stream SchemaChangeEvent);
```

**Example:**
```python
request = StreamRequest(
    subjects=["com.example.*", "com.acme.order.*"],
    event_types=[
        EventType.EVENT_TYPE_SCHEMA_REGISTERED,
        EventType.EVENT_TYPE_SCHEMA_UPDATED
    ]
)

for event in stub.StreamSchemaChanges(request, metadata=metadata):
    print(f"{event.event_type}: {event.subject} v{event.version}")
    print(f"Changed by: {event.changed_by}")
```

### 8. HealthCheck

```protobuf
rpc HealthCheck(google.protobuf.Empty) returns (HealthCheckResponse);
```

**Example:**
```python
from google.protobuf.empty_pb2 import Empty

response = stub.HealthCheck(Empty())
print(f"Status: {response.status}")
for component, health in response.components.items():
    print(f"{component}: {health.status}")
```

## Complete Python Example

```python
import grpc
import json
from schema_registry_pb2 import (
    RegisterSchemaRequest,
    GetSchemaRequest,
    ValidateDataRequest,
    CompatibilityCheckRequest,
    SchemaType,
    CompatibilityLevel
)
from schema_registry_pb2_grpc import SchemaRegistryStub

class SchemaRegistryClient:
    def __init__(self, endpoint, token):
        self.channel = grpc.insecure_channel(endpoint)
        self.stub = SchemaRegistryStub(self.channel)
        self.metadata = [('authorization', f'Bearer {token}')]

    def register_schema(self, subject, schema, schema_type='json'):
        schema_type_enum = {
            'json': SchemaType.SCHEMA_TYPE_JSON,
            'avro': SchemaType.SCHEMA_TYPE_AVRO,
            'protobuf': SchemaType.SCHEMA_TYPE_PROTOBUF
        }[schema_type]

        request = RegisterSchemaRequest(
            subject=subject,
            schema_content=json.dumps(schema).encode('utf-8'),
            schema_type=schema_type_enum,
            auto_version=True
        )

        return self.stub.RegisterSchema(request, metadata=self.metadata)

    def get_schema(self, schema_id):
        request = GetSchemaRequest(schema_id=schema_id)
        return self.stub.GetSchema(request, metadata=self.metadata)

    def validate_data(self, schema_id, data):
        request = ValidateDataRequest(
            schema_id=schema_id,
            data=json.dumps(data).encode('utf-8'),
            strict=True
        )
        return self.stub.ValidateData(request, metadata=self.metadata)

    def check_compatibility(self, subject, new_schema):
        request = CompatibilityCheckRequest(
            subject=subject,
            new_schema=json.dumps(new_schema).encode('utf-8'),
            level=CompatibilityLevel.COMPATIBILITY_LEVEL_BACKWARD
        )
        return self.stub.CheckCompatibility(request, metadata=self.metadata)

# Usage
client = SchemaRegistryClient('localhost:9090', 'your-jwt-token')

# Register schema
schema = {
    "type": "object",
    "properties": {
        "id": {"type": "string"},
        "email": {"type": "string", "format": "email"}
    },
    "required": ["id", "email"]
}

response = client.register_schema("com.example.user", schema)
print(f"Registered schema: {response.schema_id}")

# Validate data
data = {"id": "user-123", "email": "user@example.com"}
report = client.validate_data(response.schema_id, data)
print(f"Valid: {report.valid}")
```

## Complete Go Example

```go
package main

import (
    "context"
    "encoding/json"
    "log"

    "google.golang.org/grpc"
    "google.golang.org/grpc/metadata"
    pb "your-org/schema-registry/proto"
)

type Client struct {
    conn   *grpc.ClientConn
    client pb.SchemaRegistryClient
    token  string
}

func NewClient(endpoint, token string) (*Client, error) {
    conn, err := grpc.Dial(endpoint, grpc.WithInsecure())
    if err != nil {
        return nil, err
    }

    return &Client{
        conn:   conn,
        client: pb.NewSchemaRegistryClient(conn),
        token:  token,
    }, nil
}

func (c *Client) context() context.Context {
    md := metadata.New(map[string]string{
        "authorization": "Bearer " + c.token,
    })
    return metadata.NewOutgoingContext(context.Background(), md)
}

func (c *Client) RegisterSchema(subject string, schema map[string]interface{}) (*pb.RegisterSchemaResponse, error) {
    schemaBytes, _ := json.Marshal(schema)

    req := &pb.RegisterSchemaRequest{
        Subject:      subject,
        SchemaContent: schemaBytes,
        SchemaType:   pb.SchemaType_SCHEMA_TYPE_JSON,
        AutoVersion:  true,
    }

    return c.client.RegisterSchema(c.context(), req)
}

func (c *Client) ValidateData(schemaID string, data map[string]interface{}) (*pb.ValidationReport, error) {
    dataBytes, _ := json.Marshal(data)

    req := &pb.ValidateDataRequest{
        SchemaId: schemaID,
        Data:     dataBytes,
        Strict:   true,
    }

    return c.client.ValidateData(c.context(), req)
}

func main() {
    client, err := NewClient("localhost:9090", "your-jwt-token")
    if err != nil {
        log.Fatal(err)
    }
    defer client.conn.Close()

    // Register schema
    schema := map[string]interface{}{
        "type": "object",
        "properties": map[string]interface{}{
            "id":    map[string]string{"type": "string"},
            "email": map[string]string{"type": "string", "format": "email"},
        },
        "required": []string{"id", "email"},
    }

    resp, err := client.RegisterSchema("com.example.user", schema)
    if err != nil {
        log.Fatal(err)
    }
    log.Printf("Registered: %s v%s", resp.SchemaId, resp.Version)

    // Validate data
    data := map[string]string{
        "id":    "user-123",
        "email": "user@example.com",
    }

    report, err := client.ValidateData(resp.SchemaId, data)
    if err != nil {
        log.Fatal(err)
    }
    log.Printf("Valid: %v", report.Valid)
}
```

## Performance Optimization

### Connection Pooling

```python
# Reuse channel across requests
channel = grpc.insecure_channel(
    'localhost:9090',
    options=[
        ('grpc.keepalive_time_ms', 10000),
        ('grpc.keepalive_timeout_ms', 5000),
        ('grpc.http2.max_pings_without_data', 0),
        ('grpc.keepalive_permit_without_calls', 1)
    ]
)
```

### Compression

```python
# Enable gzip compression
response = stub.GetSchema(
    request,
    metadata=metadata,
    compression=grpc.Compression.Gzip
)
```

### Deadline/Timeout

```python
import time

# Set 5-second deadline
response = stub.GetSchema(
    request,
    metadata=metadata,
    timeout=5.0
)
```

## Error Handling

```python
import grpc

try:
    response = stub.GetSchema(request, metadata=metadata)
except grpc.RpcError as e:
    if e.code() == grpc.StatusCode.NOT_FOUND:
        print("Schema not found")
    elif e.code() == grpc.StatusCode.UNAUTHENTICATED:
        print("Authentication failed")
    elif e.code() == grpc.StatusCode.PERMISSION_DENIED:
        print("Permission denied")
    else:
        print(f"Error: {e.details()}")
```

## TLS/mTLS Configuration

### Server-side TLS

```python
with open('server.crt', 'rb') as f:
    server_cert = f.read()

credentials = grpc.ssl_channel_credentials(root_certificates=server_cert)
channel = grpc.secure_channel('registry.example.com:9090', credentials)
```

### Mutual TLS (mTLS)

```python
with open('client.crt', 'rb') as f:
    client_cert = f.read()
with open('client.key', 'rb') as f:
    client_key = f.read()
with open('ca.crt', 'rb') as f:
    ca_cert = f.read()

credentials = grpc.ssl_channel_credentials(
    root_certificates=ca_cert,
    private_key=client_key,
    certificate_chain=client_cert
)

channel = grpc.secure_channel('registry.example.com:9090', credentials)
```

## Service Reflection

The gRPC server supports reflection for dynamic discovery:

```bash
# List services
grpcurl -plaintext localhost:9090 list

# Describe service
grpcurl -plaintext localhost:9090 describe schema_registry.v1.SchemaRegistry

# Call method
grpcurl -plaintext \
  -d '{"schema_id": "550e8400-e29b-41d4-a716-446655440000"}' \
  localhost:9090 \
  schema_registry.v1.SchemaRegistry/GetSchema
```

## Next Steps

- [REST API Guide](./API-GUIDE.md)
- [Client SDK Examples](./SDK-EXAMPLES.md)
- [Performance Tuning](./PERFORMANCE.md)

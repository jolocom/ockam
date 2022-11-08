## Discovery API

Shows info about services running on the node

Worker address: "discovery"

Implemented in `Ockam.Services.API.Discovery`

**NOTE: this API is a work in progress**

#### List services
Method: GET
Path: ""
Request: ""
Response: [ServiceInfo]

#### Show service
Method: GET
Path: ":service_id"
Request: ""
Response: ServiceInfo

#### Register service
Method: PUT
Path: ":service_id"
Request: ServiceInfo
Response: ""

Errors:
400 - cannot decode ServiceInfo
405 - method not allowed

Some backends do not support service registration and will always return status 405

Where:
```
ServiceInfo: {
  id: text,
  route: {
    type: uint,
    value: binary
  },
  metadata: {* binary => binary}
}
```




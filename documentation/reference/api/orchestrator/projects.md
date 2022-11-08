## Projects API

Provides control over orchestrator projects.

Worker address: "projects"

Authorization:
- Requires connection via secure channel
- Identity needs to be enrolled to the Orchestrator Controller via [Auth0](./auth0.md)

#### List projects
Method: GET
Path: "/v0"
Request: ""
Response: `[Project]`

#### Create project for space
Method: POST
Path: "/v0/:space_id"
Request: CreateRequest
Response: Project

Errors:
401 - current identity does not have permission to create a project for that space
409 - name should be unique
400 - invalid name, it should match the regexp: `^([[:alnum:]])+([-_\.]?[[:alnum:]])*`
400 - invalid request format

#### Show project
Method: GET
Path: /v0/:project_id
Request: ""
Response: Project

Errors:
404 - not found
401 - current identity does not have permission to show the project

#### Update project
Mathod: PUT
Path: /v0/:project_id
Request: UpdateRequest
Response: Project

Errors:
404 - not found
401 - current identity does not have permission to update the project
409 - name should be unique
400 - invalid name, it should match the regexp: `^([[:alnum:]])+([-_\.]?[[:alnum:]])*$`
400 - invalid request format

#### Delete project
Method: DELETE
Path: /v0/:space_id/:project_id
Request: ""
Response: ""

Errors:
404 - not found
401 - current identity does not have permission to delete the project

Where:
```
CreateRequest = UpdateRequest: {
  name: text,
  services: [text],
  users: [text],
  enforce_credentials: optional(bool)
}
Project: {
  id: text,
  name: text,
  space_name: text,
  services: [text],
  node_access_route: text,
  users: [text],
  space_id: text,
  node_identity: text,
  authority_access_route: text,
  authority_identity: text,
  okta_addon: optional(OktaConfig)
}

OktaConfig: {
  tenant_base_url: text,
  certificate: text,
  client_id: text,
  attributes: [text]
}
```


## Additional APIs for projects

- [Project Enrollers API](./project/enrollers.md)
- [Project Addons API](./project/addons.md)

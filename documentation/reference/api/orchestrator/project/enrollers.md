## Project Enrollers API

Controls enrollers list for projects

Worker address: "projects"

Authorization:
- Requires connection via secure channel
- Identity needs to be enrolled to the Orchestrator Controller via [Auth0](./auth0.md)

#### List enrollers
Method: GET
Path: "/v0/:project_id/enrollers"
Request: ""
Response: `[Enroller]`

Errors:
404 - project not found
401 - current identity does not have permission to get enrollers from the project

#### Create enroller
Method: POST
Path: "/v0/:project_id/enrollers"
Request: CreateRequest
Response: Enroller

Errors:
404 - project not found
401 - current identity does not have permission to create enrollers in the project

#### Delete enroller
Method: DELETE
Path: /v0/:project_id/enrollers/:enroller
Request: ""
Response: ""

Errors:
404 - project not found
401 - current identity does not have permission to delete enrollers in the project

Where:
```
CreateRequest: {
  identity_id: text,
  description: optional(text)
}

Enroller: {
  identity_id: text
  description: optional(text)
  added_by: text ;; email
  created_at: text ;; ISO 8601 date time
}
```

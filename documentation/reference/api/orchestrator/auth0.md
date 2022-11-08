## Auth0 API

Allows clients to enroll their identity with Auth0 authenticator

Worker address: "auth0_authenticator"

Authorization:
**Requires connection via secure channel**

#### Enroll with auth0
Method: POST
Path: "v0/enroll"
Request: EnrollRequest
Response: ""

Errors:
400 - invalid request format
400 - invalid token type

Where:
```
EnrollRequest: {
  token_type: TokenType,
  access_token: text
}

TokenType: 0 ;; bearer
```

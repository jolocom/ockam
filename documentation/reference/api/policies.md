## ABAC policies API

Allows to get and set [ABAC](../../authorization/ABAC.md) policies.

Worker address: "abac_policies"

Implemented in `Ockam.Services.API.ABAC.PoliciesApi`

#### List policies
Method: GET
Path: ""
Request: ""
Response: `{* ActionId => PolicyRule}`

#### Show policy
Method: GET
Path: ActionId
Request: ""
Response: PolicyRule

#### Set policy
Method: PUT
Path: ActionId
Request: PolicyRule
Response: ""

Errors:
400 - cannot decode policy

#### Delete policy
Method: DELETE
Path: ActionId
Request: ""
Response: ""

Where:
```
ActionId: text ;; ":resource/:action"
PolicyRule: text ;; ABAC s-expression rule
```

For more info on ABAC policies and rules see [ABAC](../../authorization/ABAC.md)

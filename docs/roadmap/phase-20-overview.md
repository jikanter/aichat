# Phase 20: Remote & Federated Composition : Overview - Epic 6

| Item | Description | Status |
|---|---|---|
| 20A | Remote role resolution (`remote:host:port/role-name` addressing) | -- |
| 20B | Remote role discovery (query remote aichat's `/v1/roles` for capabilities) | -- |
| 20C | `remotes:` config section (named remote aichat instances) | -- |
| 20D | Federated pipeline execution (stages can reference remote roles) | -- |

**20A Design — Remote Resolution:**

```yaml
# config.yaml
remotes:
  staging:
    endpoint: http://staging.internal:8080
    api_key: ${STAGING_API_KEY}
  security:
    endpoint: http://security-scanner.internal:8080
```

```yaml
# roles/secure-review.md
pipeline:
  - role: extract                              # local
  - role: remote:security/vulnerability-scan   # remote aichat instance
  - role: summarize                            # local
```

**Implementation:** `RemoteRoleResolver` implements `RoleResolver`. Resolution calls `GET /v1/roles/{name}` on the remote. Execution calls `POST /v1/roles/{name}/invoke`. Requires Epic 5 Phase 17B to exist.

This is the pattern the user discovered accidentally — two aichat instances composing roles across machines. A triage role on machine A routes to code-analysis on machine B (which has the codebase) and security-scan on machine C (which has vulnerability databases).

**Files:** `src/config/resolver.rs` (add `RemoteRoleResolver`), `src/config/mod.rs` (parse `remotes:` config), `src/pipe.rs` (dispatch to remote resolver for `remote:` prefix stages).

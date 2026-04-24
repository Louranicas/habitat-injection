> Back to: [[HOME]]

# Comms Layer v3 Alignment

Per Comms Layer v3 §10.4, every v3 mechanism maps 1:1 to an STDB primitive. A STDB-native v4 would collapse ~30-40% of WS-2 LOC.

## Mapping

| v3 Mechanism | STDB Primitive | Migration Phase |
|---|---|---|
| `/bus/ingress` consent-gate (WS-3) | Row-level security on [[T1 — HabitatEvent]] | D |
| Subscription patterns (WS-2a) | STDB typed query subscriptions | E |
| `/bus/forget` cascade (WS-2d) | [[Reducers#R6 forget_sphere]] | C |
| Event schemas (WS-2b) | STDB table schemas + typed bindings | A |
| `/bus/self` introspection (WS-2d) | STDB module metadata | D |
| Subscriber identity (WS-2a) | STDB OIDC Identity | Post-E |
| Auth token → OIDC | Single workstream (~5h) | Post-E |
| habitat-wire role specialization (WS-6) | RLS policies keyed on caller role | Post-E |

## OIDC Note

STDB uses OIDC natively. The v3 shared-secret token in `/bus/ingress` is a pragmatic stand-in. Migration to OIDC is ~5h when STDB lands. Supports Auth0, Clerk, Keycloak, Google, GitHub, or custom provider.

---

See: [[Executive Summary]] · [[Phase D — Cross-Service Integration]]

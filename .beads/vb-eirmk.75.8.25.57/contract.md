# Contract: expose the existing secret-unavailable runtime code

## Scope

Add the existing Section 17 `SECRET_UNAVAILABLE` runtime string to the current `RuntimeError::SecretUnavailable` projection. Preserve its numeric diagnostic `0x2026`, registered symbolic identity, display text, variant, and every refusal/state behavior. Add no public constant, API, dependency, schema, persisted format, or new runtime label.

## Domain facts

- `Shard::validate_secret_presence` returns `RuntimeError::SecretUnavailable` when a required host secret name is unavailable; strict owned activation reaches this producer through `Runtime::activate_owned_submission`.
- `SECRET_UNAVAILABLE` is already in MASTER Section 17 and currently registered symbolically. The missing behavior is only `RuntimeError::runtime_code()` reporting.
- A strict owned activation refusal must leave the retained journal event stream unchanged. Existing available-secret activation and completion remain valid.
- Section 17 coverage must classify this existing runtime producer as mapped only after the production projection and collector both include it. Keep the 33 names partitioned as 20 mapped, 13 unmapped, 0 partially mapped; do not invent mappings for other gaps.

## Observable behavior

- The existing strict owned activation test observes `Some("SECRET_UNAVAILABLE")` for the real refusal while preserving its exact `SecretUnavailable` result, numeric diagnostic, symbolic code, display, zero queued/active state, and unchanged journal event stream.
- The same test retains the available-secret acceptance and completion control.
- The Section 17 coverage collector includes `RuntimeError::SecretUnavailable`; coverage assertions retain every existing code identity and case with updated classification counts.

## Boundaries and hazards

- Change only the runtime-code projection and the existing owned-activation and Section 17 coverage tests.
- Preserve all strict artifact checks, queue/frame/refusal behavior, and accepted-secret behavior. Do not add a generic label fallback or alter diagnostics.

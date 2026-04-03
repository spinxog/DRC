Below is a **build TODO** for Deterministic Replay Compute, written like an execution plan rather than a pitch.

---

# Deterministic Replay Compute — TODO

## 0. Product boundary

[ ] Pick the first supported scope:

* single backend request replay
* async worker/job replay
* narrow multi-step workflow replay

[ ] Pick the first supported runtime:

* Go
* Node
* Python
* JVM

[ ] Decide the replay fidelity target for v1:

* I/O deterministic only
* partial concurrency awareness
* no full scheduler determinism in v1

[ ] Define what “replayable” means for v1

[ ] Define what is explicitly out of scope for v1

[ ] Define the primary user workflow:

* inspect failed execution
* replay exact execution
* mutate one variable
* compare divergence

---

# 1. Execution model

[ ] Define the system’s core unit of replay:

* request
* job
* workflow step

[ ] Define execution lifecycle states:

* started
* capturing
* finalized
* indexed
* replayable
* partial
* corrupted
* expired

[ ] Define execution metadata schema:

* execution id
* parent id
* trace/workflow id
* service name
* instance
* region
* environment
* build id
* git SHA
* runtime version
* start/end time

[ ] Define event ordering guarantees

[ ] Define causal linkage model between events

[ ] Define parent/child execution relationships

[ ] Define how retries are represented

[ ] Define how fanout/fanin is represented

---

# 2. Event schema

[ ] Define canonical event types:

* inbound request
* outbound request
* DB read
* DB write
* cache read
* cache write
* queue consume
* queue publish
* file read
* file write
* config read
* feature flag resolution
* time read
* randomness read
* function marker
* exception/error
* transaction begin/commit/rollback

[ ] Define per-event required fields

[ ] Define per-event optional fields

[ ] Define payload encoding strategy

[ ] Define large payload handling strategy

[ ] Define redaction-capable fields in schema

[ ] Define versioning for schema evolution

[ ] Define event integrity validation rules

[ ] Define deduplication rules for duplicate events

[ ] Define event correlation fields for cross-service linkage

---

# 3. Capture coverage strategy

[ ] List all nondeterministic sources that must be captured:

* wall clock
* monotonic clock
* randomness
* UUID generation
* environment reads
* config reads
* feature flags
* DB query results
* cache reads
* network responses
* queue delivery
* filesystem reads
* DNS results

[ ] Separate “must capture” from “nice to have”

[ ] Define capture confidence model

[ ] Define replay confidence model based on coverage

[ ] Define missing-capture detection strategy

[ ] Define what causes replay to be marked approximate instead of exact

---

# 4. Runtime instrumentation

[ ] Pick the v1 instrumentation strategy:

* SDK only
* SDK + proxy
* SDK + syscall supplementation

[ ] Define hook points for inbound request capture

[ ] Define hook points for outbound HTTP/gRPC capture

[ ] Define hook points for DB client capture

[ ] Define hook points for queue client capture

[ ] Define hook points for cache capture

[ ] Define hook points for time/randomness wrapping

[ ] Define hook points for config/flag resolution capture

[ ] Define hook points for exception capture

[ ] Define function marker mechanism

[ ] Define instrumentation lifecycle:

* init
* per-request context attach
* event emit
* flush
* finalize

[ ] Define agent failure behavior

[ ] Define backpressure behavior in agents

[ ] Define what happens if capture partially fails mid-execution

---

# 5. Proxy / sidecar / network interception

[ ] Decide whether v1 includes proxy-based network capture

[ ] Define supported protocols in v1:

* HTTP
* gRPC
* PostgreSQL
* MySQL
* Redis
* Kafka
* SQS
* others later

[ ] Define request/response normalization model

[ ] Define correlation strategy between application events and proxy events

[ ] Define retry visibility model

[ ] Define timeout and cancellation capture

[ ] Define partial-body / streaming capture behavior

[ ] Define binary protocol capture constraints

---

# 6. Syscall / kernel-level supplementation

[ ] Decide whether kernel/event-level capture is in v1 or later

[ ] Define which gaps syscall capture is meant to fill

[ ] Define supported host environments

[ ] Define operational model for privileged capture

[ ] Define performance overhead budget for kernel hooks

[ ] Define fallback when kernel capture unavailable

---

# 7. Artifact and environment provenance

[ ] Define how code identity is captured:

* container image digest
* build id
* git SHA
* lockfile hash
* runtime version

[ ] Define how environment identity is captured:

* env vars read
* config snapshot
* feature flag values
* secret references
* region/cluster metadata

[ ] Define artifact resolution for replay

[ ] Define retention of artifact references

[ ] Define behavior when original artifact no longer exists

[ ] Define compatibility rules for replaying with substitute artifacts

---

# 8. State reconstruction strategy

[ ] Choose v1 state model:

* read-result replay
* snapshot replay
* hybrid

[ ] Define what constitutes the execution’s starting state

[ ] Define how DB reads are recorded for replay

[ ] Define how cache reads are recorded for replay

[ ] Define how config/flag reads are recorded for replay

[ ] Define how file reads are recorded for replay

[ ] Define how implicit hidden reads are detected

[ ] Define when a state snapshot is required vs optional

[ ] Define snapshot reference model

[ ] Define snapshot integrity verification

[ ] Define behavior when state source cannot be reconstructed

[ ] Define fidelity downgrade path when reconstruction incomplete

---

# 9. Write / side-effect model

[ ] Define categories of side effects:

* DB writes
* cache writes
* queue publish
* file write
* email send
* webhook send
* third-party API mutation
* payment call

[ ] Define how writes are captured

[ ] Define how writes are represented in replay

[ ] Define the default replay side-effect policy:

* block
* sink
* simulate
* allow only via explicit override

[ ] Define sink behavior for every side-effect class

[ ] Define irreversible side-effect tagging

[ ] Define safe simulation policy for payments/notifications/webhooks

[ ] Define what side effects can never be allowed in replay

---

# 10. Replay runtime

[ ] Define replay runtime API contract

[ ] Define how original execution is loaded into replay runtime

[ ] Define replay execution lifecycle:

* prepare
* resolve artifacts
* resolve state
* start
* intercept reads
* sink writes
* finalize
* diff

[ ] Implement virtual clock conceptually:

* frozen time
* event-driven time
* stepped time

[ ] Implement virtual randomness conceptually

[ ] Define how environment reads are served during replay

[ ] Define how DB reads are served during replay

[ ] Define how outbound calls are served during replay

[ ] Define how queue deliveries are served during replay

[ ] Define isolation boundaries for replay processes

[ ] Define log capture during replay

[ ] Define what replay emits back into the same event schema

[ ] Define timeout handling for stuck replays

[ ] Define cancellation model for replay jobs

[ ] Define replay cleanup model

---

# 11. Replay fidelity and confidence

[ ] Define exact meaning of:

* exact replay
* high-confidence replay
* approximate replay
* invalid replay

[ ] Define confidence score inputs:

* capture completeness
* state reconstruction completeness
* hidden-read risk
* artifact mismatch
* unsupported features used

[ ] Define user-visible fidelity explanations

[ ] Define when replay should be blocked entirely

[ ] Define how unsupported behaviors are surfaced

---

# 12. Concurrency model

[ ] Decide whether v1 ignores scheduler determinism

[ ] Define concurrency support level for v1

[ ] Define how thread/task boundaries are represented

[ ] Define whether lock acquisition/release events are recorded

[ ] Define whether goroutine/thread/task spawn/join is recorded

[ ] Define whether async callbacks are recorded as causal children

[ ] Define race-condition handling posture in v1:

* unsupported
* best effort
* partially modeled

[ ] Define phase-2 plan for happens-before graph

[ ] Define phase-3 plan for deterministic scheduling

---

# 13. Mutation engine

[ ] Define supported mutation classes in v1:

* request payload mutation
* header/auth mutation
* env/config mutation
* feature flag mutation
* dependency response mutation
* timeout injection
* artifact swap

[ ] Define mutation application order

[ ] Define mutation validation rules

[ ] Define mutation conflict resolution rules

[ ] Define mutation auditability

[ ] Define immutable original execution guarantee

[ ] Define replay spec format for mutations

[ ] Define unsupported mutation classes for v1

[ ] Define later mutation classes:

* scheduler mutation
* DB row mutation
* queue reorder
* code patch injection

---

# 14. Diff engine

[ ] Define comparison dimensions:

* output diff
* state-write diff
* path diff
* dependency-call diff
* exception diff
* timing diff

[ ] Define output equality rules

[ ] Define semantic diffing vs byte diffing

[ ] Define diff normalization rules

[ ] Define how to compare structured DB writes

[ ] Define how to compare emitted events/messages

[ ] Define how to compare function/trace paths

[ ] Define first-divergence algorithm

[ ] Define noisy-diff suppression rules

[ ] Define diff severity ranking

[ ] Define diff explanation format

[ ] Define machine-readable diff format

---

# 15. Root divergence localization

[ ] Define what qualifies as first meaningful divergence

[ ] Define branch divergence detection

[ ] Define read divergence detection

[ ] Define write divergence detection

[ ] Define exception divergence detection

[ ] Define dependency divergence detection

[ ] Define user-visible causal chain format:

* first changed input
* first changed read
* first changed branch
* downstream changed writes
* final changed output

[ ] Define confidence score for root-cause localization

---

# 16. Storage architecture

[ ] Define hot index storage for metadata/search

[ ] Define immutable event segment storage

[ ] Define snapshot/blob storage

[ ] Define artifact reference storage

[ ] Define retention tiers:

* hot
* warm
* cold
* expired

[ ] Define compression strategy for event segments

[ ] Define deduplication strategy for repeated payloads

[ ] Define content-addressing model

[ ] Define encryption-at-rest requirements

[ ] Define per-tenant isolation model

[ ] Define cleanup and GC model

[ ] Define corruption detection and repair strategy

---

# 17. Ingestion pipeline

[ ] Define collector-to-backend ingest protocol

[ ] Define event batching policy

[ ] Define backpressure handling

[ ] Define out-of-order event handling

[ ] Define duplicate event handling

[ ] Define partial execution finalization rules

[ ] Define ingestion retry behavior

[ ] Define exactly-once vs at-least-once posture

[ ] Define segment flush thresholds

[ ] Define integrity checksum strategy

[ ] Define ingestion observability for the ingest pipeline itself

---

# 18. Query and search layer

[ ] Define search dimensions:

* service
* endpoint/job
* error class
* deploy/build
* time range
* tenant
* execution id
* trace/workflow id

[ ] Define filters for replayable vs partial executions

[ ] Define filters for fidelity levels

[ ] Define saved searches / common incident views

[ ] Define grouping by failure signature

[ ] Define grouping by first-divergence site

[ ] Define indexing policy for large payload fields

---

# 19. Control plane

[ ] Define operator workflow screens:

* execution detail
* replay launch
* mutation setup
* replay result
* diff view

[ ] Define APIs for:

* fetch execution
* create replay
* apply mutations
* launch replay
* fetch diff
* fetch fidelity report

[ ] Define replay job state model in control plane

[ ] Define permissions by action:

* view capture
* create replay
* mutate replay
* swap artifact
* allow live dependency access

[ ] Define audit log model

[ ] Define replay history tracking

---

# 20. Security model

[ ] Define tenant isolation boundaries

[ ] Define authn/authz model

[ ] Define field-level redaction policy

[ ] Define PII classification policy

[ ] Define encryption for data in transit

[ ] Define encryption for data at rest

[ ] Define secret handling strategy during capture

[ ] Define secret handling strategy during replay

[ ] Define key management requirements

[ ] Define role restrictions for sensitive replays

[ ] Define approval gates for high-risk mutation types

[ ] Define export restrictions

[ ] Define data residency requirements

---

# 21. Privacy and compliance

[ ] Define GDPR/CCPA deletion model

[ ] Define retention override model for regulated workloads

[ ] Define access logging requirements

[ ] Define replay data masking policies

[ ] Define customer-managed keys support requirements

[ ] Define compliance posture targets:

* SOC 2
* ISO 27001
* HIPAA later if relevant
* PCI considerations if payment traffic involved

[ ] Define legal/audit trail format

---

# 22. Performance and overhead

[ ] Define allowed latency overhead budget for capture

[ ] Define allowed CPU overhead budget

[ ] Define allowed memory overhead budget

[ ] Define allowed storage amplification budget

[ ] Define event sampling strategy, if any

[ ] Define escalation strategy from sampled to full capture

[ ] Define high-traffic protection mechanisms

[ ] Define adaptive throttling policy

[ ] Define loss budgets for noncritical event classes

[ ] Define “strict capture mode” for incident-targeted workloads

---

# 23. Reliability of the DRC system itself

[ ] Define failure modes of agents

[ ] Define failure modes of collectors

[ ] Define failure modes of ingest services

[ ] Define failure modes of replay workers

[ ] Define failure modes of storage tiers

[ ] Define what data loss is acceptable vs unacceptable

[ ] Define health checks for all components

[ ] Define replay worker isolation and crash recovery

[ ] Define ingestion durability guarantees

[ ] Define control plane degradation behavior

---

# 24. SDK / agent developer experience

[ ] Define agent install flow

[ ] Define minimal integration steps for users

[ ] Define instrumentation config format

[ ] Define runtime toggles:

* capture on/off
* strict mode
* payload limits
* redaction rules

[ ] Define debugging visibility for missing hooks

[ ] Define local validation tooling for instrumentation completeness

[ ] Define compatibility matrix by runtime version

---

# 25. Replay sandbox environment

[ ] Define where replays run:

* local dev
* CI worker
* isolated cluster
* dedicated replay environment

[ ] Define network isolation policy

[ ] Define filesystem isolation policy

[ ] Define outbound egress restrictions

[ ] Define artifact loading rules

[ ] Define resource limits for replay workers

[ ] Define replay environment reproducibility guarantees

[ ] Define replay logs/metrics collection

---

# 26. Artifact swapping and version comparison

[ ] Define how candidate builds are supplied to replay

[ ] Define compatibility checks between original execution and candidate build

[ ] Define replay-against-new-build workflow

[ ] Define replay-against-old-build workflow

[ ] Define deploy regression comparison workflow

[ ] Define binary mismatch warnings

[ ] Define source attribution for changed behavior between builds

---

# 27. DB / cache / dependency adapters

[ ] Pick the first DB to support

[ ] Pick the first cache to support

[ ] Pick the first outbound HTTP client patterns to support

[ ] Pick the first queue system to support

[ ] Define adapter interface for all dependency types

[ ] Define read capture format per adapter

[ ] Define write sink format per adapter

[ ] Define unsupported query/payload behavior

[ ] Define streaming and cursor behavior for DB results

[ ] Define partial result-set replay behavior

---

# 28. Multi-service roadmap

[ ] Define how service-to-service edges are correlated

[ ] Define how causal slices are assembled

[ ] Define how partial graph capture is represented

[ ] Define orchestration model for multi-service replay

[ ] Define boundary stubbing between replayed and non-replayed services

[ ] Define eventual consistency modeling approach

[ ] Define cross-service clock consistency policy

[ ] Define cross-service diffing approach

[ ] Define phase criteria for moving from single-service to multi-service

---

# 29. Observability for DRC itself

[ ] Define metrics for capture coverage

[ ] Define metrics for replay success rate

[ ] Define metrics for fidelity levels

[ ] Define metrics for storage amplification

[ ] Define metrics for ingest lag

[ ] Define metrics for agent drop rates

[ ] Define metrics for replay runtime overhead

[ ] Define alerting rules for the DRC system

[ ] Define internal dashboards

---

# 30. Testing strategy

[ ] Define schema validation tests

[ ] Define capture completeness tests

[ ] Define replay fidelity tests

[ ] Define mutation correctness tests

[ ] Define side-effect blocking tests

[ ] Define diff correctness tests

[ ] Define corruption resilience tests

[ ] Define performance tests

[ ] Define load tests for ingestion/storage

[ ] Define chaos tests for partial capture and storage failures

[ ] Define golden-execution tests for regression detection in DRC itself

---

# 31. Replay correctness validation

[ ] Define how to prove a replay matches the original

[ ] Define equality criteria for successful exact replay

[ ] Define acceptable timing variance criteria

[ ] Define structured comparison policies for floating-point / ordering edge cases

[ ] Define confidence downgrade triggers

[ ] Define replay certification workflow for supported stacks

---

# 32. UX requirements

[ ] Define what an execution detail page must show

[ ] Define what a replay setup flow must show

[ ] Define what a diff result must show first

[ ] Define how fidelity/confidence is explained

[ ] Define how unsupported behavior is surfaced clearly

[ ] Define one-click workflows:

* replay exact
* replay with flag change
* replay with candidate build
* replay with dependency timeout

[ ] Define exportable replay reports

---

# 33. Pricing / packaging thinking

[ ] Define metering dimensions:

* executions captured
* GB stored
* replay jobs
* premium fidelity modes

[ ] Define free/low-end boundary

[ ] Define enterprise-only capabilities

[ ] Define self-hosted vs SaaS packaging boundary

[ ] Define features that create expansion revenue:

* more retention
* more runtimes
* multi-service replay
* compliance features

---

# 34. Documentation requirements

[ ] Define architecture docs needed internally

[ ] Define supported runtime docs

[ ] Define instrumentation guide

[ ] Define security whitepaper

[ ] Define replay fidelity doc

[ ] Define known limitations doc

[ ] Define onboarding guide for first customers

---

# 35. MVP cut

[ ] Lock MVP to one runtime

[ ] Lock MVP to one execution type

[ ] Lock MVP to one DB type

[ ] Lock MVP to outbound HTTP only for dependency replay

[ ] Lock MVP to I/O-deterministic replay only

[ ] Lock MVP to exact-request/job replay, no full workflow graph

[ ] Lock MVP to request payload / config / dependency mutation only

[ ] Lock MVP to output diff + first-divergence report only

[ ] Explicitly remove all “later” features from MVP plan

---

# 36. MVP acceptance criteria

[ ] Can capture a real production execution end-to-end

[ ] Can reconstruct replay inputs without live prod dependency access

[ ] Can replay in isolated environment without emitting real side effects

[ ] Can produce same output for supported exact-replay cases

[ ] Can mutate at least one dependency response and show changed result

[ ] Can replay against a candidate build and compare outcome

[ ] Can identify first divergence for supported scenarios

[ ] Can display replay fidelity/confidence clearly

---

# 37. Phase 2 TODO

[ ] Add more dependency adapters

[ ] Add more runtimes

[ ] Add partial snapshot support

[ ] Add richer path diffing

[ ] Add replay in CI flows

[ ] Add deploy regression workflows

[ ] Add queue/job support if request-first in MVP, or vice versa

[ ] Add better hidden-read detection

---

# 38. Phase 3 TODO

[ ] Add multi-service causal slice replay

[ ] Add replay orchestration across services

[ ] Add cross-service divergence view

[ ] Add concurrency-aware capture

[ ] Add happens-before modeling

[ ] Add scheduler mutation experiments

[ ] Add broader artifact bisect workflows

---

# 39. Risks to actively manage

[ ] Hidden uncaptured reads making replay misleading

[ ] Overhead too high for prod use

[ ] Storage costs exploding

[ ] Too much runtime-specific engineering

[ ] Users trusting approximate replay as exact

[ ] Security/privacy concerns blocking adoption

[ ] Side-effect escape in replay environments

[ ] Diff output too noisy to be useful

---

# 40. Final build discipline

[ ] Keep “replayability” as a strict measurable property

[ ] Keep “confidence/fidelity” first-class everywhere

[ ] Keep original executions immutable

[ ] Keep replay side effects blocked by default

[ ] Keep unsupported behavior explicit, never implied

[ ] Keep scope narrow until exact replay works for one real stack

---

# Recommended implementation order

[ ] Finalize scope and fidelity definition

[ ] Finalize event schema and execution model

[ ] Build one runtime agent/SDK

[ ] Build collector + ingest + searchable execution store

[ ] Build replay runtime with virtual time/randomness

[ ] Build DB/outbound dependency replay adapters

[ ] Build side-effect firewall

[ ] Build exact replay flow for one supported stack

[ ] Build mutation spec + mutation application

[ ] Build diff engine + first-divergence locator

[ ] Build fidelity/confidence reporting

[ ] Build control plane workflow around replay + compare

[ ] Harden security, redaction, retention, and isolation

[ ] Only then widen runtime/dependency/service coverage

---

If you want, I can turn this next into a much tighter **founder-style milestone checklist** with sections like “week 1–2,” “month 1,” “month 2–3,” and “what must exist before first customer.”

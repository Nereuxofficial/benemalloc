# Span ownership and reclamation

Small allocations belong to spans owned by one heap. Each heap receives a
monotonically increasing identity; identities are never reused. A span's atomic
owner identity allows the freeing thread to select local or remote return.
Ownership only transfers after the previous owner exits.

The word immediately before a payload identifies its span. Standalone mappings
use a null word in the same position and retain their mapping base and length.
This also distinguishes allocations made during TLS destruction or allocator
reentry, regardless of the requested layout.

## Local and remote returns

Only the owner accesses the span's local intrusive list, outstanding count, and
list links, stored inside UnsafeCell. Remote producers only access immutable
mapping metadata, the owner atomic, and the remote inbox atomic. The code must
never construct an exclusive reference to the entire shared span.

Allocation increments outstanding. Local return decrements it. Remote return
publishes its segment using a release CAS, without decrementing outstanding.
After successful publication the producer must not access the segment or span.
The owner detaches the inbox with an acquire exchange, merges the detached
segments, and decrements outstanding once per collected segment. Producers never
dereference their observed inbox head; the head is only a link/CAS operand.

Outstanding therefore includes live allocations, remote frees in progress, and
published returns not yet collected. Zero permits reclamation after unlinking
the span from the owner's list. No other heap caches segments from that span.

## Collection and exit

An empty local list triggers inbox collection before refill. Every 256 small
allocations, and on explicit BeneAlloc::collect(), the heap adopts the global
orphan list, drains its spans, and releases surplus empty spans. One empty span
per class may remain cached. A thread doing only frees, or an idle owner, may
retain memory until collection or exit; there is no background collector.

On TLS destruction the owner drains each span, releases empty spans, and moves
live spans to the mutex-protected orphan pool. Their inboxes remain valid for
concurrent producers. The next collecting heap takes exclusive ownership of
their local fields. No pointer to destroyed TLS is stored in a span.

TLS reentry is guarded before constructing a mutable heap reference. Allocation
during reentry or after destruction uses standalone mappings; freeing such a
mapping does not require TLS.

## Current tradeoffs

The prefix implementation uses a stride of twice the class size to preserve
class alignment. Thus payload capacity occupies at most half the slot storage,
before accounting for rounding and span metadata. This deliberately simple
layout can later be replaced by aligned-span lookup or a page map.

Classes keep lists of all their spans. A hit at the first span is constant-time;
a miss can scan full spans, and periodic collection scans all owned spans.
Separate available/full lists are a future optimization. Remote returns incur
one CAS attempt when uncontended, with retries under contention.

Tests cover remote publication while the owner collects, outstanding accounting,
surplus-span removal, adoption of live allocations after owner exit, alignment,
reuse, reentry, and allocations after heap TLS destruction. These are runtime
tests, not exhaustive concurrency model checking. Linux is the tested platform.

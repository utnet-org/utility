Observability (o11y) helpers for the Utility codebase.

This crate contains all sorts of utilities to enable a more convenient observability implementation
in the UNC codebase.

The are three infrastructures:

* `tracing`, for structured, hierarchical logging of events (see [`default_subscriber`] function function in particular)
* `metrics` -- convenience wrappers around prometheus metric, for reporting statistics.
* `io-tracer` -- custom infrastructure for observing DB accesses in particular (mostly for parameter estimator)

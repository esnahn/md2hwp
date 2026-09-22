# Product-owned rulesets

Rulesets are versioned, declarative product assets interpreted by closed core
handlers. They are source-controlled policy, not arbitrary user scripts or
template profiles.

The active built-in ruleset is `ast2ir/ir-v0.2.json`, targeting IR 0.2.
The 0.1 ruleset is retained as historical data and is not loaded for new jobs.

Unknown fields, versions, constructors, handlers, and values must fail before
normalization. Distribution may embed the validated ruleset or package it with
an integrity-checked manifest; runtime files never override it implicitly.

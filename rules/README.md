# Product-owned rulesets

Rulesets are versioned, declarative product assets interpreted by closed core
handlers. They are source-controlled policy, not arbitrary user scripts or
template profiles.

Unknown fields, versions, constructors, handlers, and values must fail before
normalization. Distribution may embed the validated ruleset or package it with
an integrity-checked manifest; runtime files never override it implicitly.

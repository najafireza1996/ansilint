//! Ansible-aware model (plays, blocks, tasks, roles) built from the YAML AST.
//!
//! It must not: read files, resolve includes, or evaluate Jinja.
#![forbid(unsafe_code)]

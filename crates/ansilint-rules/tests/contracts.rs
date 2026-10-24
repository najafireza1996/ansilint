//! Contract tests for every registered rule.

use ansilint_rules::{InternalErrorRule, LoadFailureRule, ParserErrorRule, Rule, WarningRule};
use ansilint_testing::rule_contract_tests;

rule_contract_tests!(internal_error_contract, InternalErrorRule);
rule_contract_tests!(load_failure_contract, LoadFailureRule);
rule_contract_tests!(parser_error_contract, ParserErrorRule);
rule_contract_tests!(warning_contract, WarningRule);

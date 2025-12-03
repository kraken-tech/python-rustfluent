use chrono::NaiveDate;
use fluent::FluentArgs;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use miette::{LabeledSpan, miette};
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDict, PyInt, PyList, PyString};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use unic_langid::LanguageIdentifier;

use pyo3::create_exception;

create_exception!(rustfluent, ParserError, pyo3::exceptions::PyException);

/// Helper function to convert byte position to line and column numbers
fn byte_pos_to_line_col(source: &str, byte_pos: usize) -> (usize, usize) {
    let relevant = &source[..byte_pos.min(source.len())];
    let line = relevant.chars().filter(|&c| c == '\n').count() + 1;
    let col = relevant.len() - relevant.rfind('\n').map_or(0, |pos| pos + 1) + 1;
    (line, col)
}

/// Represents a single parsing error with detailed location information
#[pyclass]
#[derive(Clone)]
struct ParseErrorDetail {
    /// Human-readable error message
    #[pyo3(get)]
    message: String,

    /// Line number where the error occurred (1-indexed)
    #[pyo3(get)]
    line: usize,

    /// Column number where the error occurred (1-indexed)
    #[pyo3(get)]
    column: usize,

    /// Byte position where the error starts (0-indexed)
    #[pyo3(get)]
    byte_start: usize,

    /// Byte position where the error ends (0-indexed)
    #[pyo3(get)]
    byte_end: usize,

    /// Optional file path where the error occurred
    #[pyo3(get)]
    filename: Option<String>,
}

#[pymethods]
impl ParseErrorDetail {
    fn __repr__(&self) -> String {
        format!(
            "ParseErrorDetail(message={:?}, line={}, column={}, byte_start={}, byte_end={})",
            self.message, self.line, self.column, self.byte_start, self.byte_end
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref filename) = self.filename {
            format!(
                "{}:{}:{}: {}",
                filename, self.line, self.column, self.message
            )
        } else {
            format!("{}:{}: {}", self.line, self.column, self.message)
        }
    }
}

impl ParseErrorDetail {
    fn from_parser_error(
        error: fluent_syntax::parser::ParserError,
        source: &str,
        filename: Option<String>,
    ) -> Self {
        let (line, column) = byte_pos_to_line_col(source, error.pos.start);
        Self {
            message: error.kind.to_string(),
            line,
            column,
            byte_start: error.pos.start,
            byte_end: error.pos.end,
            filename,
        }
    }
}

/// Represents a validation error found during compile-time checking
#[pyclass]
#[derive(Clone)]
struct ValidationError {
    #[pyo3(get)]
    error_type: String,
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    message_id: Option<String>,
    #[pyo3(get)]
    reference: Option<String>,
}

#[pymethods]
impl ValidationError {
    fn __repr__(&self) -> String {
        format!(
            "ValidationError(type={:?}, message={:?}, message_id={:?})",
            self.error_type, self.message, self.message_id
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref msg_id) = self.message_id {
            format!("{} in '{}': {}", self.error_type, msg_id, self.message)
        } else {
            format!("{}: {}", self.error_type, self.message)
        }
    }
}

/// Represents a format error during message formatting
#[pyclass]
#[derive(Clone)]
struct FormatError {
    #[pyo3(get)]
    error_type: String,
    #[pyo3(get)]
    message: String,

    // Enhanced context fields
    #[pyo3(get)]
    message_id: Option<String>, // Which message had the error

    #[pyo3(get)]
    variable_name: Option<String>, // Which variable (if applicable)

    #[pyo3(get)]
    expected_type: Option<String>, // What type was expected

    #[pyo3(get)]
    actual_type: Option<String>, // What type was provided
}

#[pymethods]
impl FormatError {
    fn __repr__(&self) -> String {
        let mut parts = vec![
            format!("error_type={:?}", self.error_type),
            format!("message={:?}", self.message),
        ];

        if let Some(ref msg_id) = self.message_id {
            parts.push(format!("message_id={:?}", msg_id));
        }
        if let Some(ref var) = self.variable_name {
            parts.push(format!("variable_name={:?}", var));
        }
        if let Some(ref expected) = self.expected_type {
            parts.push(format!("expected_type={:?}", expected));
        }
        if let Some(ref actual) = self.actual_type {
            parts.push(format!("actual_type={:?}", actual));
        }

        format!("FormatError({})", parts.join(", "))
    }

    fn __str__(&self) -> String {
        let mut result = format!("{}: {}", self.error_type, self.message);

        if let Some(ref msg_id) = self.message_id {
            result = format!("{} in '{}'", result, msg_id);
        }
        if let Some(ref var) = self.variable_name {
            result = format!("{} (variable: {})", result, var);
        }
        if self.expected_type.is_some() && self.actual_type.is_some() {
            result = format!(
                "{} (expected {}, got {})",
                result,
                self.expected_type.as_ref().unwrap(),
                self.actual_type.as_ref().unwrap()
            );
        }

        result
    }
}

impl FormatError {
    fn from_fluent_error(error: &fluent_bundle::FluentError) -> Self {
        use fluent_bundle::FluentError as BundleFluentError;
        let error_type = match error {
            BundleFluentError::Overriding { .. } => "Overriding",
            BundleFluentError::ParserError(_) => "ParserError",
            BundleFluentError::ResolverError(_) => "ResolverError",
        };
        Self {
            error_type: error_type.to_string(),
            message: error.to_string(),
            message_id: None,
            variable_name: None,
            expected_type: None,
            actual_type: None,
        }
    }
}

/// Extract all variable references from a pattern
fn extract_variable_references(pattern: &fluent_syntax::ast::Pattern<&str>) -> HashSet<String> {
    let mut vars = HashSet::new();
    collect_vars_from_pattern(pattern, &mut vars);
    vars
}

fn collect_vars_from_pattern(
    pattern: &fluent_syntax::ast::Pattern<&str>,
    vars: &mut HashSet<String>,
) {
    use fluent_syntax::ast;
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            collect_vars_from_expression(expression, vars);
        }
    }
}

fn collect_vars_from_expression(
    expr: &fluent_syntax::ast::Expression<&str>,
    vars: &mut HashSet<String>,
) {
    use fluent_syntax::ast;
    match expr {
        ast::Expression::Inline(inline) => match inline {
            ast::InlineExpression::VariableReference { id } => {
                vars.insert(id.name.to_string());
            }
            ast::InlineExpression::FunctionReference { arguments, .. } => {
                // Check positional args
                for arg in &arguments.positional {
                    collect_vars_from_expression(&ast::Expression::Inline(arg.clone()), vars);
                }
                // Check named args
                for arg in &arguments.named {
                    collect_vars_from_expression(&ast::Expression::Inline(arg.value.clone()), vars);
                }
            }
            ast::InlineExpression::TermReference { arguments, .. } => {
                if let Some(args) = arguments {
                    // Check positional args
                    for arg in &args.positional {
                        collect_vars_from_expression(&ast::Expression::Inline(arg.clone()), vars);
                    }
                    // Check named args
                    for arg in &args.named {
                        collect_vars_from_expression(
                            &ast::Expression::Inline(arg.value.clone()),
                            vars,
                        );
                    }
                }
            }
            _ => {}
        },
        ast::Expression::Select { selector, variants } => {
            // Check selector expression
            collect_vars_from_expression(&ast::Expression::Inline((*selector).clone()), vars);

            // Check all variant values
            for variant in variants {
                collect_vars_from_pattern(&variant.value, vars);
            }
        }
    }
}

fn collect_references(pattern: &Option<fluent_syntax::ast::Pattern<&str>>, refs: &mut Vec<String>) {
    use fluent_syntax::ast;

    if let Some(pattern) = pattern {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                collect_expression_references(expression, refs);
            }
        }
    }
}

fn collect_expression_references(
    expression: &fluent_syntax::ast::Expression<&str>,
    refs: &mut Vec<String>,
) {
    use fluent_syntax::ast;

    match expression {
        ast::Expression::Inline(inline) => match inline {
            ast::InlineExpression::MessageReference { id, .. } => {
                refs.push(id.name.to_string());
            }
            ast::InlineExpression::TermReference { id, .. } => {
                refs.push(format!("-{}", id.name));
            }
            _ => {}
        },
        ast::Expression::Select { selector, variants } => {
            collect_expression_references(&ast::Expression::Inline((*selector).clone()), refs);
            for variant in variants {
                collect_references(&Some(variant.value.clone()), refs);
            }
        }
    }
}

/// Helper struct to hold term information for validation
#[derive(Debug, Clone)]
struct TermInfo {
    attributes: HashSet<String>,
}

/// Collect all term definitions from a resource
fn collect_terms_from_resource(resource: &FluentResource) -> HashMap<String, TermInfo> {
    use fluent_syntax::ast;
    let mut terms = HashMap::new();

    for entry in resource.entries() {
        if let ast::Entry::Term(term) = entry {
            let term_id = format!("-{}", term.id.name);
            let attributes: HashSet<String> = term
                .attributes
                .iter()
                .map(|attr| attr.id.name.to_string())
                .collect();
            terms.insert(term_id, TermInfo { attributes });
        }
    }

    terms
}

/// Helper function to check all references in a resource against the bundle and available terms
fn check_references(
    resource: &FluentResource,
    bundle: &FluentBundle<FluentResource>,
    available_terms: &HashMap<String, TermInfo>,
) -> Vec<ValidationError> {
    use fluent_syntax::ast;
    let mut errors = Vec::new();

    for entry in resource.entries() {
        match entry {
            ast::Entry::Message(msg) => {
                let msg_id = msg.id.name.to_string();
                check_pattern_references(bundle, &msg.value, &msg_id, available_terms, &mut errors);
                for attr in &msg.attributes {
                    check_pattern_references(
                        bundle,
                        &Some(attr.value.clone()),
                        &msg_id,
                        available_terms,
                        &mut errors,
                    );
                }
            }
            ast::Entry::Term(term) => {
                let term_id = format!("-{}", term.id.name);
                check_pattern_references(
                    bundle,
                    &Some(term.value.clone()),
                    &term_id,
                    available_terms,
                    &mut errors,
                );
                for attr in &term.attributes {
                    check_pattern_references(
                        bundle,
                        &Some(attr.value.clone()),
                        &term_id,
                        available_terms,
                        &mut errors,
                    );
                }
            }
            _ => {}
        }
    }

    errors
}

fn check_pattern_references(
    bundle: &FluentBundle<FluentResource>,
    pattern: &Option<fluent_syntax::ast::Pattern<&str>>,
    current_msg_id: &str,
    available_terms: &HashMap<String, TermInfo>,
    errors: &mut Vec<ValidationError>,
) {
    use fluent_syntax::ast;

    if let Some(pattern) = pattern {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                check_expression_references(
                    bundle,
                    expression,
                    current_msg_id,
                    available_terms,
                    errors,
                );
            }
        }
    }
}

fn check_expression_references(
    bundle: &FluentBundle<FluentResource>,
    expression: &fluent_syntax::ast::Expression<&str>,
    current_msg_id: &str,
    available_terms: &HashMap<String, TermInfo>,
    errors: &mut Vec<ValidationError>,
) {
    use fluent_syntax::ast;

    match expression {
        ast::Expression::Inline(inline) => {
            match inline {
                ast::InlineExpression::MessageReference { id, attribute } => {
                    if !bundle.has_message(id.name) {
                        errors.push(ValidationError {
                            error_type: "UnknownMessage".to_string(),
                            message: format!("Unknown message: {}", id.name),
                            message_id: Some(current_msg_id.to_string()),
                            reference: Some(id.name.to_string()),
                        });
                    } else if let Some(attr) = attribute {
                        if let Some(msg) = bundle.get_message(id.name) {
                            if msg.get_attribute(attr.name).is_none() {
                                errors.push(ValidationError {
                                    error_type: "UnknownAttribute".to_string(),
                                    message: format!(
                                        "Unknown attribute: {}.{}",
                                        id.name, attr.name
                                    ),
                                    message_id: Some(current_msg_id.to_string()),
                                    reference: Some(format!("{}.{}", id.name, attr.name)),
                                });
                            }
                        }
                    }
                }
                ast::InlineExpression::TermReference {
                    id,
                    attribute,
                    arguments,
                } => {
                    let term_id = format!("-{}", id.name);

                    // Validate that terms don't receive positional arguments
                    // Per Fluent spec, positional arguments to terms are ignored, so we warn about them
                    if let Some(args) = arguments {
                        if !args.positional.is_empty() {
                            errors.push(ValidationError {
                                error_type: "IgnoredPositionalArgument".to_string(),
                                message: format!(
                                    "Positional arguments passed to term -{} are ignored. Use named arguments instead.",
                                    id.name
                                ),
                                message_id: Some(current_msg_id.to_string()),
                                reference: Some(term_id.clone()),
                            });
                        }
                    }

                    // Check against available_terms instead of bundle.has_message
                    if !available_terms.contains_key(&term_id) {
                        errors.push(ValidationError {
                            error_type: "UnknownTerm".to_string(),
                            message: format!("Unknown term: -{}", id.name),
                            message_id: Some(current_msg_id.to_string()),
                            reference: Some(term_id),
                        });
                    } else if let Some(attr) = attribute {
                        // Check term attributes from available_terms instead of bundle.get_message
                        if let Some(term_info) = available_terms.get(&term_id) {
                            if !term_info.attributes.contains(attr.name) {
                                errors.push(ValidationError {
                                    error_type: "UnknownAttribute".to_string(),
                                    message: format!(
                                        "Unknown attribute on term: -{}.{}",
                                        id.name, attr.name
                                    ),
                                    message_id: Some(current_msg_id.to_string()),
                                    reference: Some(format!("-{}.{}", id.name, attr.name)),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ast::Expression::Select { selector, variants } => {
            check_expression_references(
                bundle,
                &ast::Expression::Inline((*selector).clone()),
                current_msg_id,
                available_terms,
                errors,
            );
            for variant in variants {
                check_pattern_references(
                    bundle,
                    &Some(variant.value.clone()),
                    current_msg_id,
                    available_terms,
                    errors,
                );
            }
        }
    }
}

/// Helper function to detect cycles in message references
fn detect_cycles(resource: &FluentResource) -> Vec<ValidationError> {
    use fluent_syntax::ast;
    use std::collections::{HashMap, HashSet};

    let mut errors = Vec::new();

    // Build a map of message IDs to their referenced IDs
    let mut message_refs: HashMap<String, Vec<String>> = HashMap::new();

    for entry in resource.entries() {
        match entry {
            ast::Entry::Message(msg) => {
                let msg_id = msg.id.name.to_string();
                let mut refs = Vec::new();
                collect_references(&msg.value, &mut refs);
                for attr in &msg.attributes {
                    collect_references(&Some(attr.value.clone()), &mut refs);
                }
                message_refs.insert(msg_id, refs);
            }
            ast::Entry::Term(term) => {
                let term_id = format!("-{}", term.id.name);
                let mut refs = Vec::new();
                collect_references(&Some(term.value.clone()), &mut refs);
                for attr in &term.attributes {
                    collect_references(&Some(attr.value.clone()), &mut refs);
                }
                message_refs.insert(term_id, refs);
            }
            _ => {}
        }
    }

    // Check each message for cycles using DFS
    for (msg_id, _) in &message_refs {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        if has_cycle(msg_id, &message_refs, &mut visited, &mut path) {
            errors.push(ValidationError {
                error_type: "CyclicReference".to_string(),
                message: format!("Cyclic reference detected: {}", path.join(" -> ")),
                message_id: Some(msg_id.clone()),
                reference: None,
            });
        }
    }

    errors
}

fn has_cycle(
    msg_id: &str,
    message_refs: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> bool {
    if visited.contains(msg_id) {
        // Found a cycle - add the current message to show where cycle completes
        path.push(msg_id.to_string());
        return true;
    }

    visited.insert(msg_id.to_string());
    path.push(msg_id.to_string());

    // Check all referenced messages
    if let Some(refs) = message_refs.get(msg_id) {
        for ref_id in refs {
            if has_cycle(ref_id, message_refs, visited, path) {
                return true;
            }
        }
    }

    path.pop();
    visited.remove(msg_id);
    false
}

/// Helper function to create a comprehensive error with all parse and validation errors
fn create_comprehensive_error(
    parse_errors: &[ParseErrorDetail],
    validation_errors: &[ValidationError],
) -> PyErr {
    Python::with_gil(|py| {
        if !parse_errors.is_empty() {
            // If there are parse errors, use ParserError as primary
            // But attach validation errors too!

            let first_file = parse_errors[0]
                .filename
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("<string>");

            // Try to read source for miette display
            let source = std::fs::read_to_string(first_file).ok();

            // Create miette error with labels
            let mut labels = Vec::with_capacity(parse_errors.len());
            for error in parse_errors {
                labels.push(LabeledSpan::at(
                    error.byte_start..error.byte_end,
                    error.message.clone(),
                ));
            }

            let miette_error = if let Some(source) = source {
                miette!(
                    labels = labels,
                    "Found {} parse error(s) and {} validation error(s)",
                    parse_errors.len(),
                    validation_errors.len(),
                )
                .with_source_code(source)
            } else {
                miette!(
                    "Found {} parse error(s) and {} validation error(s) in {}",
                    parse_errors.len(),
                    validation_errors.len(),
                    first_file,
                )
            };

            let err = ParserError::new_err(format!("{miette_error:?}"));

            // Attach structured errors for programmatic access
            if let Ok(exc) = err
                .value(py)
                .downcast::<pyo3::exceptions::PyBaseException>()
            {
                let _ = exc.setattr("parse_errors", parse_errors.to_vec());
                let _ = exc.setattr("validation_errors", validation_errors.to_vec());
                let _ = exc.setattr("error_count", parse_errors.len() + validation_errors.len());
            }

            err
        } else {
            // Only validation errors
            let message = format!(
                "Found {} validation error(s):\n{}",
                validation_errors.len(),
                validation_errors
                    .iter()
                    .map(|e| format!("  - {}", e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let err = PyValueError::new_err(message);

            if let Ok(exc) = err
                .value(py)
                .downcast::<pyo3::exceptions::PyBaseException>()
            {
                let _ = exc.setattr("validation_errors", validation_errors.to_vec());
                let _ = exc.setattr("error_count", validation_errors.len());
            }

            err
        }
    })
}

#[pymodule]
mod rustfluent {
    use super::*;

    #[pymodule_export]
    use super::ParserError;

    #[pymodule_export]
    use super::ParseErrorDetail;

    #[pymodule_export]
    use super::ValidationError;

    #[pymodule_export]
    use super::FormatError;

    #[pyclass]
    struct Bundle {
        bundle: FluentBundle<FluentResource>,
        // Separated compile-time errors by type for clarity
        parse_errors: Vec<ParseErrorDetail>, // Syntax errors from FTL parsing
        validation_errors: Vec<ValidationError>, // Semantic errors (refs, cycles, etc.)
    }

    #[pymethods]
    impl Bundle {
        #[new]
        #[pyo3(signature = (language, ftl_filenames, strict=false, validate_references=true))]
        fn new(
            language: &str,
            ftl_filenames: Vec<PathBuf>,
            strict: bool,
            validate_references: bool,
        ) -> PyResult<Self> {
            let langid: LanguageIdentifier = match language.parse() {
                Ok(langid) => langid,
                Err(_) => {
                    return Err(PyValueError::new_err(format!(
                        "Invalid language: '{language}'"
                    )));
                }
            };
            let mut bundle = FluentBundle::new_concurrent(vec![langid]);
            // Separate error collections by type
            let mut all_parse_errors = Vec::new();
            let mut all_validation_errors = Vec::new();
            // Track all terms across all resources for validation
            let mut all_terms: HashMap<String, TermInfo> = HashMap::new();

            for file_path in ftl_filenames.iter() {
                let contents = fs::read_to_string(file_path)
                    .map_err(|_| PyFileNotFoundError::new_err(file_path.clone()))?;

                let resource = match FluentResource::try_new(contents) {
                    Ok(resource) => resource,
                    Err((resource, errors)) => {
                        // CHANGED: Always collect parse errors (even in strict mode)
                        // Don't raise immediately - collect all errors first
                        let source = resource.source();
                        let filename_str = file_path.to_string_lossy().to_string();

                        // Create structured error details for programmatic access
                        for error in errors {
                            all_parse_errors.push(ParseErrorDetail::from_parser_error(
                                error.clone(),
                                source,
                                Some(filename_str.clone()),
                            ));
                        }

                        // Continue processing to collect more errors
                        resource
                    }
                };

                // Check for duplicates manually before adding
                // Need to detect duplicates both within this file and against existing bundle
                use fluent_syntax::ast;
                use std::collections::HashSet;
                let mut seen_in_file = HashSet::new();

                for entry in resource.entries() {
                    let (kind, id) = match entry {
                        ast::Entry::Message(msg) => ("message", msg.id.name),
                        ast::Entry::Term(term) => ("term", term.id.name),
                        _ => continue,
                    };

                    // Check if this message/term already exists in bundle or was seen in this file
                    let full_id = if kind == "term" {
                        format!("-{}", id)
                    } else {
                        id.to_string()
                    };

                    // For terms, check our term index instead of bundle.has_message
                    let exists_in_bundle = if kind == "term" {
                        all_terms.contains_key(&full_id)
                    } else {
                        bundle.has_message(&full_id)
                    };
                    let exists_in_file = !seen_in_file.insert(full_id.clone());

                    if exists_in_bundle || exists_in_file {
                        let validation_err = ValidationError {
                            error_type: "DuplicateMessageId".to_string(),
                            message: format!(
                                "Duplicate {}: '{}'. Later definition will override.",
                                kind, id
                            ),
                            message_id: Some(id.to_string()),
                            reference: None,
                        };

                        // Don't raise immediately, collect errors
                        all_validation_errors.push(validation_err);
                    }
                }

                // Collect terms from this resource
                let current_resource_terms = collect_terms_from_resource(&resource);

                // Check references and cycles BEFORE adding if validation is enabled
                if validate_references {
                    // Merge current resource terms with all previously seen terms for validation
                    let mut available_terms = all_terms.clone();
                    available_terms.extend(current_resource_terms.clone());

                    // Check if references in this resource exist in current bundle or available terms
                    let ref_errors = check_references(&resource, &bundle, &available_terms);
                    all_validation_errors.extend(ref_errors);

                    // Check for cycles within this resource
                    let cycle_errors = detect_cycles(&resource);
                    all_validation_errors.extend(cycle_errors);
                }

                // Add terms from this resource to the cumulative term index
                all_terms.extend(current_resource_terms);

                // Add the resource (will override duplicates)
                bundle.add_resource_overriding(resource);
            }

            // Check strict mode AFTER collecting all errors
            if strict && (!all_parse_errors.is_empty() || !all_validation_errors.is_empty()) {
                return Err(create_comprehensive_error(
                    &all_parse_errors,
                    &all_validation_errors,
                ));
            }

            Ok(Self {
                bundle,
                parse_errors: all_parse_errors,
                validation_errors: all_validation_errors,
            })
        }

        /// Get all parse errors (syntax errors from FTL parsing)
        fn get_parse_errors(&self) -> Vec<ParseErrorDetail> {
            self.parse_errors.clone()
        }

        /// Get all validation errors (semantic errors: unknown refs, cycles, etc.)
        fn get_validation_errors(&self) -> Vec<ValidationError> {
            self.validation_errors.clone()
        }

        /// Get ALL compile-time errors in one call (parse + validation)
        /// Returns list of tuples: (error_category: str, error: Union[ParseErrorDetail, ValidationError])
        fn get_all_compile_errors(&self, py: Python) -> PyResult<Vec<(String, PyObject)>> {
            let mut all_errors = Vec::new();

            // Add parse errors with "parse" tag
            for err in &self.parse_errors {
                let py_err = Py::new(py, err.clone())?;
                all_errors.push(("parse".to_string(), py_err.into()));
            }

            // Add validation errors with "validation" tag
            for err in &self.validation_errors {
                let py_err = Py::new(py, err.clone())?;
                all_errors.push(("validation".to_string(), py_err.into()));
            }

            Ok(all_errors)
        }

        /// DEPRECATED: Use get_validation_errors() instead
        /// Kept for backward compatibility
        fn get_compile_errors(&self) -> Vec<ValidationError> {
            self.validation_errors.clone()
        }

        /// Get list of variable names used by a specific message
        ///
        /// # Arguments
        /// * `identifier` - Message ID (e.g., "hello" or "hello.attribute")
        ///
        /// # Returns
        /// Sorted list of variable names (e.g., ["count", "user"])
        fn get_required_variables(&self, identifier: &str) -> PyResult<Vec<String>> {
            let get_message = |id: &str| {
                self.bundle
                    .get_message(id)
                    .ok_or_else(|| PyValueError::new_err(format!("{id} not found")))
            };

            let pattern = match identifier.split_once('.') {
                Some((message_id, attribute_id)) => get_message(message_id)?
                    .get_attribute(attribute_id)
                    .ok_or_else(|| {
                        PyValueError::new_err(format!(
                            "Attribute '{attribute_id}' not found on message '{message_id}'"
                        ))
                    })?
                    .value(),
                None => get_message(identifier)?.value().ok_or_else(|| {
                    PyValueError::new_err(format!("{identifier} - Message has no value."))
                })?,
            };

            let vars = extract_variable_references(pattern);
            let mut vars_vec: Vec<String> = vars.into_iter().collect();
            vars_vec.sort(); // Sort for deterministic output

            Ok(vars_vec)
        }

        #[pyo3(signature = (identifier, variables=None, use_isolating=true, errors=None))]
        pub fn get_translation(
            &mut self,
            identifier: &str,
            variables: Option<&Bound<'_, PyDict>>,
            use_isolating: bool,
            errors: Option<&Bound<'_, PyList>>,
        ) -> PyResult<String> {
            self.bundle.set_use_isolating(use_isolating);

            let get_message = |id: &str| {
                self.bundle
                    .get_message(id)
                    .ok_or_else(|| PyValueError::new_err(format!("{id} not found")))
            };

            let pattern = match identifier.split_once('.') {
                Some((message_id, attribute_id)) => get_message(message_id)?
                    .get_attribute(attribute_id)
                    .ok_or_else(|| {
                        PyValueError::new_err(format!(
                            "{identifier} - Attribute '{attribute_id}' not found on message '{message_id}'."
                        ))
                    })?
                    .value(),
                    // Note: attribute.value() returns &Pattern directly (not Option)
                    // because attributes always have values, unlike messages
                None => get_message(identifier)?
                    .value()
                    .ok_or_else(|| {
                        PyValueError::new_err(format!("{identifier} - Message has no value."))
                    })?
            };

            // Extract all variables used by this pattern
            let required_vars = extract_variable_references(pattern);

            let mut args = FluentArgs::new();
            let mut variable_errors = Vec::new();
            let mut provided_vars = HashSet::new();

            if let Some(variables) = variables {
                for (python_key, python_value) in variables {
                    // Make sure the variable key is a Python string,
                    // raising a TypeError if not.
                    if !python_key.is_instance_of::<PyString>() {
                        return Err(PyTypeError::new_err(format!(
                            "Variable key not a str, got {python_key}."
                        )));
                    }
                    let key = python_key.to_string();
                    provided_vars.insert(key.clone());
                    // Set the variable value as a string or integer,
                    // raising a TypeError if not.
                    if python_value.is_instance_of::<PyString>() {
                        args.set(key, python_value.to_string());
                    } else if python_value.is_instance_of::<PyInt>()
                        && let Ok(int_value) = python_value.extract::<i32>()
                    {
                        args.set(key, int_value);
                    } else if python_value.is_instance_of::<PyDate>()
                        && let Ok(chrono_date) = python_value.extract::<NaiveDate>()
                    {
                        args.set(key, chrono_date.format("%Y-%m-%d").to_string());
                    } else {
                        // The variable value was of an unsupported type.
                        // Collect error and fall back to displaying the variable key
                        variable_errors.push(FormatError {
                            error_type: "InvalidVariableType".to_string(),
                            message: format!(
                                "Variable '{}' has unsupported type, expected str/int/date. Using key as fallback.",
                                key
                            ),
                            message_id: Some(identifier.to_string()),
                            variable_name: Some(key.clone()),
                            expected_type: Some("str|int|date".to_string()),
                            actual_type: Some(format!("{:?}", python_value.get_type().name())),
                        });
                        let fallback_value = key.clone();
                        args.set(key, fallback_value);
                    }
                }
            }

            // Check for missing variables
            for required_var in &required_vars {
                if !provided_vars.contains(required_var) {
                    variable_errors.push(FormatError {
                        error_type: "MissingVariable".to_string(),
                        message: format!("Unknown external: {}", required_var),
                        message_id: Some(identifier.to_string()),
                        variable_name: Some(required_var.clone()),
                        expected_type: None,
                        actual_type: None,
                    });
                }
            }

            // Format the message and collect errors
            let mut format_errors = vec![];
            let value = self
                .bundle
                .format_pattern(pattern, Some(&args), &mut format_errors);

            // Convert and append all errors to the provided list
            if let Some(error_list) = errors {
                // Add variable type errors
                for var_err in variable_errors {
                    error_list.append(var_err).ok();
                }

                // Add format errors (cycles, unknown refs, etc.)
                for format_err in format_errors {
                    let py_error = FormatError::from_fluent_error(&format_err);
                    error_list.append(py_error).ok();
                }
            }

            Ok(value.to_string())
        }
    }
}

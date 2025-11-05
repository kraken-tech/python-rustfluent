use chrono::NaiveDate;
use fluent::FluentArgs;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use miette::{LabeledSpan, miette};
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDict, PyInt, PyList, PyString};
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
    }

    #[pymethods]
    impl Bundle {
        #[new]
        #[pyo3(signature = (language, ftl_filenames, strict=false))]
        fn new(language: &str, ftl_filenames: Vec<PathBuf>, strict: bool) -> PyResult<Self> {
            let langid: LanguageIdentifier = match language.parse() {
                Ok(langid) => langid,
                Err(_) => {
                    return Err(PyValueError::new_err(format!(
                        "Invalid language: '{language}'"
                    )));
                }
            };
            let mut bundle = FluentBundle::new_concurrent(vec![langid]);

            for file_path in ftl_filenames.iter() {
                let contents = fs::read_to_string(file_path)
                    .map_err(|_| PyFileNotFoundError::new_err(file_path.clone()))?;

                let resource = match FluentResource::try_new(contents) {
                    Ok(resource) => resource,
                    Err((resource, errors)) if strict => {
                        let mut labels = Vec::with_capacity(errors.len());
                        for error in errors {
                            labels.push(LabeledSpan::at(error.pos, format!("{}", error.kind)))
                        }
                        let error = miette!(
                            labels = labels,
                            "Error when parsing {}",
                            file_path.to_string_lossy()
                        )
                        .with_source_code(resource.source().to_string());
                        return Err(ParserError::new_err(format!("{error:?}")));
                    }
                    Err((resource, _errors)) => resource,
                };
                bundle.add_resource_overriding(resource);
            }

            Ok(Self { bundle })
        }

        #[pyo3(signature = (identifier, variables=None, use_isolating=true))]
        pub fn get_translation(
            &mut self,
            identifier: &str,
            variables: Option<&Bound<'_, PyDict>>,
            use_isolating: bool,
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

            let mut args = FluentArgs::new();

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
                        // Fall back to displaying the variable key as its value.
                        let fallback_value = key.clone();
                        args.set(key, fallback_value);
                    }
                }
            }

            let mut errors = vec![];
            let value = self
                .bundle
                .format_pattern(pattern, Some(&args), &mut errors);
            Ok(value.to_string())
        }
    }
}

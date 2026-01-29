use chrono::NaiveDate;
use fluent::FluentArgs;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use miette::{LabeledSpan, miette};
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDict, PyInt, PyString};
use std::fs;
use std::path::PathBuf;
use unic_langid::LanguageIdentifier;

use pyo3::create_exception;

create_exception!(rustfluent, ParserError, pyo3::exceptions::PyException);

#[pymodule]
mod rustfluent {
    use super::*;

    #[pymodule_export]
    use super::ParserError;

    #[pyclass]
    struct Bundle {
        bundle: FluentBundle<FluentResource>,
    }

    #[pymethods]
    impl Bundle {
        #[new]
        #[pyo3(signature = (language, ftl_filename, strict=false))]
        fn new(language: &str, ftl_filename: PathBuf, strict: bool) -> PyResult<Self> {
            let langid: LanguageIdentifier = match language.parse() {
                Ok(langid) => langid,
                Err(_) => {
                    return Err(PyValueError::new_err(format!(
                        "Invalid language: '{language}'"
                    )));
                }
            };
            let mut bundle = FluentBundle::new_concurrent(vec![langid]);

            let contents = fs::read_to_string(&ftl_filename)
                .map_err(|_| PyFileNotFoundError::new_err(ftl_filename.clone()))?;

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
                        ftl_filename.to_string_lossy()
                    )
                    .with_source_code(resource.source().to_string());
                    return Err(ParserError::new_err(format!("{error:?}")));
                }
                Err((resource, _errors)) => resource,
            };
            bundle.add_resource_overriding(resource);

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

use chrono::NaiveDate;
use fluent::FluentArgs;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDict, PyInt, PyList, PyString};
use std::fs;
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
        #[pyo3(signature = (language, ftl_filenames, strict=false))]
        fn new(
            language: &str,
            ftl_filenames: &'_ Bound<'_, PyList>,
            strict: bool,
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

            for file_path in ftl_filenames.iter() {
                let path_string = file_path.to_string();
                let contents = fs::read_to_string(path_string)
                    .map_err(|_| PyFileNotFoundError::new_err(file_path.to_string()))?;

                let resource = match FluentResource::try_new(contents) {
                    Ok(resource) => resource,
                    Err(_) if strict => {
                        return Err(ParserError::new_err(format!(
                            "Error when parsing {file_path}."
                        )));
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

            let msg = self
                .bundle
                .get_message(identifier)
                .ok_or_else(|| (PyValueError::new_err(format!("{identifier} not found"))))?;

            let pattern = msg.value().ok_or_else(|| {
                PyValueError::new_err(format!("{identifier} - Message has no value.",))
            })?;

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

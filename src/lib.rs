use fluent::FluentArgs;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyList, PyString};
use std::fs;
use unic_langid::LanguageIdentifier;

use pyo3::create_exception;

create_exception!(rustfluent, PyParserError, pyo3::exceptions::PyException);

#[pymodule]
fn rustfluent(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Bundle>()?;
    m.add("ParserError", m.py().get_type_bound::<PyParserError>())?;
    Ok(())
}

#[pyclass]
struct Bundle {
    bundle: FluentBundle<FluentResource>,
}

#[pymethods]
impl Bundle {
    #[new]
    #[pyo3(signature = (language, ftl_filenames, strict=false))]
    fn new(language: &str, ftl_filenames: &'_ Bound<'_, PyList>, strict: bool) -> PyResult<Self> {
        let langid: LanguageIdentifier = language.parse().expect("Parsing failed");
        let mut bundle = FluentBundle::new_concurrent(vec![langid]);

        for file_path in ftl_filenames.iter() {
            let path_string = file_path.to_string();
            let contents = match fs::read_to_string(path_string) {
                Ok(contents) => contents,
                Err(_) => return Err(PyFileNotFoundError::new_err(file_path.to_string())),
            };

            let resource = match FluentResource::try_new(contents) {
                Ok(resource) => resource,
                Err(error) => {
                    if strict {
                        return Err(PyParserError::new_err(format!(
                            "Error when parsing {}.",
                            file_path
                        )));
                    } else {
                        // The first element of the error is the parsed resource, minus any
                        // invalid messages.
                        error.0
                    }
                }
            };
            bundle.add_resource_overriding(resource);
        }

        Ok(Self { bundle })
    }

    #[pyo3(signature = (identifier, variables=None))]
    pub fn get_translation(
        &self,
        identifier: &str,
        variables: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        let msg = match self.bundle.get_message(identifier) {
            Some(m) => m,
            None => return Err(PyValueError::new_err(format!("{} not found", identifier))),
        };

        let mut errors = vec![];
        let pattern = match msg.value() {
            Some(m) => m,
            None => {
                return Err(PyValueError::new_err(format!(
                    "{} - Message has no value.",
                    identifier
                )))
            }
        };

        let mut args = FluentArgs::new();

        if let Some(variables) = variables {
            for variable in variables {
                // Make sure the variable key is a Python string,
                // raising a TypeError if not.
                let python_key = variable.0;
                if !python_key.is_instance_of::<PyString>() {
                    return Err(PyTypeError::new_err(format!(
                        "Variable key not a str, got {}.",
                        python_key
                    )));
                }
                let key = python_key.to_string();
                // Set the variable value as a string or integer,
                // raising a TypeError if not.
                let python_value = variable.1;
                if python_value.is_instance_of::<PyString>() {
                    args.set(key, python_value.to_string());
                } else if python_value.is_instance_of::<PyInt>() {
                    match python_value.extract::<i32>() {
                        Ok(int_value) => {
                            args.set(key, int_value);
                        }
                        _ => {
                            // The Python integer overflowed i32.
                            // Fall back to displaying the variable key as its value.
                            let fallback_value = key.clone();
                            args.set(key, fallback_value);
                        }
                    }
                } else {
                    // The variable value was of an unsupported type.
                    // Fall back to displaying the variable key as its value.
                    let fallback_value = key.clone();
                    args.set(key, fallback_value);
                }
            }
        }
        let value = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors);
        Ok(value.to_string())
    }
}

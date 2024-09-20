use fluent::FluentArgs;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::fs;
use unic_langid::LanguageIdentifier;

#[pymodule]
fn rustfluent(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Bundle>()?;
    Ok(())
}

#[pyclass]
struct Bundle {
    bundle: FluentBundle<FluentResource>,
}

#[pymethods]
impl Bundle {
    #[new]
    fn new(language: &str, ftl_filenames: &'_ Bound<'_, PyList>) -> PyResult<Self> {
        let langid: LanguageIdentifier = language.parse().expect("Parsing failed");
        let mut bundle = FluentBundle::new_concurrent(vec![langid]);

        for file_path in ftl_filenames.iter() {
            let path_string = file_path.to_string();
            let contents = match fs::read_to_string(path_string) {
                Ok(contents) => contents,
                Err(_) => return Err(PyFileNotFoundError::new_err(file_path.to_string())),
            };

            let res = match FluentResource::try_new(contents) {
                Ok(res) => res,
                Err(error) => {
                    return Err(PyValueError::new_err(format!(
                        "{error:?} - Fluent file contains errors"
                    )))
                }
            };

            bundle.add_resource_overriding(res);
        }

        Ok(Self { bundle })
    }

    #[pyo3(signature = (identifier, **kwargs))]
    pub fn get_translation(
        &self,
        identifier: &str,
        kwargs: Option<&Bound<'_, PyDict>>,
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

        if let Some(kwargs) = kwargs {
            for kwarg in kwargs {
                args.set(kwarg.0.to_string(), kwarg.1.to_string());
            }
        }

        let value = self
            .bundle
            .format_pattern(&pattern, Some(&args), &mut errors);
        Ok(value.to_string())
    }
}

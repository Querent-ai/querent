use querent_synapse::querent::py_runtime::py_runtime_init;

fn main() -> pyo3::PyResult<()> {
	match py_runtime_init() {
		Ok(_) => {
			println!("Python runtime initialized.");
		},
		Err(e) =>
			return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
				"Failed to initialize Python runtime: {}",
				e
			))),
	}
	Ok(())
}

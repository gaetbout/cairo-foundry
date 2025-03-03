use std::{fmt::Display, io::Write, path::PathBuf, str::from_utf8};

use clap::{Args, ValueHint};
use log::error;
use serde::Serialize;

use super::CommandExecution;
use cairo_rs::{
	cairo_run::cairo_run,
	hint_processor::{
		builtin_hint_processor::{
			builtin_hint_processor_definition::{BuiltinHintProcessor, HintFunc},
			hint_utils::get_integer_from_var_name,
		},
		hint_processor_definition::HintReference,
		proxies::{exec_scopes_proxy::ExecutionScopesProxy, vm_proxy::VMProxy},
	},
	serde::deserialize_program::ApTracking,
	vm::errors::vm_errors::VirtualMachineError,
};
use std::collections::HashMap;

#[derive(Args, Debug)]
pub struct ExecuteArgs {
	/// Path to a json compiled cairo program
	#[clap(short, long, value_hint=ValueHint::FilePath, value_parser=is_json)]
	program: PathBuf,
}

fn is_json(path: &str) -> Result<PathBuf, String> {
	let path = PathBuf::from(path);
	if path.exists() && path.is_file() {
		match path.extension() {
			Some(ext) if ext == "json" => Ok(path),
			_ => Err(format!("\"{}\" is not a json file", path.display())),
		}
	} else {
		Err(format!("\"{}\" is not a valid file", path.display()))
	}
}

/// Execute command output
#[derive(Debug, Serialize)]
pub struct ExecuteOutput(Vec<u8>);

impl Write for ExecuteOutput {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.0.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.0.flush()
	}
}

impl Display for ExecuteOutput {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			from_utf8(&self.0).map_err(|e| {
				error!("failed to format the execution output due to invalid utf8 encodig: {e}");
				std::fmt::Error
			})?
		)
	}
}

impl CommandExecution<ExecuteOutput> for ExecuteArgs {
	fn exec(&self) -> Result<ExecuteOutput, String> {
		let hint = HintFunc(Box::new(greater_than_hint));
		let mut hint_processor = BuiltinHintProcessor::new_empty();
		hint_processor.add_hint(String::from("print(ids.a > ids.b)"), hint);

		let mut cairo_runner =
			cairo_run(&self.program, "main", false, &hint_processor).map_err(|e| {
				format!(
					"failed to run the program \"{}\": {}",
					self.program.display(),
					e,
				)
			})?;

		let mut output = ExecuteOutput(vec![]);

		cairo_runner.write_output(&mut output).map_err(|e| {
			format!(
				"failed to print the program output \"{}\": {}",
				self.program.display(),
				e,
			)
		})?;

		Ok(output)
	}
}

fn greater_than_hint(
	vm_proxy: &mut VMProxy,
	_exec_scopes_proxy: &mut ExecutionScopesProxy,
	ids_data: &HashMap<String, HintReference>,
	ap_tracking: &ApTracking,
) -> Result<(), VirtualMachineError> {
	let a = get_integer_from_var_name("a", vm_proxy, ids_data, ap_tracking)?;
	let b = get_integer_from_var_name("b", vm_proxy, ids_data, ap_tracking)?;
	println!("{}", a > b);
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn valid_programs() {
		assert!(
			ExecuteArgs {
				program: PathBuf::from(
					"./test_starknet_projects/compiled_programs/valid_program_a.json"
				),
			}
			.exec()
			.is_ok()
		);

		assert!(
			ExecuteArgs {
				program: PathBuf::from(
					"./test_starknet_projects/compiled_programs/valid_program_b.json"
				),
			}
			.exec()
			.is_ok()
		);

		assert!(
			ExecuteArgs {
				program: PathBuf::from("./test_starknet_projects/hint_assertion/custom_hint.json"),
			}
			.exec()
			.is_ok()
		);
	}

	#[test]
	fn invalid_programs() {
		assert!(
			ExecuteArgs {
				program: PathBuf::from(
					"./test_starknet_projects/compiled_programs/invalid_odd_length_hex.json"
				),
			}
			.exec()
			.is_err()
		);

		assert!(
			ExecuteArgs {
				program: PathBuf::from(
					"./test_starknet_projects/compiled_programs/invalid_even_length_hex.json"
				),
			}
			.exec()
			.is_err()
		);
	}
}

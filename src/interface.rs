use clap::{App, AppSettings, Arg, SubCommand};

use crate::FilePath;

pub fn interface() -> crate::GenericResult {
	let matches = App::new("lucent")
		.version("version: Hello, refactor!")
		.about("Lucent language compiler")
		.setting(AppSettings::SubcommandRequiredElseHelp)
		.subcommand(SubCommand::with_name("build")
			.about("Builds a Lucent project")
			.arg(Arg::with_name("root")
				.takes_value(true).required(true)
				.help("root file of the project")))
		.subcommand(SubCommand::with_name("server")
			.about("Starts the Lucent language server"))
		.get_matches();

	Ok(match matches.subcommand() {
		("server", _) => crate::server::server()?,
		("build", Some(matches)) => {
			let path = matches.value_of_os("root").unwrap();
			let path = FilePath::from(path.to_os_string());
			crate::compile::compile(path).unwrap();
		}
		_ => (),
	})
}

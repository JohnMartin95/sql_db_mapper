use sql_db_mapper::Opt;
use structopt::StructOpt;

fn main() {
	let opt = Opt::from_args();

	let mut client = opt.get_client();
	let full_db = client.get_all(opt.no_functions);

	full_db.make_output(&opt);
}

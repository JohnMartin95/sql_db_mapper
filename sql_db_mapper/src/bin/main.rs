use structopt::StructOpt;
use sql_db_mapper::{
	Opt,
};

fn main() {
	let opt = Opt::from_args();

	let mut client = opt.get_client();
	let full_db = client.get_all();

	full_db.make_output(&opt);
}

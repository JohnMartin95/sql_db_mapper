use structopt::StructOpt;
use postgres::{Client, NoTls};
use sql_db_mapper::{
	connection::*,
	Opt,
};

fn main() {
	let opt = Opt::from_args();

	let conn = Client::connect(&opt.conn, NoTls).expect("Failed to connect to database, please check your connection string and try again");

	let mut conn = MyClient::new(conn);
	let full_db = conn.get_all();

	full_db.make_output(&opt);
}

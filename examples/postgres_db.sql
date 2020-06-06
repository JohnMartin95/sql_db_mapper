
DROP SCHEMA public CASCADE;
DROP SCHEMA people CASCADE;
DROP SCHEMA things CASCADE;
DROP SCHEMA other CASCADE;

-- create schemas
CREATE SCHEMA public
	AUTHORIZATION postgres;

COMMENT ON SCHEMA public
	IS 'standard public schema';

GRANT ALL ON SCHEMA public TO PUBLIC;
GRANT ALL ON SCHEMA public TO postgres;

CREATE SCHEMA people
	AUTHORIZATION postgres;
GRANT ALL ON SCHEMA people TO postgres;

CREATE SCHEMA things
	AUTHORIZATION postgres;
GRANT ALL ON SCHEMA things TO postgres;

CREATE SCHEMA other
	AUTHORIZATION postgres;
GRANT ALL ON SCHEMA other TO postgres;

--people tables
CREATE TABLE people.people(
	name VARCHAR(64) NOT NULL,
	birthday DATE,

	PRIMARY KEY (name)
);

CREATE TABLE people.employees(
	id INTEGER NOT NULL,
	name VARCHAR(64) NOT NULL,
	is_manager BOOLEAN NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (name) REFERENCES people.people(name)
);

CREATE TABLE people.customers(
	name VARCHAR(64) NOT NULL,

	FOREIGN KEY (name) REFERENCES people.people(name)
);

CREATE TYPE things.item_type AS ENUM(
	'car',
	'computer',
	'book'
);

CREATE TABLE things.inventory(
	id INT4 NOT NULL,
	count_in_stock smallint NOT NULL,
	item_type things.item_type,
	price money,
	price2 NUMERIC,
	price3 NUMERIC(7,3),

	PRIMARY KEY (id)
);


CREATE TYPE other.type_testing AS (
	t_bigint bigint,
	-- t_bigserial bigserial,
	-- t_bit bit(10),
	-- t_varbit varbit(10),
	t_boolean boolean,
	-- t_box box,
	t_bytea bytea,
	t_character character(10),
	t_varchar varchar(10),
	t_cidr cidr,
	-- t_circle circle,
	t_date date,
	t_float8 float8,
	t_inet inet,
	t_integer integer,
	t_interval interval,
	t_json json,
	-- t_jsonb jsonb,
	-- t_line line,
	-- t_lseg lseg,
	t_macaddr macaddr,
	t_macaddr8 macaddr8,
	-- t_money money,
	t_numeric numeric(5,5),
	-- t_path path,
	-- t_pg_lsn pg_lsn,
	-- t_point point,
	-- t_polygon polygon,
	t_real real,
	t_smallint smallint,
	-- t_smallserial smallserial,
	-- t_serial serial,
	t_text text,
	t_time time without time zone,
	t_timetz time with time zone,
	t_timestamp timestamp without time zone,
	t_timestamptz timestamp with time zone,
	-- t_tsquery tsquery,
	-- t_tsvector tsvector,
	-- t_txid_snapshot txid_snapshot,
	t_uuid uuid,
	t_xml xml
);

CREATE TYPE other.array_test AS (
	some_ints integer[5]
);

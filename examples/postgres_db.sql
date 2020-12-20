
DROP SCHEMA IF EXISTS public, people, things, other CASCADE;

-- create schemas
CREATE SCHEMA public AUTHORIZATION postgres;
COMMENT ON SCHEMA public IS 'standard public schema';

CREATE SCHEMA people AUTHORIZATION postgres;
CREATE SCHEMA things AUTHORIZATION postgres;
CREATE SCHEMA other AUTHORIZATION postgres;

GRANT ALL ON SCHEMA public TO PUBLIC;
GRANT ALL ON SCHEMA public, people, things, other TO postgres;

--people tables
CREATE TABLE people.people(
	name VARCHAR(64) NOT NULL,
	birthday DATE,

	PRIMARY KEY (name)
);

CREATE TABLE people.employees(
	id SERIAL,
	name VARCHAR(64) NOT NULL,
	is_manager BOOLEAN NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (name) REFERENCES people.people(name)
);

CREATE TABLE people.customers(
	name VARCHAR(64) NOT NULL,

	PRIMARY KEY (name),
	FOREIGN KEY (name) REFERENCES people.people(name)
);

CREATE TYPE things.item_type AS ENUM(
	'car',
	'computer',
	'book'
);

CREATE TABLE things.inventory(
	id SERIAL,
	count_in_stock SMALLINT NOT NULL,
	item_type things.item_type,
	--price MONEY,
	price2 NUMERIC,
	price3 NUMERIC(7,3),

	PRIMARY KEY (id)
);

CREATE TABLE things.order_history(
	customer VARCHAR(64) NOT NULL,
	item_bought INTEGER NOT NULL,
	date_purchased DATE,

	PRIMARY KEY (customer, item_bought),
	FOREIGN KEY (customer) REFERENCES people.people(name),
	FOREIGN KEY (item_bought) REFERENCES things.inventory(id)
);

-- Simple test of extended types
CREATE TYPE other.type_testing AS (
	t_bigint bigint,
	-- t_bit bit(10),
	-- t_varbit varbit(10),
	t_boolean boolean,
	-- t_box box,
	t_bytea bytea,
	t_character character(10),
	t_varchar varchar(10),
	-- t_cidr cidr,
	-- t_circle circle,
	t_date date,
	-- t_float8 float8,
	-- t_inet inet,
	t_integer integer,
	t_interval interval,
	-- t_json json,
	-- t_jsonb jsonb,
	-- t_line line,
	-- t_lseg lseg,
	-- t_macaddr macaddr,
	-- t_macaddr8 macaddr8,
	-- t_money money,
	t_numeric numeric(5,5),
	-- t_path path,
	-- t_pg_lsn pg_lsn,
	-- t_point point,
	-- t_polygon polygon,
	t_real real,
	t_smallint smallint,
	t_text text,
	t_time time without time zone,
	-- t_timetz time with time zone,
	t_timestamp timestamp without time zone,
	t_timestamptz timestamp with time zone,
	-- t_tsquery tsquery,
	-- t_tsvector tsvector,
	-- t_txid_snapshot txid_snapshot,
	t_uuid uuid
	--t_xml xml
);
CREATE TABLE other.type_testing2(
	t_bigserial bigserial,
	t_serial serial,
	t_smallserial smallserial
);


CREATE TYPE other.array_test AS (
	some_ints integer[5]
);
CREATE TABLE other.array_test2(
	some_ints integer[5] NOT NULL,
	text0 text NOT NULL,
	text1 text[] NOT NULL,
	text2 text[][] NOT NULL,
	text3 text[][][] NOT NULL,
	ignored_size text[1][2][3] NOT NULL,

	PRIMARY KEY (some_ints)
);

CREATE FUNCTION people.new_person(name text, birthday DATE)  RETURNS void
LANGUAGE SQL
AS $$
INSERT INTO people.people(name, birthday) VALUES (name, birthday);
$$;

CREATE FUNCTION people.add3(x integer)  RETURNS integer
AS $$
BEGIN
	RETURN x + 3;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION people.add3(x integer, y integer)  RETURNS integer
AS $$
BEGIN
RETURN x - y + 3;
END;
$$ LANGUAGE plpgsql;




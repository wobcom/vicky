-- Your SQL goes here

CREATE TABLE "users"(
	"sub" UUID NOT NULL PRIMARY KEY,
	"name" VARCHAR NOT NULL,
	"role" VARCHAR NOT NULL
);


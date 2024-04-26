CREATE TABLE tasks
(
    id             uuid PRIMARY KEY,
    display_name   VARCHAR,
    status         VARCHAR,
    flake_ref_uri  VARCHAR,
    flake_ref_args VARCHAR
);

CREATE TABLE locks
(
    id      INT,
    task_id uuid,
    name    VARCHAR NOT NULL,
    type    VARCHAR NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT fk_task
        FOREIGN KEY (task_id)
            REFERENCES tasks (id)
);

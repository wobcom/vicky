CREATE TABLE tasks
(
    id             uuid    PRIMARY KEY,
    display_name   VARCHAR NOT NULL,
    status         VARCHAR NOT NULL,
    features       VARCHAR NOT NULL,
    flake_ref_uri  VARCHAR NOT NULL,
    flake_ref_args VARCHAR NOT NULL
);

CREATE TABLE locks
(
    id      SERIAL  PRIMARY KEY,
    task_id uuid    NOT NULL,
    name    VARCHAR NOT NULL,
    type    VARCHAR NOT NULL,
    CONSTRAINT fk_task
        FOREIGN KEY (task_id)
            REFERENCES tasks (id)
);

CREATE TABLE task_templates
(
    id                      UUID PRIMARY KEY,
    name                    VARCHAR                  NOT NULL UNIQUE,
    display_name_template   VARCHAR                  NOT NULL,
    flake_ref_uri_template  VARCHAR                  NOT NULL,
    flake_ref_args_template TEXT[]                   NOT NULL,
    features                TEXT[]                   NOT NULL,
    "group"                 VARCHAR,
    created_at              TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE task_template_locks
(
    id               UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    task_template_id UUID            NOT NULL,
    name_template    VARCHAR         NOT NULL,
    type             "LockKind_Type" NOT NULL,
    CONSTRAINT fk_task_template
        FOREIGN KEY (task_template_id)
            REFERENCES task_templates (id)
            ON DELETE CASCADE
);

CREATE TABLE task_template_variables
(
    id               UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    task_template_id UUID    NOT NULL,
    name             VARCHAR NOT NULL,
    default_value    VARCHAR,
    description      VARCHAR,
    CONSTRAINT fk_task_template_var
        FOREIGN KEY (task_template_id)
            REFERENCES task_templates (id)
            ON DELETE CASCADE,
    CONSTRAINT task_template_variable_unique
        UNIQUE (task_template_id, name)
);

CREATE INDEX task_template_locks_template_id_idx ON task_template_locks (task_template_id);
CREATE INDEX task_template_variables_template_id_idx ON task_template_variables (task_template_id);

-- Your SQL goes here

create table task_groups
(
    id         uuid                     default uuid_generate_v4() not null   primary key,
    name       varchar                                             not null,
    created_at timestamp with time zone default now()              not null
);

INSERT INTO task_groups(name)
    SELECT DISTINCT "group" from tasks WHERE "group" IS NOT NULL;

ALTER TABLE tasks
    ADD COLUMN "group_id" uuid;

ALTER TABLE tasks
    ADD CONSTRAINT fk_task FOREIGN KEY(group_id) REFERENCES task_groups(id);


UPDATE tasks t
    SET group_id = tg.id
    FROM task_groups tg WHERE t.group = tg.name;

ALTER TABLE tasks
    DROP COLUMN "group";
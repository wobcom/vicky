-- This file should undo anything in `up.sql`



ALTER TABLE tasks
    ADD COLUMN "group" VARCHAR;

UPDATE tasks
    SET "group" = tg.name
    FROM task_groups tg
    WHERE group_id = tg.id;
    
ALTER TABLE tasks
    DROP COLUMN group_id;

DROP TABLE task_groups;

-- Your SQL goes here


INSERT INTO task_groups(name)
    SELECT DISTINCT CONCAT('Group of ', "display_name") as "group" from tasks WHERE "group_id" IS NULL;

UPDATE tasks t
    SET group_id = tg.id
    FROM task_groups tg WHERE CONCAT('Group of ', t."display_name") = tg.name;


alter table tasks
    alter column group_id set not null;


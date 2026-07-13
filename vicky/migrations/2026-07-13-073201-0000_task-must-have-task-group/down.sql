-- This file should undo anything in `up.sql`

alter table tasks
    alter column group_id drop not null;
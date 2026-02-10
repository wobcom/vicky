-- This file should undo anything in `up.sql`

-- can't drop enum values from an enum.
CREATE TYPE "TaskStatus_Type_New" AS ENUM (
    'NEW',
    'NEEDS_USER_VALIDATION',
    'RUNNING',
    'FINISHED::SUCCESS',
    'FINISHED::ERROR',
    'FINSIHED::TIMEOUT'
);

UPDATE tasks SET status = 'FINISHED::ERROR' WHERE status = 'FINISHED::CANCEL';

ALTER TABLE tasks
    ALTER COLUMN status TYPE "TaskStatus_Type_New"
        USING (status::text::"TaskStatus_Type_New");

DROP TYPE "TaskStatus_Type";

ALTER TYPE "TaskStatus_Type_New" RENAME TO "TaskStatus_Type";
ALTER TABLE tasks
    DROP COLUMN "last_heartbeat";

-- can't drop enum values from an enum.
CREATE TYPE "TaskStatus_Type_New" AS ENUM (
    'NEW',
    'NEEDS_USER_VALIDATION',
    'RUNNING',
    'FINISHED::SUCCESS',
    'FINISHED::ERROR'
);

UPDATE tasks SET status = 'FINISHED::ERROR' WHERE status = 'FINISHED::TIMEOUT';

ALTER TABLE tasks
    ALTER COLUMN status TYPE "TaskStatus_Type_New"
        USING (status::text::"TaskStatus_Type_New");

DROP TYPE "TaskStatus_Type";

ALTER TYPE "TaskStatus_Type_New" RENAME TO "TaskStatus_Type";
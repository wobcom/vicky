-- This file should undo anything in `up.sql`

-- can't drop enum values from an enum.
CREATE TYPE "LockKind_Type_New" AS ENUM (
    'READ',
    'WRITE',
);

DELETE FROM locks WHERE type = 'CLEAN';

ALTER TABLE tasks
    ALTER COLUMN status TYPE "LockKind_Type_New"
        USING (status::text::"LockKind_Type_New");

DROP TYPE "LockKind_Type";

ALTER TYPE "LockKind_Type_New" RENAME TO "LockKind_Type";
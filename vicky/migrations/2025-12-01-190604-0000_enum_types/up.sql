CREATE TYPE "LockKind_Type" AS ENUM ('READ', 'WRITE');
CREATE TYPE "TaskStatus_Type" AS ENUM (
    'NEW',
    'NEEDS_USER_VALIDATION',
    'RUNNING',
    'FINISHED::SUCCESS',
    'FINISHED::ERROR'
);
CREATE TYPE "Role_Type" AS ENUM ('admin');

ALTER TABLE users
    RENAME COLUMN sub TO id;

ALTER TABLE locks
    ALTER COLUMN type TYPE "LockKind_Type" USING upper("type")::"LockKind_Type";

ALTER TABLE tasks
    ALTER COLUMN status TYPE "TaskStatus_Type" USING upper(status)::"TaskStatus_Type";

ALTER TABLE users
    ALTER COLUMN role TYPE "Role_Type" USING lower(role)::"Role_Type";

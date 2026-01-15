ALTER TABLE tasks
    ADD COLUMN "last_heartbeat" timestamp;

ALTER TYPE "TaskStatus_Type" ADD VALUE 'FINISHED::TIMEOUT';

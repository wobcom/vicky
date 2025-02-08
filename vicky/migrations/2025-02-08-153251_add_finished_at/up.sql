ALTER TABLE tasks
ADD COLUMN finished_at timestamp DEFAULT now();

ALTER TABLE tasks
ALTER COLUMN finished_at DROP DEFAULT;
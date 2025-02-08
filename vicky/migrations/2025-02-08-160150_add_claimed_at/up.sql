ALTER TABLE tasks
ADD COLUMN claimed_at timestamp DEFAULT now();

ALTER TABLE tasks
ALTER COLUMN claimed_at DROP DEFAULT;
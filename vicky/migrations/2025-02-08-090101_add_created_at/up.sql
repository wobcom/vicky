ALTER TABLE tasks
ADD COLUMN created_at timestamp not null DEFAULT now();

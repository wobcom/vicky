ALTER TABLE locks
ADD COLUMN poisoned_by_task uuid;

ALTER TABLE locks
ADD FOREIGN KEY (poisoned_by_task)
    REFERENCES tasks(id)


CREATE TABLE pixurs_new (
    id INTEGER PRIMARY KEY NOT NULL,

    average_color INTEGER NOT NULL,
    thumbs_id INTEGER NOT NULL,

    FOREIGN KEY (thumbs_id) REFERENCES thumbs(id)
);

INSERT INTO pixurs_new SELECT
    id,
    average_color,
    thumbs_id
FROM pixurs;

DROP TABLE pixurs;
ALTER TABLE pixurs_new RENAME TO pixurs;

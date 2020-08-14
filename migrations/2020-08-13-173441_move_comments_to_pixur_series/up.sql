COMMIT;
PRAGMA foreign_keys=off;
BEGIN TRANSACTION;

ALTER TABLE pixur_series
    ADD comment TEXT NULL;

ALTER TABLE pixur_series
    ADD comment_position TEXT NOT NULL
        DEFAULT "bottom"
        CHECK (comment_position IN ("top", "center", "bottom"));

UPDATE pixur_series
    SET (comment, comment_position) = (
        SELECT comment, comment_position
            FROM pixurs
            WHERE pixur_series.pixurs_id = pixurs.id
    );

-- All the rest is just a matter of removing comment and comment_position from the pixurs table

CREATE TEMPORARY TABLE pixurs_tmp AS
    SELECT id, average_color, thumbs_id,
        created, image_aspect_ratio,
        crop_left, crop_right, crop_top, crop_bottom
    FROM pixurs;

DROP TABLE pixurs;

CREATE TABLE pixurs (
    id INTEGER PRIMARY KEY NOT NULL,

    average_color INTEGER NOT NULL,
    thumbs_id INTEGER NOT NULL,

    created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    image_aspect_ratio REAL NOT NULL,

    crop_left REAL NOT NULL,
    crop_right REAL NOT NULL,
    crop_top REAL NOT NULL,
    crop_bottom REAL NOT NULL,

    FOREIGN KEY (thumbs_id) REFERENCES thumbs(id),

    CHECK (image_aspect_ratio > 0),

    CHECK (0 <= crop_left),
    CHECK (crop_left <= crop_right),
    CHECK (crop_right <= 1),

    CHECK (0 <= crop_top),
    CHECK (crop_top <= crop_bottom),
    CHECK (crop_bottom <= 1)
);

INSERT INTO pixurs SELECT * FROM pixurs_tmp;

COMMIT;
PRAGMA foreign_keys=on;
BEGIN TRANSACTION;

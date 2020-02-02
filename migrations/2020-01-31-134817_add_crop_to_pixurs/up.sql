-- Create the new before renaming the old, otherwise the foreign keys pointing
-- into this table would follow along to the renamed table

CREATE TABLE pixurs_new (
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

INSERT INTO pixurs_new SELECT
    pixurs.id AS id,
    average_color,
    thumbs_id,
    CURRENT_TIMESTAMP AS created,
    CAST(width AS REAL) / height AS image_aspect_ratio,
    0.5 AS crop_left,
    0.5 AS crop_right,
    0.5 AS crop_top,
    0.5 AS crop_bottom
FROM pixurs INNER JOIN images_meta ON pixurs.id = images_meta.pixurs_id;

DROP TABLE pixurs;
ALTER TABLE pixurs_new RENAME TO pixurs;

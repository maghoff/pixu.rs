-- Separate table for thumbnails: Similar sized BLOBs are easier for SQLite,
-- thumbnailsized BLOBs in SQLite are actually faster than the filesystem
CREATE TABLE thumbs (
    id INTEGER PRIMARY KEY NOT NULL,

    media_type TEXT NOT NULL,
    data BLOB NOT NULL
);

CREATE TABLE pixurs (
    id INTEGER PRIMARY KEY NOT NULL,

    average_color INTEGER NOT NULL,
    thumbs_id INTEGER NOT NULL,

    FOREIGN KEY (thumbs_id) REFERENCES thumbs(id)
);

-- Table for all non-thumbnail sized representations of an image. Gives worse
-- performance than storing the images externally, but is easier to deal with
CREATE TABLE images (
    id INTEGER PRIMARY KEY NOT NULL,

    media_type TEXT NOT NULL,
    data BLOB NOT NULL
);

CREATE TABLE images_meta (
    id INTEGER PRIMARY KEY NOT NULL,

    width INTEGER NOT NULL,
    height INTEGER NOT NULL,

    pixurs_id INTEGER NOT NULL,

    FOREIGN KEY (id) REFERENCES images(id),
    FOREIGN KEY (pixurs_id) REFERENCES pixurs(id)
);

CREATE TABLE pixurs_series (
    id INTEGER NOT NULL,
    'order' INTEGER NOT NULL,
    pixurs_id INTEGER NOT NULL,

    PRIMARY KEY (id, 'order'),
    FOREIGN KEY (pixurs_id) REFERENCES pixurs(id)
);

INSERT INTO pixurs_series (id, 'order', pixurs_id)
    SELECT id, 0, id FROM pixurs;
